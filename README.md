# Miresidian

Miresidian 是面向外部资源集成、通信与业务调用的 Rust **节点插件开发与运行时框架**。它将集成逻辑和业务能力构建为具有统一 C ABI 的节点 DLL，供 C、C#、Python、LabVIEW 等语言的宿主应用加载和调用；测试平台、自动化系统与上位机软件都是其典型接入方。

Miresidian 本身不是插件宿主；独立节点项目产出的 DLL 才是宿主应用中的节点插件或扩展。框架负责统一其调用契约、生命周期、参数编解码、接入协议适配和并发执行，使节点开发者能够专注于资源状态、通信和业务逻辑。最低 Rust 版本为 **1.89**；当前框架源码版本为 **0.19.2**，C ABI 为 **v7**，租约协议为 **v2**。

**[文档中心](docs/README.md)** · **[架构总览与 UML](docs/architecture.md)** · **[修订记录](CHANGELOG.md)**

## 核心能力与优势

- **统一插件契约**：以 `Function + Config + Input -> Output` 调用模型和稳定 C ABI 对接多种语言与宿主应用，支持节点发现、版本查询、端口探测、初始化与关闭。
- **专注节点业务**：属性宏生成节点元数据、强类型参数转换、操作分发和 DLL 导出；节点作者只需实现资源状态、通信和业务方法。
- **安全管理节点状态**：不同实例请求可并行，同一实例请求及共享批次保持互斥；支持有界任务、截止时间、协作取消和原生 async 方法。完整规则见[并行与异步执行](docs/architecture.md#并行与异步执行)。
- Broadcast 固定执行通道1一次并将完整结果复制到有效通道；其他输入无需同值，空通道跳过。规则与例子见[广播规范](docs/instances.md#broadcast-的执行源与输出规则0160)。
- **按需扩展运行方式**：默认单实例；可用 instances feature 与同名宏配置选择多实例的 Config[0]/Handle 关联，以及顺序、并行或广播多通道执行。空主键初始化跳过，并在对象边界隔离可展开 panic。
- **统一数据与接入边界**：提供独立编解码和值模型，支持基础类型、一维数组、文本编码、HEX 与 Base64；平台适配器将宿主协议与节点业务解耦。
- **支持受控的共享资源**：需要跨进程协调资源配置的节点可启用 `lease` feature。
- **显式格式转换**：启用 `format` 并标注 Input/Output，框架按所选字符编码、HEX 或 Base64 自动转换；需要批量处理时可另加 batch，详见 [format feature](features/format/README.md)。基础类型解析不需要启用 format。
- 对单值业务逻辑需要顺序批量调用时，可启用 `batch` feature，并在具体数值 Input 参数上标记 batch，整数保留范围展开，浮点使用逐项列表；实现位于 [batch feature crate](features/batch/README.md)。

节点项目通常只直接依赖 `miresidian`。具体串口、TCP、USB、CAN 等通信实现和私有依赖归独立节点项目所有。框架库、过程宏、开发工具与节点 DLL 的关系见[架构总览](docs/architecture.md)。

统一开发命令与整套下载见 [Miresidian CLI](tools/miresidian-cli/README.md)。CLI/套件独立版本为 0.2.1，用户安装与升级由 Cargo 管理。

## 从这里开始

| 你的目标 | 阅读入口 |
| --- | --- |
| 理清项目分层、类型关系和源码位置 | [架构总览：依赖图、UML 类图与调用时序](docs/architecture.md) |
| 配置单实例、多实例及多通道执行 | [实例管理](docs/instances.md) |
| 开发一个节点插件 | [节点开发与数据编解码](docs/宏驱动开发.md) |
| 从 C、C#、Python 或 LabVIEW 调用 DLL | [平台接入与 C ABI](docs/c-abi.md) |
| 接入新的测试平台 | [平台适配器](docs/c-abi.md#平台适配器) |
| 为节点启用跨进程资源租约 | [租约接入与协议](features/lease/README.md) |
| 构建、测试或发布 | [构建环境](docs/coding-standards.md#构建环境) → [打包工具](tools/miresidian-pack/README.md) → [版本管理](docs/versioning.md) |

完整主题索引、开发工具和维护规则统一收录于[文档中心](docs/README.md)。

## 最小节点示例

在独立节点项目中声明依赖，并将节点 crate 构建为 `cdylib`：

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
miresidian = "=0.19.2"
```

```rust
use miresidian::MiresidianResult;

#[miresidian::node("回显节点")]
struct EchoNode;

impl EchoNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self)
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("回显")]
    #[miresidian::output("接收文本")]
    fn echo(&mut self, #[miresidian::input("发送文本")] text: String) -> MiresidianResult<String> {
        Ok(text)
    }
}
```

宏自动生成 `miresidian_invoke`、`miresidian_invoke_with` 与 `miresidian_abi_version`。调用参数使用 UTF-8 JSON 字符串数组；响应固定包含 `status`、`code`、`message`、`output`。DLL 持有返回文本，调用方须立即复制且不得释放指针。完整签名与有效期以 [C ABI](docs/c-abi.md) 为准。

子节点仅依赖 crates.io 已发布框架的精确版本，禁止本地路径和本地覆盖，要求见[节点依赖规则](docs/versioning.md#子节点只使用已发布框架)。

## 参与开发

版本管理是 **P0 提交门禁**。首次检出先检查已有钩子配置，再执行 `git config core.hooksPath .githooks`；存在自定义钩子时须先整合。相关更新必须同步版本、精确依赖、锁文件、带日期的修订与标签，详见[版本管理规范](docs/versioning.md)。

本地工具链、完整验证命令以及代码、文档与测试要求统一见[开发与维护规范](docs/coding-standards.md)。

0.19.2 为当前源码候选；发布前外部项目继续使用已发布精确版本，不允许本地覆盖。
