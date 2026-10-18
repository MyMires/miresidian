//! 平台字符串数组与 Rust 节点运行时之间的 DLL 适配层。
//!
//! 公开 ABI 只传递以 NULL 结尾的 UTF-8 字符串数组，不暴露 Rust 容器或长度参数。
//! 输出指针由 DLL 持有，调用方应在下一次同类输出写入前完成读取与复制。
#![allow(non_snake_case, reason = "公开 ABI 参数名必须与平台通道名称一致")]

use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, OnceLock},
};

use crate::{
    DeviceNode, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianStatus, NodeRuntime,
};

const MAX_ARRAY_ITEMS: usize = 65_536;
static OUTPUT_SLOTS: OnceLock<Mutex<OutputSlots>> = OnceLock::new();

/// 三种入口各自保留最近一次返回值，避免导出额外的释放函数。
#[derive(Default)]
struct OutputSlots {
    combined: Option<OutputBlock>,
    output: Option<OutputBlock>,
    error: Option<OutputBlock>,
}

/// 同时持有字符串与其指针数组，确保交给平台的指针持续有效。
struct OutputBlock {
    _strings: Vec<CString>,
    pointers: Vec<*mut c_char>,
}

// SAFETY: `OutputBlock` 创建后不再修改；所有替换与读取指针发布均受互斥锁保护。
unsafe impl Send for OutputBlock {}

#[derive(Clone, Copy)]
enum OutputChannel {
    Combined,
    Output,
    Error,
}

/// 使用合并结果入口。
///
/// `Config`、`Input` 与 `Output` 均为 NULL 结尾的 UTF-8 字符串数组。成功或失败时，
/// `Output` 都依次返回 `[错误码, 错误信息, ..业务输出]`。
///
/// # Safety
/// `Function` 必须指向有效 UTF-8 C 字符串；数组参数必须为 NULL 或以 NULL 结尾的有效
/// C 字符串指针数组；`Output` 必须是可写的 `char***` 地址。
pub unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    Function: *const c_char,
    Config: *const *const c_char,
    Input: *const *const c_char,
    Output: *mut *mut *mut c_char,
) -> MiresidianStatus {
    unsafe {
        invoke_with_writer(
            runtime,
            Function,
            Config,
            Input,
            |status, message, values| {
                let mut all = vec![status.to_string(), message];
                all.extend(values);
                write_array(OutputChannel::Combined, Output, all)
            },
        )
    }
}

/// 使用业务输出与错误输出分离的入口。
///
/// 成功时 `Output` 为业务输出、`Error` 为空数组；失败时反之，`Error` 为
/// `[错误码, 错误信息]`。
///
/// # Safety
/// 调用方必须满足 [`invoke`] 的字符串与数组约束，并为 `Output`、`Error` 提供可写地址。
pub unsafe fn invoke_with_error<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    Function: *const c_char,
    Config: *const *const c_char,
    Input: *const *const c_char,
    Output: *mut *mut *mut c_char,
    Error: *mut *mut *mut c_char,
) -> MiresidianStatus {
    if Output.is_null() || Error.is_null() {
        return MIRESIDIAN_STATUS_INVALID_ARGUMENT;
    }
    unsafe {
        invoke_with_writer(
            runtime,
            Function,
            Config,
            Input,
            |status, message, values| {
                if status == MIRESIDIAN_STATUS_OK {
                    write_array(OutputChannel::Output, Output, values)?;
                    write_array(OutputChannel::Error, Error, Vec::new())
                } else {
                    write_array(OutputChannel::Output, Output, Vec::new())?;
                    write_array(
                        OutputChannel::Error,
                        Error,
                        vec![status.to_string(), message],
                    )
                }
            },
        )
    }
}

/// 使用只返回业务输出的入口。
///
/// 成功时 `Output` 为业务输出；失败时 `Output` 为空数组，具体原因只由返回状态码表达。
///
/// # Safety
/// 调用方必须满足 [`invoke`] 的字符串与数组约束。
pub unsafe fn invoke_without_error<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    Function: *const c_char,
    Config: *const *const c_char,
    Input: *const *const c_char,
    Output: *mut *mut *mut c_char,
) -> MiresidianStatus {
    unsafe {
        invoke_with_writer(runtime, Function, Config, Input, |status, _, values| {
            write_array(
                OutputChannel::Output,
                Output,
                if status == MIRESIDIAN_STATUS_OK {
                    values
                } else {
                    Vec::new()
                },
            )
        })
    }
}

