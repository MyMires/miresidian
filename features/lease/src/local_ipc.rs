//! 跨 DLL 租约：客户端会话、服务、线协议与跨进程配置锁。
//!
//! 资源文件不能删除；存活客户端在服务重启期间仍持有同一文件的共享锁。
pub(crate) mod client;
#[cfg(unix)]
mod module;
pub(crate) mod protocol;
pub(crate) mod resource;
pub(crate) mod service;

pub use client::LocalLease;
// 保留底层 IPC 原语的公开路径。
pub use interprocess::local_socket::{
    ConnectOptions, GenericNamespaced, Listener, ListenerOptions, Stream, ToNsName, prelude::*,
};
