//! 共享调用：一次调用让多个已创建通道执行同一操作。
//!
//! 共享调用的 Input 首个元素为标记值 `共享`，其后每个元素是一个通道的参数串；参数串内部
//! 以英文逗号分隔同通道的不同参数。参数串为空白、拆分个数不符或缺少必填参数的通道会被
//! 跳过，并在输出中以空字符串占位。

use super::lifecycle::Channel;
use crate::{
    DeviceNode, ExecutionMode, InvocationOutput, MiresidianError, MiresidianResult,
    OperationDescriptor, ParameterDescriptor, SHARED_INVOCATION_MARKER,
};

/// 判断一次调用的 Input 是否为共享调用形式。
pub(super) fn is_shared_invocation(input: &[String]) -> bool {
    input
        .first()
        .is_some_and(|marker| marker == SHARED_INVOCATION_MARKER)
}

/// 在全部已创建对象上按预设执行方式运行有效通道。
///
/// 参数串按对象的创建顺序一一对应；不足的对象或参数槽位不执行。
pub(super) fn invoke_shared<N: DeviceNode>(
    nodes: &[Channel<N>],
    operation: &str,
    channels: &[String],
    descriptor: &OperationDescriptor,
    execution: ExecutionMode,
) -> MiresidianResult<InvocationOutput> {
    let plans = channel_plans(channels, descriptor.channel_inputs);
    let plans = match execution {
        ExecutionMode::Broadcast => broadcast_plans(plans)?,
        ExecutionMode::Sequential | ExecutionMode::Parallel => plans,
    };
    let outputs = match execution {
        ExecutionMode::Sequential => plans
            .iter()
            .enumerate()
            .map(|(index, plan)| execute_channel(nodes.get(index), operation, plan, index))
            .collect::<Result<Vec<_>, _>>(),
        ExecutionMode::Parallel => run_parallel(nodes, operation, &plans),
        ExecutionMode::Broadcast => plans
            .iter()
            .enumerate()
            .map(|(index, plan)| execute_channel(nodes.get(index), operation, plan, index))
            .collect::<Result<Vec<_>, _>>(),
    }?;
    Ok(InvocationOutput::new(
        outputs
            .into_iter()
            .map(|output| output.join(","))
            .collect::<Vec<_>>(),
    ))
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

/// 将每个通道参数串拆分为参数值；不合法的通道返回 `None`。
///
/// 拆分后的段值原样传递，`|` 数组语法由 codec 层按参数类型正常解析。
fn channel_plans(channels: &[String], parameters: &[ParameterDescriptor]) -> Vec<ChannelPlan> {
    channels
        .iter()
        .map(|channel| split_channel(channel, parameters))
        .collect()
}

/// 拆分单个通道参数串；空白、空值、参数数目不符均跳过。
fn split_channel(channel: &str, parameters: &[ParameterDescriptor]) -> ChannelPlan {
    let trimmed = channel.trim();
    if trimmed.is_empty() {
        return None;
    }
    let values = trimmed
        .split(',')
        .map(|segment| segment.trim().to_owned())
        .collect::<Vec<_>>();
    if values.len() > parameters.len()
        || values.iter().any(String::is_empty)
        || parameters[values.len()..]
            .iter()
            .any(|parameter| parameter.default_value.is_none())
    {
        return None;
    }
    Some(values)
}

/// 在当前线程上执行一个通道。
fn execute_channel<N: DeviceNode>(
    channel: Option<&Channel<N>>,
    operation: &str,
    plan: &ChannelPlan,
    index: usize,
) -> MiresidianResult<Vec<String>> {
    let Some(arguments) = plan else {
        return Ok(Vec::new());
    };
    let Some(channel) = channel else {
        return Ok(Vec::new());
    };
    let mut node = channel
        .node
        .lock()
        .map_err(|_| MiresidianError::Failure("节点状态锁已损坏".to_owned()))?;
    node.invoke(operation, &channel.config, arguments)
        .map(|output| output.into_values())
        .map_err(|error| MiresidianError::Failure(format!("通道 {} 执行失败: {error}", index + 1)))
}

/// 为各通道分发独立线程执行。
fn run_parallel<N: DeviceNode>(
    nodes: &[Channel<N>],
    operation: &str,
    plans: &[ChannelPlan],
) -> MiresidianResult<Vec<Vec<String>>> {
    let operation = operation.to_owned();
    let results = std::thread::scope(|scope| {
        plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                let channel = nodes.get(index);
                let operation = operation.clone();
                scope.spawn(move || execute_channel(channel, &operation, plan, index))
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
