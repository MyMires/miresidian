# Miresidian

[中文](README.md) · [English](README.en.md)

Miresidian 是面向外部资源集成、通信与业务调用的 Rust **节点插件开发与运行时框架**。它将集成逻辑和业务能力构建为具有统一 C ABI 的节点 DLL，供 C、C#、Python、LabVIEW 等语言的宿主应用加载和调用；测试平台、自动化系统与上位机软件都是其典型接入方。

Miresidian 本身不是插件宿主；独立节点项目产出的 DLL 才是宿主应用中的节点插件或扩展。框架负责统一其调用契约、生命周期、参数编解码、接入协议适配和并发执行，使节点开发者能够专注于资源状态、通信和业务逻辑。最低 Rust 版本为 **1.89**；当前框架源码版本为 **0.34.3**，C ABI 为 **v10**，租约协议为 **v2**。

**[0.34.0 宏简写与 encoding 迁移](docs/releases/0.34.0.md)** · **[0.33.0 instance 名称迁移](docs/releases/0.33.0.md)** · **[0.30.0 枚举与单 JSON 入口](docs/releases/0.30.0.md)** · **[0.29.0 公开类型命名迁移](docs/releases/0.29.0.md)** · **[0.28.0 宏简写与兼容迁移](docs/releases/0.28.0.md)** · **[文档中心](docs/README.md)** · **[架构总览与 UML](docs/architecture.md)** · **[修订记录](CHANGELOG.md)** · **[运行时平台选择与 v9 迁移](docs/releases/0.27.0.md)**

## 核心能力与优势

