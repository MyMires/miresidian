//! Constructed nodes remain owned while initialization output is encoded.

use super::lifecycle::{catch_node, cleanup_node};
use crate::{DeviceNode, DeviceOutput, INIT_DEVICE_OPERATION, InvocationOutput, MiresidianResult};

/// Generated initialization bridge: encode business output and clean up on failure.
///
/// The runtime still owns the lease while this helper closes and drops a node. Encoding
/// panics are caught before node cleanup, so a failing close/Drop is isolated separately.
#[doc(hidden)]
pub fn finish_initialization<N: DeviceNode, T: DeviceOutput>(
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
