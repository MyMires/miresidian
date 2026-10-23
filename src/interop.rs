//! 面向 C 与其他语言宿主的互操作边界。
//!
//! 具体 DLL 的导出函数由节点属性宏生成；本模块仅提供稳定 ABI 类型与通用运行支撑。

/// 兼容旧路径的 ABI 类型重导出；新代码可直接依赖 `miresidian_abi`。
pub mod ffi {
    pub use miresidian_abi::*;
}
#[doc(hidden)]
pub mod ffi_runtime;
