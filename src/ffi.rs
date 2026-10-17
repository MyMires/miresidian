//! Miresidian C ABI v1。
//!
//! 本模块将 `SerialNode` 的统一调用模型导出为稳定的 C 接口。外部调用方只能
//! 传递 UTF-8 文本和指针数组；Rust 的 `String`、`Vec` 和 trait 对象不会越过 ABI 边界。

use std::{
    cell::RefCell,
    ffi::{c_char, c_uint, CStr, CString},
    mem,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
    sync::{Mutex, OnceLock},
};

use crate::{DriverError, Invocation, NodeRuntime, SerialNode};

/// C ABI 调用成功。
pub const MIRESIDIAN_STATUS_OK: MiresidianStatus = 0;
/// C ABI 参数为空、格式错误或不是有效 UTF-8。
pub const MIRESIDIAN_STATUS_INVALID_ARGUMENT: MiresidianStatus = 1;
/// 节点操作、设备访问或通信失败。
pub const MIRESIDIAN_STATUS_OPERATION_FAILED: MiresidianStatus = 2;
/// Rust 内部发生了已捕获的 panic。
pub const MIRESIDIAN_STATUS_INTERNAL_ERROR: MiresidianStatus = 3;

/// C ABI 函数返回的状态码。
pub type MiresidianStatus = i32;

/// 由 `miresidian_invoke_v1` 生成的 UTF-8 输出字符串数组。
///
/// 调用方必须将此结构体零初始化；成功使用后必须调用
/// `miresidian_free_output_v1`，且不得自行释放 `values` 或其中的字符串。
#[repr(C)]
#[derive(Debug)]
pub struct MiresidianOutputV1 {
    /// 指向 UTF-8、以 NUL 结尾字符串指针的数组。
    pub values: *mut *mut c_char,
    /// `values` 中的字符串数量。
    pub count: usize,
}

impl Default for MiresidianOutputV1 {
    fn default() -> Self {
        Self {
            values: ptr::null_mut(),
            count: 0,
        }
    }
}

static SERIAL_RUNTIME: OnceLock<Mutex<NodeRuntime<SerialNode>>> = OnceLock::new();

thread_local! {
    /// 错误文本按调用线程保存，避免不同平台线程互相覆盖诊断信息。
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

/// 返回 ABI 的当前版本号。
pub extern "C" fn miresidian_abi_version() -> c_uint {
    1
}

/// 调用串口通信节点的固定操作或自定义操作。
///
/// # Safety
///
/// `operation` 必须是有效、以 NUL 结尾的 UTF-8 字符串。`config` 和 `input`
/// 在各自计数非零时必须指向连续的字符串指针数组，数组中的每个指针也必须指向
/// 有效、以 NUL 结尾的 UTF-8 字符串。`output` 必须指向可写且已零初始化的
/// `MiresidianOutputV1`。调用方仅能用 `miresidian_free_output_v1` 释放成功结果。
pub unsafe extern "C" fn miresidian_invoke_v1(
    operation: *const c_char,
    config: *const *const c_char,
    config_count: usize,
    input: *const *const c_char,
    input_count: usize,
    output: *mut MiresidianOutputV1,
) -> MiresidianStatus {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        invoke_impl(operation, config, config_count, input, input_count, output)
    })) {
        Ok(Ok(())) => MIRESIDIAN_STATUS_OK,
        Ok(Err(error)) => {
            set_last_error(&error);
            status_for_error(&error)
        }
        Err(_) => {
            set_last_error("Miresidian 内部发生未预期错误");
            MIRESIDIAN_STATUS_INTERNAL_ERROR
        }
    }
}

/// 释放 `miresidian_invoke_v1` 成功返回的输出数组。
///
/// # Safety
///
/// `output` 必须是由 `miresidian_invoke_v1` 写入的同一结构体，且在释放前不得
/// 被修改或重复释放。成功释放后，结构体会被重置为零值，可安全再次使用。
pub unsafe extern "C" fn miresidian_free_output_v1(output: *mut MiresidianOutputV1) {
    if output.is_null() {
        return;
    }

    // 安全性：由调用方契约保证该结构体来自本模块的成功调用。
    let output = unsafe { &mut *output };
    if !output.values.is_null() {
        // 安全性：数组由 allocate_output 使用相同长度和容量分配。
        let values = unsafe { Vec::from_raw_parts(output.values, output.count, output.count) };
        for value in values {
            if !value.is_null() {
                // 安全性：每个字符串均由 CString::into_raw 创建。
                drop(unsafe { CString::from_raw(value) });
            }
        }
    }
    *output = MiresidianOutputV1::default();
}

