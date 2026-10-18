//! 节点 DLL 的平台 ABI 导出生成。

use quote::quote;
use syn::Type;

/// 为每个节点生成固定的 C ABI 入口；具体字符串数组规则由运行时统一实现。
pub(super) fn exports(self_type: &Type, runtime: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        static #runtime: ::std::sync::OnceLock<::std::sync::Mutex<::miresidian::NodeRuntime<#self_type>>> = ::std::sync::OnceLock::new();

        /// 以合并数组返回错误与业务输出的 DLL 入口。
        #[allow(non_snake_case, reason = "公开 ABI 参数名必须与平台通道名称一致")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke(
            Function: *const ::std::ffi::c_char,
            Config: *const *const ::std::ffi::c_char,
            Input: *const *const ::std::ffi::c_char,
            Output: *mut *mut *mut ::std::ffi::c_char,
        ) -> ::miresidian::MiresidianStatus {
            unsafe { ::miresidian::ffi_runtime::invoke(&#runtime, Function, Config, Input, Output) }
        }

        /// 以独立数组返回业务输出与错误信息的 DLL 入口。
        #[allow(non_snake_case, reason = "公开 ABI 参数名必须与平台通道名称一致")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_with_error(
            Function: *const ::std::ffi::c_char,
            Config: *const *const ::std::ffi::c_char,
            Input: *const *const ::std::ffi::c_char,
            Output: *mut *mut *mut ::std::ffi::c_char,
            Error: *mut *mut *mut ::std::ffi::c_char,
        ) -> ::miresidian::MiresidianStatus {
            unsafe {
                ::miresidian::ffi_runtime::invoke_with_error(
                    &#runtime, Function, Config, Input, Output, Error,
                )
            }
        }

        /// 只返回业务输出的 DLL 入口。
        #[allow(non_snake_case, reason = "公开 ABI 参数名必须与平台通道名称一致")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_without_error(
            Function: *const ::std::ffi::c_char,
            Config: *const *const ::std::ffi::c_char,
            Input: *const *const ::std::ffi::c_char,
            Output: *mut *mut *mut ::std::ffi::c_char,
        ) -> ::miresidian::MiresidianStatus {
            unsafe {
                ::miresidian::ffi_runtime::invoke_without_error(
                    &#runtime, Function, Config, Input, Output,
                )
            }
        }

        /// 返回当前固定 C ABI 的版本号。
        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            3
        }
    }
}
