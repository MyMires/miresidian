//! JSON 文本 ABI 与 Rust 节点运行时之间的适配层。
//!
//! C ABI 仅接收 UTF-8 C 字符串，并直接返回 DLL 持有的 JSON 响应文本。调用方不传递输出
//! 缓冲区，也不参与响应内存的分配或释放。

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
};

use serde::Serialize;

use crate::{
    ErrorCategory, Invocation, MIRESIDIAN_PLATFORM_DEFAULT, MIRESIDIAN_STATUS_INTERNAL_ERROR,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED,
    MiresidianError, MiresidianStatus, Node, NodeRuntime, Platform,
};

const INTERNAL_ERROR_RESPONSE: &str =
    r#"{"status":3,"code":"INTERNAL_ERROR","message":"Miresidian 内部发生未预期错误","output":[]}"#;

thread_local! {
    /// 当前线程最近一次 ABI 调用的响应文本。
    ///
    /// 指针仅在同一线程下一次调用任一 ABI 调用入口前有效；外部平台应在调用返回后立即
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

/// 唯一 DLL 调用入口：完整 JSON 请求，平台由本次请求确定。
///
/// function 必填；config/input 缺省为空数组。platform 缺省或 0 使用 DLL 默认平台，
/// 非零必须已注册。Config 不接受平台选择器。响应由 DLL 持有，在同线程下次调用前复制。
///
/// # Safety
/// 非 NULL request_json 必须指向调用期间有效、NUL 结尾的 UTF-8 C 字符串。
/// NULL、非法 UTF-8/JSON 和字段错误通过响应返回；无效地址不可校验。
pub unsafe fn invoke<N: Node>(
    runtime: &impl crate::runtime::RuntimeSource<N>,
    request_json: *const c_char,
) -> *const c_char {
    store_response(caught_response(|| {
        // SAFETY: 指针有效性由调用方保证，内部检查 NULL 与 UTF-8。
        let text = unsafe { copy_required_text(request_json, "request_json") }?;
        let request = super::request::parse(&text)?;
        dispatch_platform(
            runtime,
            request.platform.unwrap_or(MIRESIDIAN_PLATFORM_DEFAULT),
            Invocation::new(request.function, request.config, request.input),
        )
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

fn dispatch_platform<N: Node>(
    runtime: &impl crate::runtime::RuntimeSource<N>,
    platform: u32,
    invocation: Invocation,
) -> crate::MiresidianResult<Vec<String>> {
    let platforms = N::platform_registry();
    let platform = if platform == MIRESIDIAN_PLATFORM_DEFAULT {
        platforms.default_platform()
    } else {
        Platform::from_abi(platform).ok_or(MiresidianError::InvalidPlatformId(platform))?
    };
    let adapter = platforms
        .adapter(platform)
        .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(platform.to_string()))?;
    invoke_runtime(
        runtime,
        || platforms,
        |runtime| runtime.invoke_with(adapter.as_ref(), invocation),
    )
}

// 兼容旧 Mutex 容器时只在克隆运行时身份期间持锁；业务调用使用实例级门禁。
fn invoke_runtime<N: Node>(
    source: &impl crate::runtime::RuntimeSource<N>,
    platforms: impl FnOnce() -> crate::PlatformRegistry,
    call: impl FnOnce(&NodeRuntime<N>) -> crate::MiresidianResult<crate::InvocationOutput>,
) -> crate::MiresidianResult<Vec<String>> {
    let runtime = source.runtime(platforms)?;
    match catch_unwind(AssertUnwindSafe(|| call(&runtime))) {
        Ok(result) => result.map(|v| v.into_values()),
        Err(_) => {
            let _ = runtime.shutdown();
            Err(MiresidianError::Internal(
                "运行时异常，已清理全部实例".into(),
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
