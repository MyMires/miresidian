//! 节点 DLL 的 JSON 文本 ABI 导出生成。

use quote::quote;
use syn::Type;

/// 为每个节点生成统一的 C ABI 入口；JSON 解析、响应保存与内存边界由运行时统一实现。
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

        /// 接收完整 JSON 请求，省略 platform 时由初始化 Config 选择平台。
        ///
        /// # Safety
        /// 非 NULL request_json 必须指向调用期间有效、NUL 结尾的 UTF-8 C 字符串。
        /// 返回响应由 DLL 持有，须在同线程下一次调用任一入口前复制，不得释放。
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_request(
            request_json: *const ::std::ffi::c_char,
        ) -> *const ::std::ffi::c_char {
            // SAFETY: 输入指针契约由调用方保证，运行时校验 NULL/UTF-8/JSON。
            unsafe {
                ::miresidian::ffi_runtime::invoke_request(&#runtime, request_json)
            }
        }

        /// 返回当前固定 C ABI 的版本号。
        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            ::miresidian::MIRESIDIAN_ABI_VERSION
        }
    }
}
