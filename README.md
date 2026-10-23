# Miresidian

Miresidian 是面向测试设备、DUT 通信与测试平台通用能力的 Rust 运行时框架。它为不同语言的测试平台提供统一、可演进的调用模型。

## 当前基础层

当前实现已经具备可编译、可测试的运行时。数据编解码、参数解析、跨语言值模型与通用错误类型位于独立的 `miresidian_codec` crate；`miresidian` 保持兼容重导出：

- 使用 `operation + config + input -> output` 的统一文本调用契约；
- 保持与既有 C# 驱动兼容的固定操作：`GetFunctions`、`GetFrameworkVersion`、`GetProjectVersion`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`；
- 通过 `DeviceNode` 定义节点生命周期与自定义操作的扩展点；
- 通过 `NodeRuntime` 按完整 Config 管理多通道对象、临时端口探测和资源关闭；
- 支持以 `Input[0] = "共享"` 对所有已创建通道批量执行自定义操作，并由操作标注选择顺序或并行分发；
- 提供受跨语言边界约束的逻辑值类型 `DeviceValue`。
- 提供框架级数据编解码：基础类型、可逆数组、ASCII、GBK、UTF-8、Unicode、HEX 与 Base64。
- 提供基于成熟 `serialport` crate 的串口通信节点 `SerialNode`，包含串口参数验证、端口探测和 UTF-8 文本收发。

已导出 C ABI v5：`miresidian_invoke` 与 `miresidian_abi_version`。唯一入口使用 `function + config_json + input_json`；字符串数组统一编码为 UTF-8 JSON 文本，DLL 返回内部持有的 JSON 响应文本，固定包含 `status`、`message` 与 `output`。调用方不提供输出缓冲区、不传容量，也不得释放返回指针。属性宏（`#[miresidian::node]`、`#[miresidian::operation]`、`#[miresidian::config]`、`#[miresidian::input]`、`#[miresidian::output]`）已经生成节点契约、类型转换、生命周期调度和 DLL 入口。

## 运行时源码结构

`src/` 按职责分组，所有模块均采用 `模块.rs + 模块/子模块.rs` 的新式结构，不使用
`mod.rs`：

```text
codec/                     # 独立的数据编解码、解析和值模型 crate
├── src/lib.rs              # 公开 API、错误和平台输出类型
└── src/                    # 标量、数组、字节、文本与参数定义规则

src/
├── runtime.rs             # 节点运行时入口
├── runtime/
│   ├── contract.rs        # 调用契约与固定操作名称
│   ├── descriptor.rs      # 节点与操作描述信息
│   ├── lifecycle.rs       # 多通道初始化、关闭与调用调度
│   ├── shared.rs          # 共享调用的参数校验与顺序/并行/广播分发
│   └── node.rs            # 节点实现接口
├── interop.rs             # C ABI 入口
├── interop/
│   ├── ffi.rs             # 公共 ABI 类型与状态码
│   └── ffi_runtime.rs     # 宏生成 DLL 入口使用的 ABI 支撑

```

节点发布工具位于 `tools/miresidian-pack/`，不参与框架运行时依赖。

工具的输入、输出与命令用法分别见 [打包工具说明](tools/miresidian-pack/README.md) 和 [Windows TestFlow 工具说明](tools/miresidian-testflow-windows/README.md)。

工具 crate 发布后，在节点项目所在机器安装一次：

```text
cargo install miresidian_pack
cargo install miresidian_testflow_windows
```

随后直接使用 `cargo miresidian-pack` 与 `cargo miresidian-testflow-windows`；两者均不进入节点 DLL 的运行时依赖图。

普通自定义操作必须传入创建该对象时完全相同的 `Config[]`；运行时据此定位通道。共享调用不使用本次调用的 `Config[]`，而是按对象创建顺序把 `Input[1..]` 的每一项分发到对应通道。详细的跳过规则、输出槽位和错误语义见[平台调用契约](docs/platform-contract.md)。

对外公开的 Rust 路径保持不变，例如 `miresidian::NodeRuntime`、
`miresidian::ffi_runtime::invoke`。节点需将普通调用的 Config 调整为通道键；其余宏生成代码无需手写。

具体通信实现不属于框架根 crate。每个实例节点将其通信细节与私有依赖保留在自身目录；
例如串口通信节点使用以下结构：

```text
../miresidian_serial/src/
├── lib.rs                  # 模块入口与 SerialNode 公开导出
├── node.rs                 # 节点状态与平台方法
├── transport.rs            # 串口会话、参数校验与文本收发
├── lease.rs                # 跨 DLL 租约模块入口
└── lease/
    ├── client.rs           # 每个 DLL 内的租约客户端
    ├── protocol.rs         # 客户端与代理共用的回环协议
    └── broker.rs           # 持有物理串口的本地代理和端口池

../miresidian_serial/tests/
└── node.rs                 # 通过公开 API 验证 SerialNode 平台契约

../miresidian_serial/tools/
├── serial_smoke.testflow     # 串口节点烟测流程
└── serial_smoke.bat          # 由 TestFlow 工具生成的入口
```

因此新增 TCP、USB、CAN 等节点时，只需在各自独立的节点项目中引入通信库；
`miresidian` 框架本身不会依赖这些具体传输库。

## crate 边界

- `miresidian`：节点项目唯一需要直接引入的运行时入口；ABI、编解码和宏均为必选能力。
- `miresidian_abi`：稳定 C ABI 契约，供有直接 FFI 需求的项目使用。
- `miresidian_codec`：跨语言数据编解码与值模型，可独立使用。
- `miresidian_macro`：节点声明和入口生成宏；由 `miresidian` 无条件重导出，节点项目通常不直接依赖。
- `miresidian_pack`、`miresidian_testflow_windows`：独立的 Cargo 子命令工具，不进入节点 DLL 的运行时依赖图。

当前不设置 Cargo feature：运行时三个基础 crate 均不可选，开发工具则应独立安装。未来出现真正可选的运行时能力时，再以 `full` 聚合全部可选功能。

## 属性宏与节点扩展

`macro/` 是声明式节点扩展的唯一代码生成边界。驱动 crate 只描述节点状态、通信实现和
带标注的方法；不得手写平台函数列表、字符串分发、`DeviceNode` 实现或 C ABI 导出。宏项目
与运行时、实例节点遵循同一套模块规则：入口文件只做路由，同一概念拥有多个协作职责时使用
`概念.rs + 概念/`。

```text
macro/src/
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
`独立节点项目/` 中创建独立 crate。宏标注、自动命名、固定入口和可使用的数据类型均以
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
串口通信节点的配置与操作说明见 [../miresidian_serial/docs/串口通信.md](../miresidian_serial/docs/串口通信.md)。
C DLL 调用约定和 Windows DLL 构建说明见 [docs/c-abi.md](docs/c-abi.md)。
构建容器与 `cc` 链接器的安装说明见 [docs/build-environment.md](docs/build-environment.md)。
框架、实例项目与 ABI 的版本、修订记录和发布流程见 [docs/versioning.md](docs/versioning.md)。
