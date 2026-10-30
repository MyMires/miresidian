//! 节点 DLL 的 JSON 文本 ABI 导出生成。

use quote::quote;
use syn::Type;

/// 为每个节点生成唯一的 C ABI 入口；JSON 解析、响应保存与内存边界由运行时统一实现。
pub(super) fn exports(self_type: &Type, runtime: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        static #runtime: ::std::sync::OnceLock<::miresidian::NodeRuntime<#self_type>> = ::std::sync::OnceLock::new();

        /// 以 DLL 持有的 JSON 响应文本返回状态、消息和业务输出的 DLL 入口。
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke(
            function: *const ::std::ffi::c_char,
            config_json: *const ::std::ffi::c_char,
            input_json: *const ::std::ffi::c_char,
        ) -> *const ::std::ffi::c_char {
            unsafe {
                ::miresidian::ffi_runtime::invoke(
                    &#runtime,
                    function,
                    config_json,
                    input_json,
                )
            }
        }

        /// 使用显式平台注册 id 调用 DLL；对应 Rust 的 `NodeRuntime::invoke_with`。
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_with(
            platform: u32,
            function: *const ::std::ffi::c_char,
            config_json: *const ::std::ffi::c_char,
            input_json: *const ::std::ffi::c_char,
        ) -> *const ::std::ffi::c_char {
            unsafe {
                ::miresidian::ffi_runtime::invoke_with(
                    &#runtime,
                    platform,
                    function,
                    config_json,
                    input_json,
                )
            }
        }

        /// 返回当前固定 C ABI 的版本号。
        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            ::miresidian::MIRESIDIAN_ABI_VERSION
        }
    }
}
