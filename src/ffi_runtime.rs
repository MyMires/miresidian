//! 由属性宏生成的驱动 DLL 共用的 FFI 调用支撑。

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    mem,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::{Mutex, OnceLock},
};

use crate::{
    DeviceNode, Invocation, MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianOutputV1, MiresidianStatus,
    NodeRuntime,
};

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

/// 由宏生成的 `miresidian_invoke_v1` 调用。
///
/// # Safety
///
/// 指针和数组必须满足 C ABI v1 的有效性与 UTF-8 约定。
pub unsafe fn invoke_v1<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    operation: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianOutputV1,
) -> MiresidianStatus {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        invoke(
            runtime,
            operation,
            config,
            config_count,
            input,
            input_count,
            output,
        )
    })) {
        Ok(Ok(())) => MIRESIDIAN_STATUS_OK,
        Ok(Err(message)) => {
            set_error(&message);
            if message.starts_with("参数 ") || message.starts_with("输出参数") {
                MIRESIDIAN_STATUS_INVALID_ARGUMENT
            } else {
                MIRESIDIAN_STATUS_OPERATION_FAILED
            }
        }
        Err(_) => {
            set_error("Miresidian 内部发生未预期错误");
            MIRESIDIAN_STATUS_INTERNAL_ERROR
        }
    }
}

/// 释放由宏生成入口返回的输出。
///
/// # Safety
///
/// `output` 必须来自同一 DLL 的成功调用，且不得重复释放。
pub unsafe fn free_output_v1(output: *mut MiresidianOutputV1) {
    if output.is_null() {
        return;
    }
    let output = unsafe { &mut *output };
    if !output.values.is_null() {
        let values = unsafe { Vec::from_raw_parts(output.values, output.count, output.count) };
        for value in values {
            if !value.is_null() {
                drop(unsafe { CString::from_raw(value) });
            }
        }
    }
    *output = MiresidianOutputV1::default();
}

/// 返回当前线程最近一次由宏入口产生的错误。
pub fn last_error_v1() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr())
    })
}

unsafe fn invoke<N: DeviceNode>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    operation: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianOutputV1,
) -> Result<(), String> {
    if output.is_null() {
        return Err("输出参数 output 不能为空".to_owned());
    }
    unsafe { *output = MiresidianOutputV1::default() };
    let operation = unsafe { copy_string(operation, "operation") }?;
    let config = unsafe { copy_array(config, config_count, "config") }?;
    let input = unsafe { copy_array(input, input_count, "input") }?;
    let result = runtime
        .get_or_init(|| Mutex::new(NodeRuntime::new()))
        .lock()
        .map_err(|_| "节点状态锁已损坏".to_owned())?
        .invoke(&Invocation::new(operation, config, input))
        .map_err(|error| error.to_string())?;
    let strings = result
        .into_values()
        .into_iter()
        .map(|value| CString::new(value).map_err(|_| "输出文本不能包含 NUL 字符".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut values = strings
        .into_iter()
        .map(CString::into_raw)
        .collect::<Vec<_>>();
    unsafe {
        *output = MiresidianOutputV1 {
            values: values.as_mut_ptr(),
            count: values.len(),
        };
    }
    mem::forget(values);
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
    Ok(())
}

unsafe fn copy_string(value: *const c_char, field: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("参数 {field} 不能为空"));
    }
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
        .map(|index| unsafe { copy_string(*values.add(index), field) })
        .collect()
}

fn set_error(message: &str) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(message.replace('\0', "\\0")).ok());
}
