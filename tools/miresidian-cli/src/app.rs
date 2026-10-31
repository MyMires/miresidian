//! 统一入口只进行进程调度，不依赖或合并任何后端工具库。
mod dispatch;
mod routing;

pub(crate) use dispatch::run;
