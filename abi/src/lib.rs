//! Miresidian C ABI 的公共类型。
//!
//! 实际导出函数由每个节点 crate 上的 `#[miresidian::node_impl]` 生成；运行时 crate
//! 本身不绑定任何具体设备，也不导出某个节点 DLL。

use std::{ffi::c_char, ptr};

/// 当前固定 C ABI 的版本号。新增 C 导出入口或改变既有 C 导出契约时递增。
pub const MIRESIDIAN_ABI_VERSION: u32 = 7;

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

/// 旧版 Rust 源码兼容保留的 UTF-8 文本数组结构。
///
/// C ABI v7 的 `miresidian_invoke` 不使用此结构，也不导出 `miresidian_free_text_array`；
/// v7 调用方应读取其返回的 `result_json`，且不得释放该指针。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_c_header_matches_the_abi_contract() {
        let header = include_str!("../api/miresidian.h");
        assert!(header.contains(&format!(
            "#define MIRESIDIAN_ABI_VERSION {MIRESIDIAN_ABI_VERSION}"
        )));
        assert!(header.contains("MIRESIDIAN_PLATFORM_TB_LV_TEST_PLATFORM = 1"));
        assert!(header.contains("MIRESIDIAN_PLATFORM_CUSTOM_MIN = 65536"));
        assert!(header.contains("MIRESIDIAN_STATUS_OK = 0"));
        assert!(header.contains("MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1"));
        assert!(header.contains("MIRESIDIAN_STATUS_OPERATION_FAILED = 2"));
        assert!(header.contains("MIRESIDIAN_STATUS_INTERNAL_ERROR = 3"));
        assert!(header.contains("miresidian_abi_version(void)"));
        assert!(header.contains("miresidian_invoke("));
        assert!(header.contains("miresidian_invoke_with("));
        assert_eq!(header, include_str!("../../api/miresidian.h"));
    }

    #[test]
    fn package_metadata_matches_the_abi_version() {
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains(&format!("abi_version = {MIRESIDIAN_ABI_VERSION}")));
    }
}
