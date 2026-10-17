# Miresidian

Miresidian 是面向测试设备、DUT 通信与测试平台通用能力的 Rust 运行时框架。它为不同语言的测试平台提供统一、可演进的调用模型。

## 当前基础层

当前实现已经具备可编译、可测试的运行时：

- 使用 `operation + config + input -> output` 的统一文本调用契约；
- 保持与既有 C# 驱动兼容的固定操作：`GetFunctions`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`；
- 通过 `DeviceNode` 定义节点生命周期与自定义操作的扩展点；
- 通过 `NodeRuntime` 管理初始化状态、临时端口探测和资源关闭；
- 提供受跨语言边界约束的逻辑值类型 `DeviceValue`。
- 提供基于成熟 `serialport` crate 的串口通信节点 `SerialNode`，包含串口参数验证、端口探测和 UTF-8 文本收发。

已导出 C ABI v1：`miresidian_invoke_v1`、`miresidian_free_output_v1`、`miresidian_last_error_v1` 与 `miresidian_abi_version`。属性宏（`#[miresidian::node]`、`#[miresidian::operation]`、`#[miresidian::config]`、`#[miresidian::input]`、`#[miresidian::output]`）将在下一阶段基于本层契约实现，避免生成没有实际调度行为的空宏。

## 验证

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

编码、文档、FFI 与测试约束见 [docs/coding-standards.md](docs/coding-standards.md)。
串口通信节点的配置与操作说明见 [docs/serial-node.md](docs/serial-node.md)。
C DLL 调用约定和 Windows DLL 构建说明见 [docs/c-abi-v1.md](docs/c-abi-v1.md)。
构建容器与 `cc` 链接器的安装说明见 [docs/build-environment.md](docs/build-environment.md)。
