//! 初始化输出编码期间保留节点所有权，编码成功后才把对象交还运行时登记。

use super::lifecycle::{catch_node, cleanup_node};
use crate::{INIT_DEVICE_OPERATION, InvocationOutput, MiresidianResult, Node, NodeOutput};

/// 宏生成的初始化收尾：编码业务输出，失败时关闭并析构已构造的节点。
///
/// 编码成功返回原节点及输出；编码错误或 panic 时，以空 Input 清理节点并返回原错误。
/// 自动 Lease 的所有者仍保护资源及配置租约；节点清理在释放这些保护之前完成。
/// 编码、close 与 Drop 的 panic 分别隔离，避免清理期间发生二次展开。
#[doc(hidden)]
pub fn finish_initialization<N: Node, T: NodeOutput>(
    node: N,
    value: T,
    config: &[String],
) -> MiresidianResult<(N, InvocationOutput)> {
    match catch_node(INIT_DEVICE_OPERATION, || crate::codec::encode_value(value)) {
        Ok(output) => Ok((node, output)),
        Err(error) => {
            let _ = cleanup_node(node, config, &[]);
            Err(error)
        }
    }
}
