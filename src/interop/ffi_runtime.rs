//! JSON 文本 ABI 与 Rust 节点运行时之间的适配层。
//!
//! C ABI 只传递普通 UTF-8 C 字符串：`config_json`、`input_json` 与 `output_json` 的
//! 内容均为 JSON。调用方持有输出缓冲区，DLL 不向调用方分配任何内存。

use serde::Serialize;
use std::{
    ffi::{CStr, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, OnceLock},
};

use crate::{
    DeviceNode, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianStatus, NodeRuntime,
};

/// 调用方提供的输出缓冲区不足以容纳完整 JSON 文本。
pub const MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL: MiresidianStatus = 4;

/// C ABI 写入输出缓冲区的固定 JSON 响应。
///
/// 字段声明顺序即 JSON 文本顺序：先业务输出，后错误信息。使用结构体而非通用 JSON 映射，
/// 避免序列化器重排字段而造成平台侧文本预期漂移。
#[derive(Serialize)]
struct InvocationResponse<'a> {
    output: &'a [String],
    error: Option<&'a str>,
}

/// 调用唯一的 DLL 入口。
///
/// 成功时 `output_json` 为 `{"output":[..业务输出],"error":null}`；失败时为
/// `{"output":[],"error":"错误信息"}`。函数返回值是唯一的错误码来源。
/// `config_json` 与 `input_json` 为 `NULL` 时按空数组处理。
///
/// # Safety
/// `function` 必须指向有效 UTF-8 C 字符串。非空 JSON 参数必须同样有效且以 NUL 结尾。
/// `output_json` 必须指向至少 `output_capacity` 字节的可写缓冲区。
pub unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
    output_json: *mut c_char,
    output_capacity: u32,
) -> MiresidianStatus {
    let (status, message, values) =
        unsafe { invoke_node(runtime, function, config_json, input_json) };
    let error = (status != MIRESIDIAN_STATUS_OK).then_some(message);
    match write_response(output_json, output_capacity, &values, error.as_deref()) {
        Ok(()) => status,
        Err(status) => status,
    }
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

fn write_response(
    target: *mut c_char,
    capacity: u32,
    output: &[String],
    error: Option<&str>,
) -> Result<(), MiresidianStatus> {
    let response = InvocationResponse { output, error };
    let json = serde_json::to_vec(&response).map_err(|_| MIRESIDIAN_STATUS_INTERNAL_ERROR)?;
    if target.is_null() || capacity == 0 {
        return Err(MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    }
    if json.len().saturating_add(1) > capacity as usize {
        return Err(MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL);
    }
    // SAFETY: 调用方契约保证 `target` 指向至少 `capacity` 字节的可写缓冲区；长度检查已完成。
    unsafe {
        core::ptr::copy_nonoverlapping(json.as_ptr().cast::<c_char>(), target, json.len());
        *target.add(json.len()) = 0;
    }
    Ok(())
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
    // SAFETY: 调用方契约要求 `value` 指向以 NUL 结尾的有效 C 字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| format!("参数 {field} 必须是有效 UTF-8 文本"))
}
