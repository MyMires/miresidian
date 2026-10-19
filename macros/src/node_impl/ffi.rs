//! 节点 DLL 的 JSON 文本 ABI 导出生成。

use quote::quote;
use syn::Type;

/// 为每个节点生成唯一的 C ABI 入口；JSON 解析、输出写入与内存边界由运行时统一实现。
pub(super) fn exports(self_type: &Type, runtime: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        static #runtime: ::std::sync::OnceLock<::std::sync::Mutex<::miresidian::NodeRuntime<#self_type>>> = ::std::sync::OnceLock::new();

        /// 以 JSON 响应对象返回业务输出或错误信息的 DLL 入口。
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke(
            function: *const ::std::ffi::c_char,
            config_json: *const ::std::ffi::c_char,
            input_json: *const ::std::ffi::c_char,
            output_json: *mut ::std::ffi::c_char,
            output_capacity: u32,
        ) -> ::miresidian::MiresidianStatus {
            unsafe {
                ::miresidian::ffi_runtime::invoke(
                    &#runtime,
                    function,
                    config_json,
                    input_json,
                    output_json,
                    output_capacity,
                )
            }
        }

        /// 返回当前固定 C ABI 的版本号。
        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            4
        }
    }
}
