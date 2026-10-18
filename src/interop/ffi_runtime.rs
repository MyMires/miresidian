//! 由属性宏生成的节点 DLL 共用的 FFI 调用支撑。

use std::{
    ffi::{CStr, CString, c_char},
    mem,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, OnceLock},
};

use crate::{
    DeviceNode, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianStatus,
    MiresidianTextArray, NodeRuntime,
};

/// 调用节点并将错误与业务输出合并为一个文本数组。
///
/// 合并数组固定为 `[错误码, 错误信息, ..业务输出]`。错误码为 `0` 时表示成功。
///
/// # Safety
///
/// 所有字符串、数组和输出指针必须符合 C ABI 契约。
pub unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianTextArray,
) -> MiresidianStatus {
    if output.is_null() {
        return MIRESIDIAN_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: 导出函数承诺所有 ABI 指针、数组长度及 UTF-8 文本符合约定；辅助函数仍会
    // 在读取前校验空指针与 UTF-8。
    unsafe {
        invoke_with_writer(
            runtime,
            function,
            config,
            config_count,
            input,
            input_count,
            |status, message, values| {
                let mut combined = vec![status.to_string(), message];
                combined.extend(values);
                write_text_array(output, combined)
            },
        )
    }
}

/// 调用节点并分别写入业务输出与错误输出。
///
/// 成功时 `output` 为业务输出、`error` 为空；失败时 `output` 为空、`error` 为
/// `[错误码, 错误信息]`。
///
/// # Safety
///
/// 所有字符串、数组和输出指针必须符合 C ABI 契约。
#[allow(
    clippy::too_many_arguments,
    reason = "C ABI 需保持 function、两个输入数组及两个输出通道的扁平参数顺序"
)]
pub unsafe fn invoke_with_error<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianTextArray,
    error: *mut MiresidianTextArray,
) -> MiresidianStatus {
    if output.is_null() || error.is_null() {
        return MIRESIDIAN_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: 已验证两个输出指针非空；调用方承诺它们均指向可写的 ABI 结构。
    unsafe {
        *output = MiresidianTextArray::default();
        *error = MiresidianTextArray::default();
        invoke_with_writer(
            runtime,
            function,
            config,
            config_count,
            input,
            input_count,
            |status, message, values| {
                if status == MIRESIDIAN_STATUS_OK {
                    write_text_array(output, values)
                } else {
                    write_text_array(error, vec![status.to_string(), message])
                }
            },
        )
    }
}

/// 调用节点并仅写入业务输出。
///
/// 错误信息不会写入输出数组；调用方只能通过返回状态码获知失败。
///
/// # Safety
///
/// 所有字符串、数组和输出指针必须符合 C ABI 契约。
pub unsafe fn invoke_without_error<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianTextArray,
) -> MiresidianStatus {
    if output.is_null() {
        return MIRESIDIAN_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: 调用方遵守同一 ABI 契约；仅成功调用会将业务输出写入数组。
    unsafe {
        invoke_with_writer(
            runtime,
            function,
            config,
            config_count,
            input,
            input_count,
            |status, _message, values| {
                if status == MIRESIDIAN_STATUS_OK {
                    write_text_array(output, values)
                } else {
                    write_text_array(output, Vec::new())
                }
            },
        )
    }
}

/// 释放任一入口返回的文本数组。
///
/// # Safety
///
/// `value` 必须来自同一 DLL 的入口调用。函数会在释放后重置结构，因此零初始化或已重置的
/// 结构可安全再次传入。
pub unsafe fn free_text_array(value: *mut MiresidianTextArray) {
    if value.is_null() {
        return;
    }
    // SAFETY: 非空指针的有效性是调用前置条件，且只在本块中借用。
    let value = unsafe { &mut *value };
    if !value.values.is_null() {
        // SAFETY: 指针仅由 `write_text_array` 以此精确长度及容量分配。
        let values = unsafe { Vec::from_raw_parts(value.values, value.count, value.count) };
        for item in values {
            if !item.is_null() {
                // SAFETY: 非空元素均由 `CString::into_raw` 生成，所有权在此收回。
                drop(unsafe { CString::from_raw(item) });
            }
        }
    }
    *value = MiresidianTextArray::default();
}

unsafe fn invoke_with_writer<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    write: impl FnOnce(MiresidianStatus, String, Vec<String>) -> Result<(), String>,
) -> MiresidianStatus {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        dispatch(runtime, function, config, config_count, input, input_count)
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
    match write(status, message, values) {
        Ok(()) => status,
        Err(_) => MIRESIDIAN_STATUS_INTERNAL_ERROR,
    }
}

unsafe fn dispatch<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    function: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
) -> Result<Vec<String>, (MiresidianStatus, String)> {
    let function = unsafe { copy_string(function, "function") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let config = unsafe { copy_array(config, config_count, "config") }
        .map_err(|message| (MIRESIDIAN_STATUS_INVALID_ARGUMENT, message))?;
    let input = unsafe { copy_array(input, input_count, "input") }
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

unsafe fn write_text_array(
    target: *mut MiresidianTextArray,
    values: Vec<String>,
) -> Result<(), String> {
    if target.is_null() {
        return Err("输出参数不能为空".to_owned());
    }
    // SAFETY: 调用方承诺非空输出指针指向可写 ABI 结构；先清空以保证失败时没有残留值。
    unsafe { *target = MiresidianTextArray::default() };
    let strings = values
        .into_iter()
        .map(|value| CString::new(value).map_err(|_| "输出文本不能包含 NUL 字符".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut values = strings
        .into_iter()
        .map(CString::into_raw)
        .collect::<Vec<_>>();
    // SAFETY: 数组来自本函数的 Rust 分配，所有权转移给调用方，并由 `free_text_array` 回收。
    unsafe {
        *target = MiresidianTextArray {
            values: values.as_mut_ptr(),
            count: values.len(),
        };
    }
    mem::forget(values);
    Ok(())
}

unsafe fn copy_string(value: *const c_char, field: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("参数 {field} 不能为空"));
    }
    // SAFETY: 调用方保证非空指针指向 NUL 结尾 C 字符串；随后立即复制为 Rust 字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| format!("参数 {field} 必须是有效 UTF-8 文本"))
}

unsafe fn copy_array(
    values: *const *const c_char,
    count: usize,
    field: &str,
) -> Result<Vec<String>, String> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if values.is_null() {
        return Err(format!("参数 {field} 在数量非零时不能为空"));
    }
    (0..count)
        .map(|index| {
            // SAFETY: 调用方保证 `values` 至少有 `count` 个元素；字符串继续由 `copy_string` 校验。
            unsafe { copy_string(*values.add(index), field) }
        })
        .collect()
}
