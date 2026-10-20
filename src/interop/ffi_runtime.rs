//! JSON 文本 ABI 与 Rust 节点运行时之间的适配层。
//!
//! C ABI 仅接收 UTF-8 C 字符串，并直接返回 DLL 持有的 JSON 响应文本。调用方不传递输出
//! 缓冲区，也不参与响应内存的分配或释放。

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, OnceLock},
};

use serde::Serialize;

use crate::{
    DeviceNode, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianStatus, NodeRuntime,
};

const INTERNAL_ERROR_RESPONSE: &str =
    r#"{"status":3,"message":"Miresidian 内部发生未预期错误","output":[]}"#;

thread_local! {
    /// 当前线程最近一次 ABI 调用的响应文本。
    ///
    /// 指针仅在同一线程下一次调用 miresidian_invoke 前有效；外部平台应在调用返回后立即
    /// 复制文本，而不需要也不得释放该指针。
    static LAST_RESPONSE: RefCell<CString> = RefCell::new(
        CString::new(INTERNAL_ERROR_RESPONSE).expect("固定 JSON 响应不含 NUL"),
    );
}

/// DLL 返回的固定 JSON 响应。
///
/// 字段顺序是平台契约的一部分：状态码、消息、业务输出。
#[derive(Serialize)]
struct InvocationResponse {
    status: MiresidianStatus,
    message: String,
    output: Vec<String>,
}

/// 调用唯一的 DLL 入口并返回 JSON C 字符串指针。
///
/// 返回指针由 DLL 持有，在同一线程下一次调用此函数前有效。调用方应立即复制文本，且不得
/// 释放该指针。JSON 始终包含 status、message 与 output。
/// config_json 与 input_json 为 NULL 时按空数组处理。
///
/// # Safety
/// function 必须指向有效 UTF-8 C 字符串。非空 JSON 参数必须同样有效且以 NUL 结尾。
pub unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> *const c_char {
    let (status, message, output) =
        unsafe { invoke_node(runtime, function, config_json, input_json) };
    store_response(InvocationResponse {
        status,
        message,
        output,
    })
}

fn store_response(response: InvocationResponse) -> *const c_char {
    let json = serde_json::to_string(&response)
        .ok()
        .and_then(|text| CString::new(text).ok());

    LAST_RESPONSE.with(|last_response| {
        let mut last_response = last_response.borrow_mut();
        *last_response = json.unwrap_or_else(|| {
            CString::new(INTERNAL_ERROR_RESPONSE).expect("固定 JSON 响应不含 NUL")
        });
        last_response.as_ptr()
    })
}

unsafe fn invoke_node<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> (MiresidianStatus, String, Vec<String>) {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        dispatch(runtime, function, config_json, input_json)
    })) {
        Ok(Ok(values)) => (MIRESIDIAN_STATUS_OK, String::new(), values),
        Ok(Err((status, message))) => (status, message, Vec::new()),
        Err(_) => (
            MIRESIDIAN_STATUS_INTERNAL_ERROR,
            "Miresidian 内部发生未预期错误".to_owned(),
            Vec::new(),
        ),
    }
}

unsafe fn dispatch<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> Result<Vec<String>, (MiresidianStatus, String)> {
    let operation = unsafe { copy_required_text(function, "function") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let config = unsafe { parse_json_array(config_json, "config_json") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let input = unsafe { parse_json_array(input_json, "input_json") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    runtime
        .get_or_init(|| Mutex::new(NodeRuntime::new()))
        .lock()
        .map_err(|_| {
            (
                MIRESIDIAN_STATUS_INTERNAL_ERROR,
                "节点状态锁已损坏".to_owned(),
            )
        })?
        .invoke(&Invocation::new(operation, config, input))
        .map(|value| value.into_values())
        .map_err(|error| (MIRESIDIAN_STATUS_OPERATION_FAILED, error.to_string()))
}

unsafe fn parse_json_array(value: *const c_char, field: &str) -> Result<Vec<String>, String> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let text = unsafe { copy_required_text(value, field) }?;
    serde_json::from_str(&text).map_err(|_| format!("参数 {field} 必须是 JSON 字符串数组"))
}

unsafe fn copy_required_text(value: *const c_char, field: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("参数 {field} 不能为空"));
    }
    // SAFETY: 调用方契约要求 value 指向以 NUL 结尾的有效 C 字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| format!("参数 {field} 必须是有效 UTF-8 文本"))
}
