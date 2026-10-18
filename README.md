# Miresidian

Miresidian 是面向测试设备、DUT 通信与测试平台通用能力的 Rust 运行时框架。它为不同语言的测试平台提供统一、可演进的调用模型。

## 当前基础层

当前实现已经具备可编译、可测试的运行时：

- 使用 `operation + config + input -> output` 的统一文本调用契约；
- 保持与既有 C# 驱动兼容的固定操作：`GetFunctions`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`；
- 通过 `DeviceNode` 定义节点生命周期与自定义操作的扩展点；
- 通过 `NodeRuntime` 管理初始化状态、临时端口探测和资源关闭；
- 提供受跨语言边界约束的逻辑值类型 `DeviceValue`。
- 提供框架级数据编解码：基础类型、可逆数组、ASCII、GBK、UTF-8、Unicode、HEX 与 Base64。
- 提供基于成熟 `serialport` crate 的串口通信节点 `SerialNode`，包含串口参数验证、端口探测和 UTF-8 文本收发。

已导出 C ABI v3：`miresidian_invoke`、`miresidian_invoke_with_error`、`miresidian_invoke_without_error` 与 `miresidian_abi_version`。三个入口均使用 `Function + Config[] + Input[]`，分别提供合并结果、独立错误通道和仅业务输出三种平台适配方式；数组以 NULL 结尾，不暴露计数字段或 Rust 结构体。属性宏（`#[miresidian::node]`、`#[miresidian::operation]`、`#[miresidian::config]`、`#[miresidian::input]`、`#[miresidian::output]`）已经生成节点契约、类型转换、生命周期调度和 DLL 入口。

## 运行时源码结构

`src/` 按职责分组，所有模块均采用 `模块.rs + 模块/子模块.rs` 的新式结构，不使用
`mod.rs`：

```text
src/
├── foundation.rs          # 基础数据、错误与数据编解码入口
├── foundation/
│   ├── codec.rs           # 编解码公共 API、类型转换与格式定义
│   ├── codec/             # 标量、数组、字节、文本与参数定义规则
│   ├── error.rs           # MiresidianError 与 MiresidianResult
│   └── value.rs           # 跨语言值模型
├── runtime.rs             # 节点运行时入口
├── runtime/
│   ├── contract.rs        # 调用契约与固定操作名称
│   ├── descriptor.rs      # 节点与操作描述信息
│   ├── lifecycle.rs       # 初始化、关闭与调用调度
│   └── node.rs            # 节点实现接口
├── interop.rs             # C ABI 入口
├── interop/
│   ├── ffi.rs             # 公共 ABI 类型与状态码
│   └── ffi_runtime.rs     # 宏生成 DLL 入口使用的 ABI 支撑
└── bin/package_node.rs     # 节点发布包整理工具
```

对外公开的 Rust 路径保持不变，例如 `miresidian::NodeRuntime`、
`miresidian::ffi_runtime::invoke`，已有节点和
宏无需修改。

具体通信实现不属于框架根 crate。每个实例节点将其通信细节与私有依赖保留在自身目录；
例如串口通信节点使用以下结构：

```text
constellation/serial/src/
├── lib.rs                  # 模块入口与 SerialNode 公开导出
├── node.rs                 # 节点状态与平台方法
├── transport.rs            # 串口会话、参数校验与文本收发
├── lease.rs                # 跨 DLL 租约模块入口
└── lease/
    ├── client.rs           # 每个 DLL 内的租约客户端
    ├── protocol.rs         # 客户端与代理共用的回环协议
    └── broker.rs           # 持有物理串口的本地代理和端口池

constellation/serial/tests/
└── node.rs                 # 通过公开 API 验证 SerialNode 平台契约
```

因此新增 TCP、USB、CAN 等节点时，只需在对应 `constellation/<节点>/` crate 中引入通信库；
`miresidian` 框架本身不会依赖这些具体传输库。

## 属性宏与节点扩展

`macros/` 是声明式节点扩展的唯一代码生成边界。驱动 crate 只描述节点状态、通信实现和
带标注的方法；不得手写平台函数列表、字符串分发、`DeviceNode` 实现或 C ABI 导出。宏项目
与运行时、实例节点遵循同一套模块规则：入口文件只做路由，同一概念拥有多个协作职责时使用
`概念.rs + 概念/`。

```text
macros/src/
├── lib.rs                 # 五个属性宏的公开入口
├── node.rs                # #[node("节点名称")] 的展开
├── node_impl.rs           # #[node_impl] 的编排入口
└── node_impl/
    ├── definition.rs      # 属性解析、签名校验和声明模型
    ├── naming.rs          # Rust 名称到平台名称的转换
    ├── descriptor.rs      # 节点与操作元数据生成
    ├── dispatch.rs        # 参数解析与操作调度生成
    └── ffi.rs             # 节点 DLL 的 C ABI 导出生成
```

新增节点时先阅读 [宏驱动开发说明](docs/宏驱动开发.md)，再在
`constellation/<节点>/` 中创建独立 crate。宏标注、自动命名、固定入口和可使用的数据类型均以
该文档为准。

## 验证

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

编码、文档、FFI 与测试约束见 [docs/coding-standards.md](docs/coding-standards.md)。
数据格式、文本编码与数组表示规则见 [docs/data-codec.md](docs/data-codec.md)。
串口通信节点的配置与操作说明见 [docs/drivers/串口通信.md](docs/drivers/串口通信.md)。
C DLL 调用约定和 Windows DLL 构建说明见 [docs/c-abi.md](docs/c-abi.md)。
构建容器与 `cc` 链接器的安装说明见 [docs/build-environment.md](docs/build-environment.md)。
