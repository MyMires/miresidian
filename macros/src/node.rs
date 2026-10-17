//! `#[miresidian::node]` 结构体标注的展开逻辑。

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, LitStr, parse_macro_input};

/// 生成节点展示名称常量，供 `node_impl` 生成的描述符读取。
pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
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
