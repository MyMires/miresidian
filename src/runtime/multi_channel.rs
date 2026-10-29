//! 将节点对象接入独立实例 crate 的多通道调度。
use super::lifecycle::Instance;
use crate::{
    DeviceNode, ExecutionMode, MiresidianResult, MultiChannelResponse, OperationDescriptor,
};

pub(super) fn invoke_multi_channel<N: DeviceNode>(
    instance: &Instance<N>,
    operation: &str,
    slots: &[Option<Vec<String>>],
    descriptor: &OperationDescriptor,
    execution: ExecutionMode,
) -> MiresidianResult<MultiChannelResponse> {
    let parameters = descriptor
        .channel_inputs
        .iter()
        .map(|parameter| parameter.default_value)
        .collect::<Vec<_>>();
    miresidian_instances::invoke_multi_channel(slots, &parameters, execution, |input| {
        instance.invoke(operation, input)
    })
    .map(MultiChannelResponse::new)
}