unsafe fn invoke_with_writer<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    Function: *const c_char,
    Config: *const *const c_char,
    Input: *const *const c_char,
    write: impl FnOnce(MiresidianStatus, String, Vec<String>) -> Result<(), String>,
) -> MiresidianStatus {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        dispatch(runtime, Function, Config, Input)
    })) {
        Ok(Ok(values)) => finish(write, MIRESIDIAN_STATUS_OK, String::new(), values),
        Ok(Err((status, message))) => finish(write, status, message, Vec::new()),
        Err(_) => finish(
            write,
            MIRESIDIAN_STATUS_INTERNAL_ERROR,
            "Miresidian 内部发生未预期错误".to_owned(),
            Vec::new(),
        ),
    }
}

fn finish(
    write: impl FnOnce(MiresidianStatus, String, Vec<String>) -> Result<(), String>,
    status: MiresidianStatus,
    message: String,
    values: Vec<String>,
) -> MiresidianStatus {
    if write(status, message, values).is_ok() {
        status
    } else {
        MIRESIDIAN_STATUS_INTERNAL_ERROR
    }
}

unsafe fn dispatch<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    Function: *const c_char,
    Config: *const *const c_char,
    Input: *const *const c_char,
) -> Result<Vec<String>, (MiresidianStatus, String)> {
    let function = unsafe { copy_string(Function, "Function") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let config = unsafe { copy_array(Config, "Config") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let input = unsafe { copy_array(Input, "Input") }
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
        .invoke(&Invocation::new(function, config, input))
        .map(|value| value.into_values())
        .map_err(|error| (MIRESIDIAN_STATUS_OPERATION_FAILED, error.to_string()))
}

fn write_array(
    channel: OutputChannel,
    target: *mut *mut *mut c_char,
    values: Vec<String>,
) -> Result<(), String> {
    if target.is_null() {
        return Err("Output 或 Error 不能为空".to_owned());
    }
    let strings = values
        .into_iter()
        .map(|value| CString::new(value).map_err(|_| "输出文本不能包含 NUL 字符".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut pointers = strings
        .iter()
        .map(|value| value.as_ptr() as *mut c_char)
        .collect::<Vec<_>>();
    pointers.push(core::ptr::null_mut());

    let mut slots = OUTPUT_SLOTS
        .get_or_init(|| Mutex::new(OutputSlots::default()))
        .lock()
        .map_err(|_| "输出缓存锁已损坏".to_owned())?;
    let slot = match channel {
        OutputChannel::Combined => &mut slots.combined,
        OutputChannel::Output => &mut slots.output,
        OutputChannel::Error => &mut slots.error,
    };
    *slot = Some(OutputBlock {
        _strings: strings,
        pointers,
    });
    let pointer = slot
        .as_mut()
        .expect("输出缓存刚刚写入，必定存在")
        .pointers
        .as_mut_ptr();
    // SAFETY: 调用方通过 C ABI 提供了可写的 `char***` 地址，非空检查已在本函数开头完成。
    unsafe { *target = pointer };
    Ok(())
}

unsafe fn copy_string(value: *const c_char, field: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("参数 {field} 不能为空"));
    }
    // SAFETY: 调用方契约要求 `value` 指向以 NUL 结尾的有效 C 字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| format!("参数 {field} 必须是有效 UTF-8 文本"))
}

unsafe fn copy_array(values: *const *const c_char, field: &str) -> Result<Vec<String>, String> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for index in 0..MAX_ARRAY_ITEMS {
        // SAFETY: 调用方契约要求数组以 NULL 结尾；上限避免错误指针无限扫描。
        let value = unsafe { *values.add(index) };
        if value.is_null() {
            return Ok(result);
        }
        result.push(unsafe { copy_string(value, field) }?);
    }
    Err(format!("参数 {field} 缺少 NULL 结尾标记"))
}
