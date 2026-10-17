# 串口通信节点开发说明

本文档只说明串口节点的源码职责与扩展边界。测试平台加载 DLL 时可调用的操作、参数、默认值、
返回值和 C ABI 生命周期，以[串口通信发布说明](drivers/串口通信.md)为唯一准则，避免两份调用
文档发生漂移。

## 源码职责

```text
constellation/serial/src/
├── lib.rs              # crate 入口与 SerialNode 导出
├── node.rs             # 节点状态、宏标注、平台契约测试
├── transport.rs        # 串口会话、参数校验和 UTF-8 收发
├── lease.rs            # 跨 DLL 租约子系统入口
└── lease/
    ├── client.rs       # 每个 DLL 中的租约客户端
    ├── protocol.rs     # 本地回环帧协议与无硬件协议测试
    └── broker.rs       # 唯一持有物理串口的代理与状态测试
```

平台方法必须声明在 `node.rs` 的 `SerialNode` 固有 `impl` 中，并由
`#[miresidian::node_impl]` 生成 `DeviceNode` 适配、函数列表、参数元数据和 DLL C ABI。
`transport.rs` 与 `lease/` 不得直接声明平台 Operation。

## 扩展边界

`SerialNode` 是文本传输节点，不绑定任何仪器协议。上层仪器节点可复用 `SerialTransport`，并在
自身 crate 中实现命令、帧边界、校验和、二进制数据或设备识别；这些协议规则不应回流到本节点。

租约通过本机回环端点协调不同目录下的 DLL。物理通信参数必须一致，每个租约可拥有不同超时，
完整“写入—读取”查询在同一端口锁中执行。该回环协议不是面向网络的公共服务，也不是平台调用
入口。
