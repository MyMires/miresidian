//! 现有 lease 标签自动共享；初始化直接打开独占资源。
//! 与 leased_other_dll 分别编译并保持相同节点接口及初始化状态，Lease 模式共用业务对象。
//! 构造只由资源所有者执行，最后引用关闭并析构；模拟文件锁不代替真实设备验收。
#[path = "support/leased_node.rs"]
mod node;
