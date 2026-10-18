//! Miresidian C ABI 的公共类型。
//!
//! 实际导出函数由每个节点 crate 上的 `#[miresidian::node_impl]` 生成；运行时 crate
//! 本身不绑定任何具体设备，也不导出某个节点 DLL。

use std::{ffi::c_char, ptr};

/// C ABI 调用成功。
pub const MIRESIDIAN_STATUS_OK: MiresidianStatus = 0;
/// C ABI 参数为空、格式错误或不是有效 UTF-8。
pub const MIRESIDIAN_STATUS_INVALID_ARGUMENT: MiresidianStatus = 1;
/// 设备操作、生命周期或通信失败。
pub const MIRESIDIAN_STATUS_OPERATION_FAILED: MiresidianStatus = 2;
/// Rust 内部发生了已捕获的 panic。
pub const MIRESIDIAN_STATUS_INTERNAL_ERROR: MiresidianStatus = 3;

/// C ABI 函数返回的状态码。
pub type MiresidianStatus = i32;

/// C ABI 中传递 UTF-8 文本数组的通用结构。
///
/// `miresidian_invoke` 同时用它承载业务 `output` 与错误 `error`。调用方必须将结构体
/// 零初始化，并在调用后以同一 DLL 的 `miresidian_free_text_array` 释放。
#[repr(C)]
#[derive(Debug)]
pub struct MiresidianTextArray {
    /// 指向以 NUL 结尾字符串指针的连续数组。
    pub values: *mut *mut c_char,
    /// `values` 数组中的元素数量。
    pub count: usize,
}

impl Default for MiresidianTextArray {
    fn default() -> Self {
        Self {
            values: ptr::null_mut(),
            count: 0,
        }
    }
}
