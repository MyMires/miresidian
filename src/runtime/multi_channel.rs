//! 整个多通道批次独占目标实例的请求执行权，各通道再通过同一状态锁访问对象。
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
        // 通道 Future 异步等待状态锁，不为每个等待项分配操作系统线程。
        // 等待全部已接收项结束后，按输入位置返回首个错误；其他项的副作用不回滚。
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
