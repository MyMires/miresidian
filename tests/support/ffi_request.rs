//! 业务回归测试的 JSON 构造器；生产 DLL 没有分参数兼容入口。
use std::ffi::{CStr, CString, c_char};

/// 将现有业务测试的数据组装为真实单请求调用。
///
/// # Safety
/// 非 NULL 输入指针必须是调用期间可读、NUL 结尾的 C 字符串。
pub unsafe fn request(
    call: impl FnOnce(*const c_char) -> *const c_char,
    platform: Option<u32>,
    function: *const c_char,
    config: *const c_char,
    input: *const c_char,
) -> *const c_char {
    let text = |pointer: *const c_char| {
        if pointer.is_null() {
            None
        } else {
            // SAFETY: 指针条件由本 helper 的调用方保证。
            unsafe { CStr::from_ptr(pointer) }.to_str().ok()
        }
    };
    let array = |pointer: *const c_char| {
        if pointer.is_null() {
            serde_json::json!([])
        } else {
            text(pointer)
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or(serde_json::Value::Null)
        }
    };
    let mut value = serde_json::json!({
        "function": text(function), "config": array(config), "input": array(input),
    });
    if let Some(platform) = platform {
        value["platform"] = platform.into();
    }
    let value = CString::new(value.to_string()).unwrap();
    call(value.as_ptr())
}
