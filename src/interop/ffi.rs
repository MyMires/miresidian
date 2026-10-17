//! Miresidian C ABI 的公共类型。
//!
//! 实际导出函数由每个驱动 crate 上的 `#[miresidian::node_impl]` 生成；运行时 crate
//! 本身不再绑定任何具体设备，也不导出某个设备 DLL。

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

/// `miresidian_invoke_v1` 成功返回的 UTF-8 输出字符串数组。
///
/// 调用方必须将此结构体零初始化；成功调用后必须使用同一 DLL 导出的
/// `miresidian_free_output_v1` 释放，不得自行释放数组或其中任何字符串。
#[repr(C)]
#[derive(Debug)]
pub struct MiresidianOutputV1 {
    /// 指向以 NUL 结尾字符串指针的连续数组。
    pub values: *mut *mut c_char,
    /// `values` 数组中的元素数量。
    pub count: usize,
}

impl Default for MiresidianOutputV1 {
    fn default() -> Self {
        Self {
            values: ptr::null_mut(),
            count: 0,
        }
    }
}
