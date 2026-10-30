//! One batch owns instance request rights; each item uses its state lock.
use super::lifecycle::Instance;
use crate::{
    CallContext, DeviceNode, ExecutionMode, MiresidianResult, MultiChannelResponse,
    OperationDescriptor,
};
pub(super) async fn invoke_multi_channel<N: DeviceNode>(
    instance: &Instance<N>,
    operation: &str,
    slots: &[Option<Vec<String>>],
    descriptor: &OperationDescriptor,
    execution: ExecutionMode,
    context: &CallContext,
) -> MiresidianResult<MultiChannelResponse> {
    let parameters = descriptor
        .channel_inputs
        .iter()
        .map(|p| p.default_value)
        .collect::<Vec<_>>();
    let plans = miresidian_instances::plan_multi_channel(slots, &parameters, execution)?;
    let execute = |index: usize, plan: Option<Vec<String>>| async move {
        match plan {
            None => Ok(None),
            Some(input) => instance
                .invoke(operation, &input, context)
                .await
                .map(Some)
                .map_err(|e| e.with_context(format!("通道 {} 执行失败", index + 1))),
        }
    };
    let slots = if execution == ExecutionMode::Broadcast {
        let output = execute(0, plans.first().cloned().flatten()).await?;
        plans
            .into_iter()
            .map(|plan| if plan.is_some() { output.clone() } else { None })
            .collect()
    } else if execution == ExecutionMode::Parallel {
        // Futures wait on the instance mutex without allocating OS threads;
        // every admitted item settles before returning the first indexed error.
        futures_util::future::join_all(plans.into_iter().enumerate().map(|(i, p)| execute(i, p)))
            .await
            .into_iter()
            .collect::<MiresidianResult<Vec<_>>>()?
    } else {
        let mut output = Vec::new();
        for (index, plan) in plans.into_iter().enumerate() {
            output.push(execute(index, plan).await?);
        }
        output
    };
    Ok(MultiChannelResponse::new(slots))
}
