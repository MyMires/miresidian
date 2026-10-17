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
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
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
    // SAFETY: 此公开函数的调用方已承诺所有 ABI 指针、数组长度和 UTF-8 文本满足 v1
    // 契约；`invoke` 在读取每个指针前仍执行空指针与文本校验。
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
/// `output` 必须来自同一 DLL 的成功调用。函数会在释放后将其重置为默认值，因此零初始化或已
/// 重置的输出可安全再次传入；不得传入任意伪造的非空指针。
pub unsafe fn free_output_v1(output: *mut MiresidianOutputV1) {
    if output.is_null() {
        return;
    }
    // SAFETY: 非空指针的有效性是 `free_output_v1` 的调用前置条件，且只在本块中借用。
    let output = unsafe { &mut *output };
    if !output.values.is_null() {
        // SAFETY: 成功调用仅以此精确长度和容量分配 `values`，释放后会立刻重置输出结构。
        let values = unsafe { Vec::from_raw_parts(output.values, output.count, output.count) };
        for value in values {
            if !value.is_null() {
                // SAFETY: 数组中的非空元素均由 `CString::into_raw` 生成，所有权在此收回。
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
    // SAFETY: 已在函数开头拒绝空输出指针；调用方保证其指向可写的 ABI 结构。
    unsafe { *output = MiresidianOutputV1::default() };
    // SAFETY: 各辅助函数根据 C ABI 契约读取 NUL 结尾字符串和数组，并在内部验证空指针。
    let operation = unsafe { copy_string(operation, "operation") }?;
    // SAFETY: 同上；数量为零时允许空数组指针。
    let config = unsafe { copy_array(config, config_count, "config") }?;
    // SAFETY: 同上；数量为零时允许空数组指针。
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
    // SAFETY: `values` 来自本函数的 Rust 分配，指针与长度将作为 ABI 输出移交并由
    // `miresidian_free_output_v1` 使用完全相同的长度和容量回收。
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
            // SAFETY: 调用方保证 `values` 指向至少 `count` 个指针；每个元素的字符串
            // 有效性继续由 `copy_string` 验证。
            unsafe { copy_string(*values.add(index), field) }
        })
        .collect()
}

fn set_error(message: &str) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(message.replace('\0', "\\0")).ok());
}
