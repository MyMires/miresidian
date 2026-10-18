//! 节点运行时静态量与 C ABI 导出令牌生成。

use quote::quote;
use syn::Type;

/// 为一个节点 DLL 生成唯一运行时静态量和稳定 C ABI 导出。
///
/// 所有 C ABI 调用都经运行时辅助函数处理，避免节点代码直接接触裸指针及输出内存所有权。
pub(super) fn exports(self_type: &Type, runtime: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        static #runtime: ::std::sync::OnceLock<::std::sync::Mutex<::miresidian::NodeRuntime<#self_type>>> = ::std::sync::OnceLock::new();
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke(function: *const ::std::ffi::c_char, config: *const *const ::std::ffi::c_char, config_count: usize, input: *const *const ::std::ffi::c_char, input_count: usize, output: *mut ::miresidian::MiresidianTextArray) -> ::miresidian::MiresidianStatus {
            unsafe { ::miresidian::ffi_runtime::invoke(&#runtime, function, config, config_count, input, input_count, output) }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_with_error(function: *const ::std::ffi::c_char, config: *const *const ::std::ffi::c_char, config_count: usize, input: *const *const ::std::ffi::c_char, input_count: usize, output: *mut ::miresidian::MiresidianTextArray, error: *mut ::miresidian::MiresidianTextArray) -> ::miresidian::MiresidianStatus {
            unsafe { ::miresidian::ffi_runtime::invoke_with_error(&#runtime, function, config, config_count, input, input_count, output, error) }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_without_error(function: *const ::std::ffi::c_char, config: *const *const ::std::ffi::c_char, config_count: usize, input: *const *const ::std::ffi::c_char, input_count: usize, output: *mut ::miresidian::MiresidianTextArray) -> ::miresidian::MiresidianStatus {
            unsafe { ::miresidian::ffi_runtime::invoke_without_error(&#runtime, function, config, config_count, input, input_count, output) }
        }
        #[unsafe(no_mangle)] pub unsafe extern "C" fn miresidian_free_text_array(value: *mut ::miresidian::MiresidianTextArray) { unsafe { ::miresidian::ffi_runtime::free_text_array(value) } }
        #[unsafe(no_mangle)] pub extern "C" fn miresidian_abi_version() -> u32 { 2 }
    }
}
