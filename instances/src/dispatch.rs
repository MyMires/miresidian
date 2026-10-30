//! 结构化平台输入计划的通用调度，不接触设备或平台语法。
use crate::ExecutionMode;
use miresidian_codec::{InvocationOutput, MiresidianError, MiresidianResult};

/// 在调用方提供的执行函数上处理各平台输入，并保持输出位置。
///
/// 参数默认值仅用于判定缺失尾部输入是否可执行。调用方负责完整方法调用的资源互斥；
/// parallel 为有效输入创建线程，执行失败后仍会等待已开始的输入完成。
pub fn invoke_multi_channel<F>(
    slots: &[Option<Vec<String>>],
    parameters: &[Option<&str>],
    execution: ExecutionMode,
    call: F,
) -> MiresidianResult<Vec<Option<InvocationOutput>>>
where
    F: Fn(&[String]) -> MiresidianResult<InvocationOutput> + Sync,
{
    let plans = plan_multi_channel(slots, parameters, execution)?;
    match execution {
        ExecutionMode::Sequential | ExecutionMode::Broadcast => plans
            .iter()
            .enumerate()
            .map(|(index, plan)| execute_channel(&call, plan, index))
            .collect(),
        ExecutionMode::Parallel => run_parallel(&call, &plans),
    }
}

/// Validate and normalize a batch without executing user code. Shared by sync
/// and native async frontends; positions and broadcast rules stay identical.
pub fn plan_multi_channel(
    slots: &[Option<Vec<String>>],
    parameters: &[Option<&str>],
    execution: ExecutionMode,
) -> MiresidianResult<Vec<Option<Vec<String>>>> {
    let plans = channel_plans(slots, parameters);
    if execution == ExecutionMode::Broadcast {
        broadcast_plans(plans)
    } else {
        Ok(plans)
    }
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
            field: "多通道输入".to_owned(),
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
fn channel_plans(slots: &[Option<Vec<String>>], parameters: &[Option<&str>]) -> Vec<ChannelPlan> {
    slots
        .iter()
        .map(|slot| {
            slot.as_ref()
                .and_then(|values| validate_slot(values, parameters))
        })
        .collect()
}

/// 参数数目不符、空参数或缺少无默认值参数的槽位视为跳过。
fn validate_slot(values: &[String], parameters: &[Option<&str>]) -> ChannelPlan {
    if values.len() > parameters.len()
        || values.iter().any(|value| value.trim().is_empty())
        || parameters[values.len()..]
            .iter()
            .any(|parameter| parameter.is_none())
    {
        return None;
    }
    Some(values.to_vec())
}

/// 在当前线程上使用共享对象执行一个平台通道输入。
fn execute_channel<F>(
    call: &F,
    plan: &ChannelPlan,
    index: usize,
) -> MiresidianResult<Option<InvocationOutput>>
where
    F: Fn(&[String]) -> MiresidianResult<InvocationOutput> + Sync,
{
    let Some(arguments) = plan else {
        return Ok(None);
    };
    call(arguments)
        .map(Some)
        .map_err(|error| error.with_context(format!("通道 {} 执行失败", index + 1)))
}

/// 为各平台输入分发独立线程；同一节点的完整调用仍由通道锁互斥保护。
fn run_parallel<F>(
    call: &F,
    plans: &[ChannelPlan],
) -> MiresidianResult<Vec<Option<InvocationOutput>>>
where
    F: Fn(&[String]) -> MiresidianResult<InvocationOutput> + Sync,
{
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    let next = AtomicUsize::new(0);
    let results = Mutex::new((0..plans.len()).map(|_| None).collect::<Vec<_>>());
    let workers = std::thread::available_parallelism()
        .map_or(2, usize::from)
        .clamp(2, 32)
        .min(plans.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let next = &next;
            let results = &results;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(plan) = plans.get(index) else {
                        break;
                    };
                    if plan.is_none() {
                        continue;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute_channel(call, plan, index)
                    }));
                    results.lock().unwrap_or_else(|e| e.into_inner())[index] = Some(result);
                }
            });
        }
    });
    let results = results.into_inner().unwrap_or_else(|e| e.into_inner());
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
