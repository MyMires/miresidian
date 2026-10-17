//! 跨 DLL 串口租约。
//!
//! 租约作为完整子系统，内部按客户端、协议和代理端分层；节点其他模块只通过
//! `SerialLease` 使用它。

mod broker;
mod client;
mod protocol;

pub(super) use client::SerialLease;