- **统一插件契约**：以 `Function + Config + Input -> Output` 调用模型和稳定 C ABI 对接多种语言与宿主应用，支持节点发现、版本查询、端口探测、初始化与关闭。
- **专注节点业务**：属性宏生成节点元数据、强类型参数转换、操作分发和 DLL 导出；节点作者只需实现资源状态、通信和业务方法。
- **安全管理节点状态**：不同实例请求可并行，同一实例请求及共享批次保持互斥；支持有界任务、截止时间、协作取消和原生 async 方法。完整规则见[并行与异步执行](docs/architecture.md#并行与异步执行)。
- Broadcast 固定执行通道1一次并将完整结果复制到有效通道；其他输入无需同值，空通道跳过。规则与例子见[广播规范](docs/instances.md#broadcast-的执行源与输出规则0160)。
- **按需扩展运行方式**：默认单实例；可用 instance feature 与同名宏配置选择多实例的 Config 组合键/Handle 关联，以及顺序、并行或广播多通道执行。空主键初始化跳过，并在对象边界隔离可展开 panic。
- **统一数据与接入边界**：提供独立编解码和值模型，支持基础类型、一维数组、文本编码、HEX 与 Base64；平台适配器将宿主协议与节点业务解耦。
- **支持受控的共享资源**：启用 `lease` feature 并保留现有 `lease(...)` 标注，Lease 模式自动跨 DLL/进程共用一个节点资源对象：首次初始化打开、后续调用转发、最后引用关闭并析构；完整业务调用互斥。节点状态也共用，完整初始化参数与节点契约必须一致。独立业务对象共用通信连接时使用 [SharedResource 代理](docs/shared-resources.md)。
- **显式格式转换**：启用 `encoding` 并标注 Input/Output，框架按所选字符编码、HEX 或 Base64 自动转换；需要批量处理时可另加 batch，详见 [encoding feature](features/encoding/README.md)。基础类型解析不需要启用 encoding。
- 对单值业务逻辑需要顺序批量调用时，可启用 `batch` feature，并在普通 Input 参数上标记 batch，按原类型解析字符串、布尔、数值等记录；整数保留范围与混合展开，格式可逐条选择；实现位于 [batch feature crate](features/batch/README.md)。

节点项目通常只直接依赖 `miresidian`。具体串口、TCP、USB、CAN 等通信实现和私有依赖归独立节点项目所有。框架库、过程宏、开发工具与节点 DLL 的关系见[架构总览](docs/architecture.md)。

本次0.34.3仅修正TestFlow相关指导内容，运行时实现、宏和ABI契约保持。TestFlow 0.14系列支持通道映射、逐通道Config和内联断言，最终通过CloseAllDevices释放本地引用；使用方式及Windows验收边界见[工具说明](tools/miresidian-testflow-windows/README.md)。

统一开发命令与整套下载见 [Miresidian CLI](tools/miresidian-cli/README.md)。CLI/套件源码版本为 0.3.10，用户安装与升级由 Cargo 管理。Pack 的 new/init 模板支持 ABI v7–v10，按实际已发布依赖生成对应调用说明；新版本发行与安装状态分别核对，见工具说明。

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

以下依赖采用先前已核实发行的 **0.26.0**。其 registry 与精确 docs.rs 验收已在 [2026-11-06 发布记录](plans/2026-11-06-unified-abi-request.md#发布执行结果)归档，2026-11-07 再次确认可用。本检出的 **0.34.2** 是尚未发布的宏简写与 encoding 命名迁移源码候选，不把候选写成可直接安装依赖；后续采用其他版本前仍须核实各自发行记录。

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
miresidian = "=0.26.0"
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

本源码 0.34.2 / ABI v10 仅生成 `miresidian_invoke(const char *request_json)` 业务调用入口和 `miresidian_abi_version()` 版本查询。请求如 `{"platform":1,"function":"GetFunctions"}`；platform 非零指定已注册平台，0/省略使用 DLL 固定默认平台。Config 不发布也不接受“平台适配”选择。先核对 ABI 10 再使用单参数签名，不能把上面已发布 0.26.0/v8 的调用方式与本候选混用。响应和指针复制要求见 [C ABI](docs/c-abi.md)。0.34.2 在生命周期锁内复核 CloseDevice/CloseAllDevices 的绑定平台；排队期间已切换平台的旧关闭请求返回 INVALID_ARGUMENT，保留新实例。调用参数无需迁移，Rust 管理接口 shutdown/shutdown_async 仍不限定平台。

子节点仅依赖 crates.io 已发布框架的精确版本，禁止本地路径和本地覆盖，要求见[节点依赖规则](docs/versioning.md#子节点只使用已发布框架)。

交付 Node DLL 时，每个节点对应架构目录顶层只放一个供用户调用的主 DLL，随附的第三方及辅助运行时 DLL 放入 `dependencies/`。调用说明标明入口文件名、位数及依赖用途；无依赖时无需空目录。完整布局与实际加载验收要求见[交付规范](docs/coding-standards.md#6-注释与文档)。

## 当前源码的 feature 简写

以下对应框架 **0.34.3 / macro 0.30.0 / encoding 0.5.1** 源码候选，尚未发布；不能将上面的旧已发布依赖示例与新语法混用。format 能力统一更名为 encoding，包括 Cargo feature、宏标签、公开模块及 crate；旧入口不保留别名。迁移见 [0.34.0](docs/releases/0.34.0.md)。

```rust,ignore
#[miresidian::node("设备", instance(multiple, selectable(sequential)), lease)]
#[miresidian::input("数据", batch, encoding(hex))]
#[miresidian::output("结果", encoding(utf8, hex, base64))]
// 显式指定选择器名称：
#[miresidian::input("数据", encoding("输入编码", utf8, hex))]
```

- 裸 instance/instance() 为单实例、固定顺序多通道；不写 instance 仍禁用多通道。multiple 默认 Config 关联，multiple(config) 显式同义，multiple(handle) 使用 Handle；可追加 parallel、broadcast 或 selectable(sequential)。旧平铺短写与完整长写在 instance 分组下继续支持。
- multiple()、重复模式/关联或 single 携带关联方式报错。默认 Config 不改变参数 instance(key/ignore) 的主键选择规则。同一对象的完整业务调用仍互斥，包括 async 等待期间；parallel 不承诺同对象业务重叠。
- 裸 lease 等价于 lease(required)，固定租约且不生成访问方式选择器；lease(disabled) 固定直连，lease(selectable(required)) 允许选择且默认租约。参数 lease(key/ignore) 保持。
- encoding(hex) 固定编码、不增加 Input；多编码第一项默认，缺省名称自动使用“参数显示名 + 格式”，忽略冒号后的描述。上例输出生成“结果格式”，选择器仍追加在方法 Input 末尾；输入选择器紧随对应数据。
- encoding("输入编码", utf8, hex) 显式命名。encoding(utf8, hex, selector = "输入编码") 和 encoding(formats = [utf8, hex], selector = "输入编码") 保留；多编码长写同样允许自动命名。名称冲突时编译报错，须显式使用不同名称，不自动编号。
- 单编码禁止选择器名称；名称空白、包含 : / |、重复编码、混用列表语法或重复命名均报错。batch 继续独立组合，记录对齐、编码字节契约及错误行为保持。
- Cargo features 仍须显式启用：instance、lease、encoding、batch。旧版本节点继续按其依赖版本使用原名；候选发布前不升级独立 Node 或使用本地覆盖。

完整转换规则见 [encoding](features/encoding/README.md)，实例规则见[实例管理](docs/instances.md)。

## 参与开发

0.34.1 修复版本门禁对依赖别名的遗漏：按真实 package 名核对框架依赖的精确版本及锁文件，覆盖开发/构建/target 声明和当前清单中的 workspace 继承；同时补齐 encoding/batch Rustdoc 旧名。宏、运行时和 ABI 保持。

0.32.0 修正 GetPortNO 原因输出：未实现方法时固定返回 `["0", "当前未实现端口发现能力"]`，None 返回 `["0", "未找到匹配设备"]`，找到时仍返回地址。Option<String> 签名保持；NotSupported 说明作为未找到原因返回，其他错误仍走错误响应。见[迁移说明](docs/releases/0.32.0.md)。本次源码候选未发布。

版本管理是 **P0 提交门禁**。首次检出先检查已有钩子配置，再执行 `git config core.hooksPath .githooks`；存在自定义钩子时须先整合。相关更新必须同步版本、精确依赖、锁文件、带日期的修订与标签，详见[版本管理规范](docs/versioning.md)。

本地工具链、完整验证命令以及代码、文档与测试要求统一见[开发与维护规范](docs/coding-standards.md)。

本检出对应框架 0.34.3、macro 0.30.0、codec 0.18.0、ABI 0.17.1/v10。`#[derive(miresidian::NodeEnum)]` 为无字段业务枚举生成通用 codec 和结构化候选；参数缺少非空描述时，由本次调用的平台适配器生成候选格式并配套规范化输入，已有手写描述保持。默认值、集合、配置身份和租约规则见[节点手册](docs/宏驱动开发.md)；旧调用入口/平台 Config 的迁移见[0.30.0](docs/releases/0.30.0.md)。

0.31.0 支持成员显示值的长写与简写，含义相同：

```rust
#[derive(miresidian::NodeEnum)]
enum State {
    #[node_enum(display = "空闲")]
    Idle,
    #[node_enum("运行中")]
    Running,
}
```

参数只声明 `#[miresidian::input("状态")]` 时，TB 自动发布 `状态:Idle-空闲/Running-运行中`。输入“空闲”经平台转换为 `Idle`，输出仍为成员名；未标注时显示值使用成员名。重复、空白或错误类型的标注编译报错，平台语法/歧义由对应适配器校验，详见[0.31.0 迁移](docs/releases/0.31.0.md)。本次仅为未发布源码候选，不作为可直接安装版本。

已发布版本可查 [0.26.0 API](https://docs.rs/miresidian/0.26.0/miresidian/) 与[该版本的 format API](https://docs.rs/miresidian/0.26.0/miresidian/format/index.html)。本次未发布的新版本文档须在本检出执行 `cargo doc -p miresidian --all-features --no-deps --locked` 查看；源码提交、本地标签、registry 发布与 docs.rs 可用性分别确认。配置租约 v2、资源代理 v2 和 MSRV 1.89 保持。

0.30.1 仅补齐原始元数据与平台渲染的 Rustdoc、通用状态描述及规范查阅入口；NodeEnum 和 ABI v10 行为保持，详见[资料修正](docs/releases/0.30.1.md)。

0.30.2 使用 lease 0.17.4，修复 Windows 命名管道并发启动的错误 5：竞争者仅在实际连接已有服务成功后复用，真正连接失败仍返回错误。已有失效会话不重连、不重放；协议无需迁移，详见[修复说明](docs/releases/0.30.2.md)。
