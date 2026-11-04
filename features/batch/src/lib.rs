//! 按参数类型解析记录，并提供顺序批量调用的对齐与输出聚合。
//!
//! 本 crate 不依赖节点运行时、过程宏或 format。解析复用 codec，宏组合各项 API 完成调用。
mod alignment;
mod outputs;
mod records;

pub use alignment::batch_length;
pub use outputs::batch_outputs;
pub use records::{batch_records, decode_batch};
