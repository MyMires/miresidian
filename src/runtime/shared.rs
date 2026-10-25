//! 共享调用：在全部已创建通道上按结构化槽位执行同一操作。
//!
//! 平台适配器负责把其输入格式转换成槽位；本模块只维护通道对齐、参数有效性和调度语义。

use super::lifecycle::Channel;
use crate::{
    DeviceNode, ExecutionMode, InvocationOutput, MiresidianError, MiresidianResult,
    OperationDescriptor, ParameterDescriptor, SharedResponse,
};

/// 在全部已创建对象上按预设执行方式运行有效通道。
///
/// 输入槽位不足时补齐跳过槽位；超过已初始化通道数时拒绝请求。因此结果槽位数始终等于
/// 已初始化通道数。
pub(super) fn invoke_shared<N: DeviceNode>(
    nodes: &[Channel<N>],
    operation: &str,
    slots: &[Option<Vec<String>>],
    descriptor: &OperationDescriptor,
    execution: ExecutionMode,
) -> MiresidianResult<SharedResponse> {
    let plans = channel_plans(slots, descriptor.channel_inputs, nodes.len())?;
    let plans = match execution {
        ExecutionMode::Broadcast => broadcast_plans(plans)?,
        ExecutionMode::Sequential | ExecutionMode::Parallel => plans,
    };
    let outputs = match execution {
        ExecutionMode::Sequential | ExecutionMode::Broadcast => plans
            .iter()
            .enumerate()
            .map(|(index, plan)| execute_channel(&nodes[index], operation, plan, index))
            .collect::<Result<Vec<_>, _>>(),
        ExecutionMode::Parallel => run_parallel(nodes, operation, &plans),
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

/// 单个通道的执行计划；`None` 表示该通道被跳过。
type ChannelPlan = Option<Vec<String>>;

/// 校验平台适配器提供的槽位，并补齐遗漏通道的跳过槽位。
fn channel_plans(
    slots: &[Option<Vec<String>>],
    parameters: &[ParameterDescriptor],
    channel_count: usize,
) -> MiresidianResult<Vec<ChannelPlan>> {
    if slots.len() > channel_count {
        return Err(MiresidianError::InvalidArgument {
            field: "共享输入".to_owned(),
            message: "共享输入槽位数量超过已初始化通道数".to_owned(),
        });
    }
    let mut plans = slots
        .iter()
        .map(|slot| {
            slot.as_ref()
                .and_then(|values| validate_slot(values, parameters))
        })
        .collect::<Vec<_>>();
    plans.resize(channel_count, None);
    Ok(plans)
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

/// 在当前线程上执行一个通道。
fn execute_channel<N: DeviceNode>(
    channel: &Channel<N>,
    operation: &str,
    plan: &ChannelPlan,
    index: usize,
) -> MiresidianResult<Option<InvocationOutput>> {
    let Some(arguments) = plan else {
        return Ok(None);
    };
    let mut node = channel
        .node
        .lock()
        .map_err(|_| MiresidianError::Failure("节点状态锁已损坏".to_owned()))?;
    node.invoke(operation, &channel.config, arguments)
        .map(Some)
        .map_err(|error| MiresidianError::Failure(format!("通道 {} 执行失败: {error}", index + 1)))
}

/// 为各通道分发独立线程执行。
fn run_parallel<N: DeviceNode>(
    nodes: &[Channel<N>],
    operation: &str,
    plans: &[ChannelPlan],
) -> MiresidianResult<Vec<Option<InvocationOutput>>> {
    let operation = operation.to_owned();
    let results = std::thread::scope(|scope| {
        plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                let operation = operation.clone();
                scope.spawn(move || execute_channel(&nodes[index], &operation, plan, index))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut outputs = Vec::with_capacity(results.len());
    for (index, result) in results.into_iter().enumerate() {
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
