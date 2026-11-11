//! 节点 DLL 的 JSON 文本 ABI 导出生成。

use quote::quote;
use syn::Type;

/// 为每个节点生成统一的 C ABI 入口；JSON 解析、响应保存与内存边界由运行时统一实现。
pub(super) fn exports(self_type: &Type, runtime: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        static #runtime: ::std::sync::OnceLock<::miresidian::NodeRuntime<#self_type>> = ::std::sync::OnceLock::new();

        /// 接收完整 JSON 请求，platform 在本次调用时确定，不使用 Config 平台选择。
        ///
        /// # Safety
        /// 非 NULL request_json 必须指向调用期间有效、NUL 结尾的 UTF-8 C 字符串。
        /// 返回值由 DLL 持有，须在同线程下一次调用前复制，不得释放。
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke(
            request_json: *const ::std::ffi::c_char,
        ) -> *const ::std::ffi::c_char {
            // SAFETY: 指针有效性由调用方保证，运行时检查 NULL/UTF-8/JSON。
            unsafe { ::miresidian::ffi_runtime::invoke(&#runtime, request_json) }
        }

        /// 返回当前固定 C ABI 的版本号。
        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            ::miresidian::MIRESIDIAN_ABI_VERSION
        }
    }
}
