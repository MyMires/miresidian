//! Miresidian 节点声明所使用的属性过程宏。

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemImpl, ItemStruct, LitStr};

/// 标记一个可由 Miresidian 加载的节点结构体。
///
/// 参数为平台展示名称，例如 `#[miresidian::node("串口通信")]`。
#[proc_macro_attribute]
pub fn node(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let name = parse_macro_input!(attribute as LitStr);
    let item_struct = parse_macro_input!(item as ItemStruct);
    let identifier = &item_struct.ident;

    quote! {
        #item_struct

        impl #identifier {
            #[doc(hidden)]
            pub const __MIRESIDIAN_NODE_NAME: &'static str = #name;
        }
    }
    .into()
}

/// 标记节点实现块。
///
/// 后续展开逻辑会在此处读取 `operation`、`config`、`input` 与 `output` 标记，
/// 生成节点契约和 ABI 入口。
#[proc_macro_attribute]
pub fn node_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let self_type = &item_impl.self_ty;
    let runtime = format_ident!("__MIRESIDIAN_RUNTIME");

    quote! {
        #item_impl

        static #runtime: ::std::sync::OnceLock<
            ::std::sync::Mutex<::miresidian::NodeRuntime<#self_type>>
        > = ::std::sync::OnceLock::new();

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_v1(
            operation: *const ::std::ffi::c_char,
            config: *const *const ::std::ffi::c_char,
            config_count: usize,
            input: *const *const ::std::ffi::c_char,
            input_count: usize,
            output: *mut ::miresidian::MiresidianOutputV1,
        ) -> ::miresidian::MiresidianStatus {
            unsafe {
                ::miresidian::ffi_runtime::invoke_v1(
                    &#runtime,
                    operation,
                    config,
                    config_count,
                    input,
                    input_count,
                    output,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_free_output_v1(
            output: *mut ::miresidian::MiresidianOutputV1,
        ) {
            unsafe { ::miresidian::ffi_runtime::free_output_v1(output) }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_last_error_v1() -> *const ::std::ffi::c_char {
            ::miresidian::ffi_runtime::last_error_v1()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 {
            1
        }
    }
    .into()
}

/// 标记可由测试平台调用的节点操作。
#[proc_macro_attribute]
pub fn operation(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记初始化配置参数。
#[proc_macro_attribute]
pub fn config(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记操作输入参数。
#[proc_macro_attribute]
pub fn input(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记操作输出。
#[proc_macro_attribute]
pub fn output(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
