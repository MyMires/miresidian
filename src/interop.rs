//! 面向 C 与其他语言宿主的互操作边界。
//!
//! 具体 DLL 的导出函数由节点属性宏生成；本模块仅提供稳定 ABI 类型与通用运行支撑。

pub mod ffi;
#[doc(hidden)]
pub mod ffi_runtime;
