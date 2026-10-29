//! 共享调用：平台各通道的输入共用唯一 Config 定位的同一节点对象。
//!
//! 槽位仅维护输入输出对齐，不用于选择对象；对象定位在生命周期模块中完成。

use super::lifecycle::Channel;
use crate::{
    DeviceNode, ExecutionMode, InvocationOutput, MiresidianError, MiresidianResult,
    OperationDescriptor, ParameterDescriptor, SharedResponse,
};

/// 在同一个节点对象上按预设执行方式运行各平台通道的有效输入。
///
/// 结果与输入保持相同顺序和数量，输入数量与已创建对象数无关。
pub(super) fn invoke_shared<N: DeviceNode>(
    channel: &Channel<N>,
    operation: &str,
    slots: &[Option<Vec<String>>],
    descriptor: &OperationDescriptor,
    execution: ExecutionMode,
) -> MiresidianResult<SharedResponse> {
    let plans = channel_plans(slots, descriptor.channel_inputs);
    let plans = match execution {
        ExecutionMode::Broadcast => broadcast_plans(plans)?,
        ExecutionMode::Sequential | ExecutionMode::Parallel => plans,
    };
    let outputs = match execution {
        ExecutionMode::Sequential | ExecutionMode::Broadcast => plans
            .iter()
            .enumerate()
            .map(|(index, plan)| execute_channel(channel, operation, plan, index))
            .collect::<Result<Vec<_>, _>>(),
        ExecutionMode::Parallel => run_parallel(channel, operation, &plans),
    }?;
    Ok(SharedResponse::new(outputs))
}

/// 将全部有效通道参数计划收敛为第一份计划，并拒绝不一致的广播输入。
fn broadcast_plans(plans: Vec<ChannelPlan>) -> MiresidianResult<Vec<ChannelPlan>> {
    let Some(arguments) = plans.iter().flatten().next().cloned() else {
        return Ok(plans);
    };
    if plans
        .iter()
        .flatten()
        .any(|candidate| candidate != &arguments)
    {
        return Err(MiresidianError::InvalidArgument {
            field: "共享输入".to_owned(),
            message: "广播执行要求各通道 Input 参数完全一致".to_owned(),
        });
    }
    Ok(plans
        .into_iter()
        .map(|plan| plan.map(|_| arguments.clone()))
        .collect())
}

/// 单个平台通道输入的执行计划；`None` 表示该输入被跳过。
type ChannelPlan = Option<Vec<String>>;

/// 校验平台适配器提供的槽位，保留其数量与顺序。
fn channel_plans(
    slots: &[Option<Vec<String>>],
    parameters: &[ParameterDescriptor],
) -> Vec<ChannelPlan> {
    slots
        .iter()
        .map(|slot| {
            slot.as_ref()
                .and_then(|values| validate_slot(values, parameters))
        })
        .collect()
}

/// 参数数目不符、空参数或缺少无默认值参数的槽位视为跳过。
fn validate_slot(values: &[String], parameters: &[ParameterDescriptor]) -> ChannelPlan {
    if values.len() > parameters.len()
        || values.iter().any(|value| value.trim().is_empty())
        || parameters[values.len()..]
            .iter()
            .any(|parameter| parameter.default_value.is_none())
    {
        return None;
    }
    Some(values.to_vec())
}

/// 在当前线程上使用共享对象执行一个平台通道输入。
fn execute_channel<N: DeviceNode>(
    channel: &Channel<N>,
    operation: &str,
    plan: &ChannelPlan,
    index: usize,
) -> MiresidianResult<Option<InvocationOutput>> {
    let Some(arguments) = plan else {
        return Ok(None);
    };
    channel
        .invoke(operation, arguments)
        .map(Some)
        .map_err(|error| error.with_context(format!("通道 {} 执行失败", index + 1)))
}

/// 为各平台输入分发独立线程；同一节点的完整调用仍由通道锁互斥保护。
fn run_parallel<N: DeviceNode>(
    channel: &Channel<N>,
    operation: &str,
    plans: &[ChannelPlan],
) -> MiresidianResult<Vec<Option<InvocationOutput>>> {
    let results = std::thread::scope(|scope| {
        plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                if plan.is_none() {
                    return None;
                }
                Some(scope.spawn(move || execute_channel(channel, operation, plan, index)))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.map(|handle| handle.join()))
            .collect::<Vec<_>>()
    });
    let mut outputs = Vec::with_capacity(results.len());
    for (index, result) in results.into_iter().enumerate() {
        let Some(result) = result else {
            outputs.push(None);
            continue;
        };
        match result {
            Ok(Ok(output)) => outputs.push(output),
            Ok(Err(error)) => return Err(error),
            Err(_) => {
                return Err(MiresidianError::Failure(format!(
                    "通道 {} 执行线程异常退出",
                    index + 1
                )));
            }
        }
    }
    Ok(outputs)
}