/// 返回当前线程最近一次 ABI 调用错误的 UTF-8 文本。
///
/// 返回指针由 Miresidian 管理；下一次本线程 ABI 调用后该指针可能失效，外部调用方
/// 如需长期保存，必须立即复制。没有错误时返回空指针。
pub extern "C" fn miresidian_last_error_v1() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(ptr::null(), |message| message.as_ptr())
    })
}

unsafe fn invoke_impl(
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
    // 安全性：调用方已承诺 output 指向可写的零初始化结构体。
    unsafe { *output = MiresidianOutputV1::default() };

    let operation = unsafe { copy_string(operation, "operation") }?;
    let config = unsafe { copy_string_array(config, config_count, "config") }?;
    let input = unsafe { copy_string_array(input, input_count, "input") }?;
    let invocation = Invocation::new(operation, config, input);

    let runtime = SERIAL_RUNTIME.get_or_init(|| Mutex::new(NodeRuntime::new()));
    let output_values = runtime
        .lock()
        .map_err(|_| "串口通信节点状态锁已损坏".to_owned())?
        .invoke(&invocation)
        .map_err(driver_error_text)?;
    let output_values = allocate_output(output_values.into_values())?;

    // 安全性：调用方已承诺 output 指向可写结构体。
    unsafe { *output = output_values };
    clear_last_error();
    Ok(())
}

unsafe fn copy_string(value: *const c_char, field: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("参数 {field} 不能为空"));
    }
    // 安全性：调用方契约保证 value 是有效的 NUL 结尾字符串。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| format!("参数 {field} 必须是有效 UTF-8 文本"))
}

unsafe fn copy_string_array(
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

    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        // 安全性：调用方契约保证 values 指向 count 个连续的字符串指针。
        let value = unsafe { *values.add(index) };
        result.push(unsafe { copy_string(value, field) }?);
    }
    Ok(result)
}

fn allocate_output(values: Vec<String>) -> Result<MiresidianOutputV1, String> {
    let strings = values
        .into_iter()
        .map(|value| {
            CString::new(value).map_err(|_| "输出文本不能包含 NUL 字符".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut pointers = strings
        .into_iter()
        .map(CString::into_raw)
        .collect::<Vec<_>>();
    let output = MiresidianOutputV1 {
        values: pointers.as_mut_ptr(),
        count: pointers.len(),
    };
    mem::forget(pointers);
    Ok(output)
}

fn status_for_error(error: &str) -> MiresidianStatus {
    if error.starts_with("参数 ") || error.starts_with("输出参数") {
        MIRESIDIAN_STATUS_INVALID_ARGUMENT
    } else {
        MIRESIDIAN_STATUS_OPERATION_FAILED
    }
}

fn driver_error_text(error: DriverError) -> String {
    error.to_string()
}

fn set_last_error(message: &str) {
    let sanitized = message.replace('\0', "\\0");
    let message = CString::new(sanitized).ok();
    LAST_ERROR.with(|slot| *slot.borrow_mut() = message);
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 固定操作可通过_c_abi_调用() {
        let operation = CString::new("GetFunctions").unwrap();
        let mut output = MiresidianOutputV1::default();

        let status = unsafe {
            miresidian_invoke_v1(
                operation.as_ptr(),
                ptr::null(),
                0,
                ptr::null(),
                0,
                &mut output,
            )
        };

        assert_eq!(status, MIRESIDIAN_STATUS_OK);
        assert!(output.count >= 8);
        unsafe { miresidian_free_output_v1(&mut output) };
        assert!(output.values.is_null());
    }

    #[test]
    fn 无效指针会返回参数错误() {
        let mut output = MiresidianOutputV1::default();

        let status = unsafe {
            miresidian_invoke_v1(
                ptr::null(),
                ptr::null(),
                0,
                ptr::null(),
                0,
                &mut output,
            )
        };

        assert_eq!(status, MIRESIDIAN_STATUS_INVALID_ARGUMENT);
        assert!(!miresidian_last_error_v1().is_null());
    }
}
