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
        pub unsafe extern "C" fn miresidian_invoke_v1(operation: *const ::std::ffi::c_char, config: *const *const ::std::ffi::c_char, config_count: usize, input: *const *const ::std::ffi::c_char, input_count: usize, output: *mut ::miresidian::MiresidianOutputV1) -> ::miresidian::MiresidianStatus {
            unsafe { ::miresidian::ffi_runtime::invoke_v1(&#runtime, operation, config, config_count, input, input_count, output) }
        }
        #[unsafe(no_mangle)] pub unsafe extern "C" fn miresidian_free_output_v1(output: *mut ::miresidian::MiresidianOutputV1) { unsafe { ::miresidian::ffi_runtime::free_output_v1(output) } }
        #[unsafe(no_mangle)] pub extern "C" fn miresidian_last_error_v1() -> *const ::std::ffi::c_char { ::miresidian::ffi_runtime::last_error_v1() }
        #[unsafe(no_mangle)] pub extern "C" fn miresidian_abi_version() -> u32 { 1 }
    }
}
