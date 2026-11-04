# Node 开发参考

实现或修改节点前读取，局部维护只执行相关要求。适配版本见 [主文档](../SKILL.md#适配版本)。

## 项目与依赖

- 独立 crate 声明 `cdylib`，一个 crate 声明一个具体节点；不靠多份条件节点声明切换实现。
- 从 crates.io 精确依赖已发布框架，由 Cargo 生成并维护真实 Cargo.lock；禁止框架 path/git 依赖、patch/replace/paths 等本地覆盖或伪造锁文件。基础依赖写法：`miresidian = { version = "=0.20.1" }`，已有项目保留原精确版本，升级须属于本次任务。
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

    #[miresidian::operation]
    fn get_port_no(
        #[miresidian::input("设备标识")] device_id: String,
    ) -> miresidian::MiresidianResult<Option<String>> {
        crate::device::Device::discover(&device_id)
    }
}
```

## 固定入口与节点职责

以下约定适用于主文档列明的版本。下列节点方法均标注 `#[miresidian::operation]`，固定映射为对应平台名称；方法体遵守 node.rs 的委托边界。

### InitDevice / init_device（必需）

- 无 self 的关联函数，参数默认来自 Config；初始化专用输入可用 `#[miresidian::input]` 声明。返回 `MiresidianResult<Self>` 或 `MiresidianResult<(Self, T)>`，Self 由框架持有，仅 T 编码为业务输出；无 T 时输出为空。
- 校验业务配置，按业务需要建立连接、会话或任务，并由业务对象持有成功取得的资源。框架管理通道、实例复用及声明的租约，节点不自行登记或管理其他实例。
- 返回实例前失败时，通过所有权、Drop 或显式回滚释放已取得的资源；此时框架没有实例可供关闭。返回实例后若输出编码或租约校验失败，框架负责调用关闭并析构节点。
- 复用依据规范化业务 Config 及所选实例策略；命中已有对象时直接返回复用提示，不再执行构造函数。初始化 Input 不参与对象身份，改变它不会触发重建；需重新应用时显式关闭后初始化，或设计独立业务操作。
- 复用不累计初始化引用；业务对象与物理资源代理分别管理。需要不同对象共用独占连接时按 [资源共享接入](lease.md#共享连接接入) 实现通信层，不能在每个对象中重复 open。
- 默认单实例更换 Config 时先预解析参数：解析失败保留旧对象；通过后先关闭旧对象，再构造新对象。关闭失败阻止新构造；旧对象释放后构造失败保持未初始化，不恢复旧连接。多实例关联见 [instances](instances.md)。

### CloseDevice / close_device（必需）

- 使用 `&mut self`，返回 `MiresidianResult<()>`，平台输出为空；通常无需参数，未标注参数默认来自 Input。框架内部清理会传空 Input，不能依赖调用方额外提供必填关闭输入。
- 停止当前对象的任务，释放连接、句柄及其他资源；支持连接已断开或业务部分失败后的清理，某项失败仍尽力释放其余资源，必要时由 Drop 兜底。
- 框架随后析构对象、移除对应通道并释放租约，即使关闭返回错误也不会保留该对象继续使用；节点不得关闭其他实例。对象关联和全局关闭由框架处理。
- 重复初始化复用的对象仍由一次 CloseDevice 关闭。业务对象持有共享代理时只释放自己的代理，底层连接由资源所有者在最后一个代理退出时关闭。

### GetPortNO / get_port_no（可选）

- 用于初始化前按业务条件发现设备，返回后续连接所需的地址或资源标识，如 COM7；字符串格式由节点约定。无 self，无需先调用 InitDevice，参数默认来自 Input，可声明筛选条件、探测配置及超时。
- 返回 `MiresidianResult<Option<String>>`：`Ok(Some(标识))` 对应平台 output `["1", "标识"]`；`Ok(None)` 对应 `["0", ""]`，表示未找到；探测执行失败返回 Err，不伪装成未找到。未实现时框架默认返回未找到。
- 探测不登记节点实例；自行释放临时连接和任务，不关闭既有实例。明确匹配规则、候选选择和超时边界；不具备自动发现能力时可省略，不以配置值或虚构地址冒充发现结果。

其余固定入口由框架提供，无需编写同名节点方法。GetFunctions、GetInitConfig、两个版本查询及 GetPortNO 均可在初始化前调用：

| 入口 | 作用及节点接入 |
| --- | --- |
| GetFunctions | 返回固定入口和业务操作的签名；通过宏标注维护参数与输出元数据。 |
| GetInitConfig | 返回初始化配置定义及所选能力追加的字段；调用方据此组织 Config，顺序见下文。 |
| GetFrameworkVersion | 返回 DLL 使用的框架版本；来源为框架编译期版本。 |
| GetProjectVersion | 返回节点 crate 的 CARGO_PKG_VERSION；维护节点自身 Cargo.toml 版本。 |
| CloseAllDevices | 无需先初始化，不接受 Config/Input；幂等地关闭并释放当前运行时全部通道，逐个调用节点关闭方法。 |

## 宏与参数契约

- 使用 `node`、`operation` 及参数/输出标注。普通业务方法使用 `&mut self`；名称可显式声明，或由 snake_case 转为 PascalCase。
- `node` 与 `operation` 宏生成 DeviceNode 实现、操作分发和 DLL 导出，节点只声明业务契约并委托实现。
- Config 描述初始化/对象关联配置，Input 描述本次业务输入，Output 描述结果。初始化默认取 Config，其他操作默认取 Input，可用标注明确覆盖来源；两个通道各按其参数声明顺序独立索引。输入可传裸值或 `名称:值`，须确定类型、单位、默认值、范围和错误。
- 描述保留原默认文本。当前值与 `9600/19200` 等完整候选文本匹配时处理首项实际值；显式 `19200` 仍是 19200，其他输入独立解析或报错，不回退首项。未声明运行时默认值时不因候选首项补缺。
- 存在业务 Config 时，Config[0] 必须是真实对象主键，如端口或资源地址；无业务配置的最小单实例允许空 Config，不伪造主键。实例关联见 [instances](instances.md)，附加框架字段见 [Config 布局](#config-布局)。

### 完整选项与枚举

- 有限离散取值的 Config/Input 必须在宏描述中列出全部实际支持选项，声明的默认值须对应集合内实际值；单一合法值也须声明选项。文档取值表或仅写默认值不能替代宏声明；连续范围、开放地址和文本按真实规则校验。
- 选项以 `/` 分隔，`-` 分隔实际值与显示值，例如 `校验:None-无校验/Odd-奇校验/Even-偶校验`。实际值和显示值须无歧义，不能自行嵌入分隔符；上例须按真实支持集合替换。
- Rust 内部优先用业务枚举；实现下述 DeviceInput 后可直接用于参数签名，类型、解析和校验归业务模块。使用 String/基础类型在业务边界转换须有接口复用等理由。
- 选项、枚举、别名、默认值、文档和实际支持范围保持一致；未知值须在资源操作或副作用前失败，不静默回退。验证每个规范值及至少一个集合外值，不展开无关选项的笛卡尔积。
- UTF-8 格式接受 `UTF-8`、`UTF8`、`UTF_8`，宏选项可统一写 `UTF8`；普通字符编码说明写 UTF-8。

### 基础类型与输出

复用框架编解码；类型转换和业务允许范围分别校验，不手拼平台响应 JSON。

- String 保留非空裸文本；整数支持十进制、`0x` 和 `0b`，浮点使用小数点 `.` 并拒绝 NaN/无穷值；bool 接受 true/false、1/0、是/否、真/假，输出为小写 true/false。
- 声明默认值后，缺失或空白参数先采用默认值。无默认值的 `Option<T>` 缺参仍报错，显式传空字符串才得到 None；如需允许省略，可声明空默认值 `#[miresidian::input("备注:")] note: Option<String>`。输出 None 占一个空字符串槽位。
- 普通一维 Vec 占一个参数/输出槽位，元素用竖线分隔；内容中的竖线和反斜杠分别转义为 `\|`、`\\`。整型数组支持递增范围；Vec<u8> 可输入整数数组或 `base64:...`，输出固定为 Base64。Vec 是一次业务的数据集合，逐条重复调用见 [batch](batch.md)。
- 标量和标准 Vec 返回一个输出，`()` 无输出，受支持的平坦元组按元素顺序输出；output 标注的名称数量须与槽位一致。未标注时单项名为“返回值”，多项依次为“返回值1”等；有副作用且无业务结果时返回 `MiresidianResult<()>`。
- 自定义类型按用途实现 `miresidian::DeviceInput::parse_device_text(value, field)` 或 `DeviceOutput::into_device_output(self)`；作为 Vec 元素还须实现 DeviceCollectionElement 的集合解析/格式化。未知枚举值返回 `MiresidianError::InvalidArgument { field, message }`，不静默选择默认项；实现放业务模块。

## Config 布局

组合所需能力，GetInitConfig 是最终参数顺序的依据：

业务 Config → Handle（handle 关联）→ 资源访问方式（selectable 租约）→ 平台适配（普通入口）→ 执行方式（selectable 执行）。

显式 `invoke_with` 移除平台适配项；框架字段在调用业务前剥离，不写入初始化签名。新增能力后同步调用说明和流程；instances + lease 验证对象/资源关联及关闭隔离。

## 业务与生命周期

- 按资料确定帧边界、长度、校验、编码和超时；业务边界解析并验证配置，后续使用已验证值。协议限制及型号差异注明资料版本/章节，未证实推断明确标识。
- 资源创建与清理遵守上述固定入口约定。完整请求—响应需保持事务互斥，实例锁不能代替跨资源或跨进程保护。
- 预期失败用框架提供的 `MiresidianResult` 错误 API，带操作/资源上下文；不用 panic，不将失败包装为成功默认值。有副作用的操作须核实幂等性后才考虑重试。业务 panic 或租约失效后对象仅可清理，关闭后重新初始化。

### 异步与取消（按需）

- operation 支持 `async fn`，无需额外 feature；普通业务生成原生 Future 桥，Future 必须为 Send。初始化、关闭和探测也可为 async，由生命周期工作线程等待完成。契约名称和参数编解码与同步方法一致。
- 同一实例的独占访问覆盖整个方法及 await，不能同步重入同一实例。使用真正的异步 I/O；同步 I/O 仍需自身超时。
- 长操作通过 `miresidian::CallContext::current()` 获取上下文，在循环/I/O 边界调用 `check()?`，将 `remaining()` 用于 I/O 截止时间；异步可等待 `cancelled().await` 后清理并返回取消错误。取消是协作请求，不强杀同步线程，也不丢弃执行中的业务 Future；未响应取消的方法仍可能成功完成，不据此提前释放资源。

## 调用说明与 ABI

`docs/<节点显示名>.md` 说明 DLL/位数、配置与选项、方法输入/输出、单位/默认值、错误、调用顺序和可执行示例。同步源码注释、Rustdoc 与实际注册元数据；注释解释协议约束、资源归属和特殊处理原因。

ABI v7 使用框架生成的 `miresidian_abi_version`、`miresidian_invoke`、`miresidian_invoke_with`，正式签名以对应版本的 miresidian.h 为准。Config/Input 是 UTF-8 JSON 字符串数组，如 `["COM3", "9600"]`；业务结果位于响应的 output 字符串数组，按 status 和稳定 code 判断失败，不解析 message。

返回文本由 DLL 当前线程持有，同线程下一次调用前立即复制，调用方不释放指针。调用说明须明确此生命周期；语法片段、模拟示例与真实资源结果分别标识。

## 验证与审查

单元测试就近放业务模块，可内联；影响阅读时再外置私有测试子模块。公开接口集成测试放 tests/，不为测试公开内部实现。夹具按实际复用范围组织，断言业务结果、错误类别、释放或副作用。

| 改动 | 必要证据 |
| --- | --- |
| 契约/配置 | 实际注册元数据的操作、顺序、默认值、完整选项及输出；枚举/显示值映射、缺失/空值、集合/元组槽位及非法输入拒绝 |
| 业务/协议 | 正常结果、相关边界与错误样本；缺陷修复保留回归，条件允许时证明修复前失败、后通过 |
| 生命周期 | 初始化 → 业务 → 关闭；同配置复用、初始化 Input 不触发重建、替换失败、构造回滚、空 Input 关闭及故障后清理；探测验证初始化前的找到/未找到/失败与临时资源回收 |
| 异步/取消（使用时） | await 期间同实例互斥，截止时间/取消处理、完成确认及资源清理，不将取消请求当作已停止 |
| 结构/依赖 | 枚举 node.rs 手写项并沿改动调用链审查上述边界；核对复用依据、依赖方向、资源归属和拆分收益 |
| 可选能力 | 对应参考中的所选策略验收，实际交付 features 与检查配置一致 |
| DLL 交付 | 指定位数实际加载、版本/元数据及业务调用；manifest 的版本、架构和 SHA-256 对应本次产物 |
| 仅文档/skill | 内容、示例、引用、格式和适配版本，不运行无关编译或硬件测试 |

执行适用的结构、格式、静态检查、测试和已有钩子/CI 门禁，修复本次相关违规。重组保留有效断言，不删除或弱化测试换取通过；结构检查、编译、打包各只证明自身范围。

核对文档、注释、宏契约与示例一致，以代码和实际结果简述审查依据，无需额外报告文件。无硬件结果不能外推真实通信；未完成项明确报告，无关历史问题单独记录。

版本核对来源：[MyMires/miresidian](https://github.com/MyMires/miresidian) 的对应版本文档与测试、[docs.rs API](https://docs.rs/miresidian) 的精确版本；不默认 HEAD/latest 与节点一致。
