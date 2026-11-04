# Node 开发参考

实现或修改节点前读取，局部维护只执行相关要求。版本基线见 [主文档](../SKILL.md#适配版本)。

## 项目与依赖

- 独立 crate 声明 `cdylib`，一个 crate 声明一个具体节点；不靠多份条件节点声明切换实现。
- 从 crates.io 精确依赖已发布框架，由 Cargo 生成并维护真实 Cargo.lock；禁止框架 path/git 依赖、patch/replace/paths 等本地覆盖或伪造锁文件。基础依赖写法：`miresidian = { version = "=0.19.4" }`，已有项目保留原精确版本，升级须属于本次任务。
- 实现前核对框架、标准库、已有依赖和成熟实现。通用通信、编解码及校验优先复用；自行实现或额外封装须有实际缺口。新依赖核实真实 API、维护状态、许可证、MSRV 和目标平台。

## 能力依赖

框架 default 为空，基本宏、运行时、ABI 和通用编解码无需额外 feature。串口/TCP 依赖由节点选择，不猜测同名 feature。

将所需能力固定在框架依赖的同一 features 数组中。确需节点自身可选 feature 时显式转发，例如 `instances = ["miresidian/instances"]`，并保证交付默认配置有效；空 `instances = []` 不启用依赖能力，`cfg(feature = "...")` 判断当前 crate。

Cargo features 按依赖图加法合并，`default-features = false` 不撤销其他路径的启用项；不以临时检查或 `--all-features` 代替交付配置。各能力示例版本见对应参考，不因示例升级或降级；检查方式见 [工具命令](commands.md#feature-检查)。

## 文件职责

以下为基础布局，已有等价命名可保留；协议、传输、测试和脚本按实际需要创建。

```text
<node-project>/
├── Cargo.toml / Cargo.lock
├── AGENTS.md / README.md / CHANGELOG.md
├── src/
│   ├── lib.rs             # 模块声明与必要导出
│   ├── node.rs            # 集中声明契约并委托业务
│   ├── device.rs          # 业务对象；按领域命名
│   ├── protocol.rs        # 按需：协议编解码
│   └── transport.rs       # 按需：连接与字节 I/O
├── tests/                 # 按需：公开接口集成测试
├── docs/<节点显示名>.md   # 调用说明
└── tools/smoke.testflow   # 按需：真实配置与操作
```

- 依赖方向为节点 → 业务对象 → 所需协议/传输；下层不依赖具体节点、平台索引或 ABI 文本。连接、状态和缓存各有明确管理者，不重复维护；生产代码不依赖测试夹具。
- 新模块须有独立职责和真实调用方。少量紧密相关的协议/I/O 可归业务模块；有实际会话管理职责才提取 session，不预建空模块、通用框架或纯转发层。
- 类型、常量、错误和配置解析归所属业务模块，确有共享者再提取；默认私有，不为测试扩大 API。引入 trait 须有实际替换或测试边界。
- 单个概念用 `概念.rs`，多个协作子职责用 `概念.rs + 概念/`，父文件收敛声明和导出，不用 mod.rs。约 300–500 行时审查职责与阅读负担，按实际边界拆分，不设机械行数上限。
- 项目 AGENTS.md 记录实际模块边界、测试约定和有效命令；README 指向构建与调用入口。target/dist 是生成目录，旧产物不能充当本次证据。

## node.rs 边界

node.rs（或已有等价文件）集中展示宏标记的节点结构体、契约方法及 Config/Input/Output，只附带必需的 use、impl、对象持有字段和契约注释。方法体仅委托外部业务对象、构造节点包装或直接映射参数/结果。

业务状态检查、参数校验、协议、计算、I/O、重试和资源清理归业务模块。契约文件禁止手写辅助函数/方法、辅助类型、常量、测试实现或测试模块声明；不能通过额外 impl、闭包、内嵌模块或本地宏隐藏业务子逻辑。短小、私有或单一调用方均不构成例外。

签名与委托示意，`crate::device::Device` 的真实实现另置业务模块：

```rust
#[miresidian::node("示例节点")]
struct ExampleNode {
    device: crate::device::Device,
}

impl ExampleNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("连接地址")] address: String,
        #[miresidian::config("超时/ms:1000")] timeout_ms: u64,
    ) -> miresidian::MiresidianResult<Self> {
        crate::device::Device::connect(address, timeout_ms).map(|device| Self { device })
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> {
        self.device.close()
    }
}
```

## 宏与参数契约

- 使用 `node`、`operation` 及参数/输出标注。初始化无 self，返回 `MiresidianResult<Self>` 或 `MiresidianResult<(Self, T)>`，后者只有 T 是业务输出；关闭和普通方法使用 `&mut self`。可选 `get_port_no` 无 self，探测资源自行回收。
- 不添加旧 `node_impl`，不手写 DeviceNode、操作字符串分发、GetFunctions、GetInitConfig 或 DLL 导出。版本查询与 CloseAllDevices 也由框架提供；普通方法名称可显式声明，或由 snake_case 转为 PascalCase。
- Config 描述初始化/对象关联配置，Input 描述本次业务输入，Output 描述结果。初始化默认取 Config，其他操作默认取 Input，可用标注明确覆盖来源；参数顺序、类型、单位、默认值、范围和错误须确定。描述保留原默认文本。0.19.4 起，采用 `9600/19200` 等完整候选文本时处理首项实际值；显式 `19200` 仍是 19200，其他输入独立解析或报错，不回退首项。未声明运行时默认值时不因候选首项补缺；旧版按其解析行为显式传值。
- 存在业务 Config 时，Config[0] 必须是真实对象主键，如端口或资源地址；无业务配置的最小单实例允许空 Config，不伪造主键。实例关联见 [instances](instances.md)，附加框架字段见 [Config 布局](#config-布局)。
- 类型使用所选版本支持的 String、数值、bool、Option、基础类型 Vec 等；输出标注与实际结果对应，复用框架编解码，不手拼平台响应 JSON。

### 完整选项与枚举

- 有限离散取值的 Config/Input 必须在宏描述中列出全部实际支持选项，声明的默认值须对应集合内实际值；单一合法值也须声明选项。文档取值表或仅写默认值不能替代宏声明；连续范围、开放地址和文本按真实规则校验。
- 选项以 `/` 分隔，`-` 分隔实际值与显示值，例如 `校验:None-无校验/Odd-奇校验/Even-偶校验`。实际值和显示值须无歧义，不能自行嵌入分隔符；上例须按真实支持集合替换。
- Rust 内部优先用业务枚举；类型、解析和校验均归业务模块。先核实框架是否支持该类型直接作为契约参数，不支持时用 String/基础类型在业务边界转换。保留基础类型须有接口复用等理由。
- 选项、枚举、别名、默认值、文档和实际支持范围保持一致；未知值须在资源操作或副作用前失败，不静默回退。验证每个规范值及至少一个集合外值，不展开无关选项的笛卡尔积。
- 格式选项可用通用名称 `UTF8`；`UTF_8` 需 codec 0.14.1 及对应框架支持。普通字符编码说明仍写 UTF-8。

## Config 布局

组合所需能力，GetInitConfig 是最终参数顺序的依据：

业务 Config → Handle（handle 关联）→ 资源访问方式（selectable 租约）→ 平台适配（普通入口）→ 执行方式（selectable 执行）。

显式 `invoke_with` 移除平台适配项；框架字段在调用业务前剥离，不写入初始化签名。新增能力后同步调用说明和流程；instances + lease 验证对象/资源关联及关闭隔离。

## 业务与生命周期

- 按资料确定帧边界、长度、校验、编码和超时；业务边界解析并验证配置，后续使用已验证值。协议限制及型号差异注明资料版本/章节，未证实推断明确标识。
- 初始化取得资源，部分失败释放局部连接与后台任务；成功后由业务对象持有。完整请求—响应需保持事务互斥，实例锁不能代替跨资源或跨进程保护。
- 预期失败用所选版本真实提供的 `MiresidianResult` 错误 API，带操作/资源上下文；不用 panic，不将失败包装为成功默认值。有副作用的操作须核实幂等性后才考虑重试。
- 关闭先停止任务并释放连接，失败也尽力清理其余资源，必要时由 Drop 兜底；关闭后不得继续使用遗留句柄。

## 调用说明与 ABI

`docs/<节点显示名>.md` 说明 DLL/位数、配置与选项、方法输入/输出、单位/默认值、错误、调用顺序和可执行示例。同步源码注释、Rustdoc 与实际注册元数据；注释解释协议约束、资源归属和特殊处理原因。

ABI v7 使用框架生成的 `miresidian_abi_version`、`miresidian_invoke`、`miresidian_invoke_with`，正式签名以依赖版本的 miresidian.h 为准。Config/Input 是 UTF-8 JSON 字符串数组，如 `["COM3", "9600"]`；业务结果位于响应的 output 字符串数组，按 status 和版本支持的稳定 code 判断失败，不解析 message。

返回文本由 DLL 当前线程持有，同线程下一次调用前立即复制，调用方不释放指针。调用说明须明确此生命周期；语法片段、模拟示例与真实资源结果分别标识。

## 验证与审查

单元测试就近放业务模块，可内联；影响阅读时再外置私有测试子模块。公开接口集成测试放 tests/，不为测试公开内部实现。夹具按实际复用范围组织，断言业务结果、错误类别、释放或副作用。

| 改动 | 必要证据 |
| --- | --- |
| 契约/配置 | 实际注册元数据的操作、顺序、默认值、完整选项及输出；枚举/显示值映射和非法输入拒绝 |
| 业务/协议 | 正常结果、相关边界与错误样本；缺陷修复保留回归，条件允许时证明修复前失败、后通过 |
| 生命周期 | 初始化 → 业务 → 关闭，部分失败清理、关闭释放及相关再次使用行为 |
| 结构/依赖 | 枚举 node.rs 手写项并沿改动调用链审查上述边界；核对复用依据、依赖方向、资源归属和拆分收益 |
| 可选能力 | 对应参考中的所选策略验收，实际交付 features 与检查配置一致 |
| DLL 交付 | 指定位数实际加载、版本/元数据及业务调用；manifest 的版本、架构和 SHA-256 对应本次产物 |
| 仅文档/skill | 内容、示例、引用、格式和适配版本，不运行无关编译或硬件测试 |

执行适用的结构、格式、静态检查、测试和已有钩子/CI 门禁，修复本次相关违规。重组保留有效断言，不删除或弱化测试换取通过；结构检查、编译、打包各只证明自身范围。

核对文档、注释、宏契约与示例一致，以代码和实际结果简述审查依据，无需额外报告文件。无硬件结果不能外推真实通信；未完成项明确报告，无关历史问题单独记录。

版本核对来源：[MyMires/miresidian](https://github.com/MyMires/miresidian) 的对应版本文档与测试、[docs.rs API](https://docs.rs/miresidian) 的精确版本；不默认 HEAD/latest 与节点一致。
