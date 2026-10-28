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
    DeviceNode, ErrorCategory, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED,
    MiresidianError, MiresidianStatus, NodeRuntime, Platform,
};

const INTERNAL_ERROR_RESPONSE: &str =
    r#"{"status":3,"code":"INTERNAL_ERROR","message":"Miresidian 内部发生未预期错误","output":[]}"#;

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
/// 字段顺序是平台契约的一部分：状态分类、稳定代码、诊断消息、业务输出。
#[derive(Serialize)]
struct InvocationResponse {
    status: MiresidianStatus,
    code: &'static str,
    message: String,
    output: Vec<String>,
}

/// 调用唯一的 DLL 入口并返回 JSON C 字符串指针。
///
/// 返回指针由 DLL 持有，在同一线程下一次调用此函数前有效。调用方应立即复制文本，且不得
/// 释放该指针。JSON 始终包含 status、code、message 与 output。
/// config_json 与 input_json 为 NULL 时按空数组处理。
///
/// # Safety
/// 所有非空指针必须指向有效、以 NUL 结尾的 C 字符串缓冲区。NULL function、
/// 非 UTF-8 文本和非法 JSON 均由入口返回参数错误。
pub unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> *const c_char {
    // SAFETY: 指针前置条件由此函数的调用契约保证，内部继续校验空指针与 UTF-8。
    store_response(caught_response(|| unsafe {
        dispatch(runtime, function, config_json, input_json)
    }))
}

/// 使用显式平台适配器调用 DLL，并返回 DLL 持有的 JSON C 字符串指针。
///
/// 此路径对应 `NodeRuntime::invoke_with`：`platform` 必须是节点 DLL 已注册的平台枚举数值；
/// `InitDevice` 的 Config 不发布也不接受 `平台适配` 选择器。
///
/// # Safety
/// 所有非空指针必须指向有效、以 NUL 结尾的 C 字符串缓冲区。NULL function、
/// 非 UTF-8 文本和非法 JSON 均由入口返回参数错误。
/// platform 的合法性和注册状态由入口校验。
pub unsafe fn invoke_with<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    platform: u32,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> *const c_char {
    // SAFETY: 字符串指针满足此函数的调用契约；platform 在 dispatch_with 中校验。
    store_response(caught_response(|| unsafe {
        dispatch_with(runtime, platform, function, config_json, input_json)
    }))
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

fn caught_response(
    call: impl FnOnce() -> crate::MiresidianResult<Vec<String>>,
) -> InvocationResponse {
    let result = catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|_| {
        Err(MiresidianError::Internal(
            "Miresidian 内部发生未预期错误".to_owned(),
        ))
    });
    match result {
        Ok(output) => InvocationResponse {
            status: MIRESIDIAN_STATUS_OK,
            code: "OK",
            message: String::new(),
            output,
        },
        Err(error) => InvocationResponse {
            status: status_for_error(&error),
            code: error.code(),
            message: error.to_string(),
            output: Vec::new(),
        },
    }
}

unsafe fn dispatch<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> crate::MiresidianResult<Vec<String>> {
    // SAFETY: 调用方提供有效 C 字符串；这些函数检查空指针、UTF-8 和 JSON。
    let operation = unsafe { copy_required_text(function, "function") }?;
    // SAFETY: 同上，NULL JSON 指针按空数组处理。
    let config = unsafe { parse_json_array(config_json, "config_json") }?;
    // SAFETY: 同上。
    let input = unsafe { parse_json_array(input_json, "input_json") }?;
    invoke_runtime(runtime, N::platform_registry, |runtime| {
        runtime.invoke(&Invocation::new(operation, config, input))
    })
}

unsafe fn dispatch_with<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    platform: u32,
    function: *const c_char,
    config_json: *const c_char,
    input_json: *const c_char,
) -> crate::MiresidianResult<Vec<String>> {
    let platform =
        Platform::from_abi(platform).ok_or(MiresidianError::InvalidPlatformId(platform))?;
    // SAFETY: 调用方提供有效 C 字符串；这些函数检查空指针、UTF-8 和 JSON。
    let operation = unsafe { copy_required_text(function, "function") }?;
    // SAFETY: 同上，NULL JSON 指针按空数组处理。
    let config = unsafe { parse_json_array(config_json, "config_json") }?;
    // SAFETY: 同上。
    let input = unsafe { parse_json_array(input_json, "input_json") }?;
    let platforms = N::platform_registry();
    let adapter = platforms
        .adapter(platform)
        .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(platform.to_string()))?;
    invoke_runtime(
        runtime,
        || platforms,
        |runtime| runtime.invoke_with(adapter.as_ref(), Invocation::new(operation, config, input)),
    )
}

// 在运行时锁仍由当前栈持有时捕获意外异常，避免全局 Mutex 被污染。
fn invoke_runtime<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    platforms: impl FnOnce() -> crate::PlatformRegistry,
    call: impl FnOnce(&mut NodeRuntime<N>) -> crate::MiresidianResult<crate::InvocationOutput>,
) -> crate::MiresidianResult<Vec<String>> {
    let mutex = runtime.get_or_init(|| Mutex::new(NodeRuntime::with_platforms(platforms())));
    let mut runtime = match mutex.lock() {
        Ok(runtime) => runtime,
        Err(poisoned) => {
            // 外部 Rust 调用者可能已污染此锁；旧对象仅清理，不恢复其业务状态。
            let mut runtime = poisoned.into_inner();
            let _ = runtime.shutdown();
            mutex.clear_poison();
            runtime
        }
    };
    match catch_unwind(AssertUnwindSafe(|| call(&mut runtime))) {
        Ok(result) => result.map(|value| value.into_values()),
        Err(_) => {
            let _ = runtime.shutdown();
            Err(MiresidianError::Internal(
                "运行时发生未预期错误，已清理全部通道；可以重新初始化".to_owned(),
            ))
        }
    }
}

fn status_for_error(error: &MiresidianError) -> MiresidianStatus {
    match error.category() {
        ErrorCategory::InvalidArgument => MIRESIDIAN_STATUS_INVALID_ARGUMENT,
        ErrorCategory::OperationFailed => MIRESIDIAN_STATUS_OPERATION_FAILED,
        ErrorCategory::InternalError => MIRESIDIAN_STATUS_INTERNAL_ERROR,
    }
}

unsafe fn parse_json_array(
    value: *const c_char,
    field: &str,
) -> crate::MiresidianResult<Vec<String>> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    // SAFETY: 非空指针满足调用方 C 字符串契约。
    let text = unsafe { copy_required_text(value, field) }?;
    serde_json::from_str(&text).map_err(|_| MiresidianError::InvalidArgument {
        field: field.to_owned(),
        message: "必须是 JSON 字符串数组".to_owned(),
    })
}

unsafe fn copy_required_text(value: *const c_char, field: &str) -> crate::MiresidianResult<String> {
    if value.is_null() {
        return Err(MiresidianError::InvalidArgument {
            field: field.to_owned(),
            message: "不能为空".to_owned(),
        });
    }
    // SAFETY: 调用方契约要求 value 指向以 NUL 结尾的有效 C 字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| MiresidianError::InvalidArgument {
            field: field.to_owned(),
            message: "必须是有效 UTF-8 文本".to_owned(),
        })
}
