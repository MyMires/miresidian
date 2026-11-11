# 宏标签简化方案

日期：2026-11-11。

## 实施决议（2026-11-11，优先于下方历史候选）

用户在确认 lease、instances、format 及裸 lease 默认规则后询问“可以开始实现了吗”，按本次任务上下文授权开始已确认范围的实现、必要验证和仓库规则要求的提交/标签核验。下方早期“仅方案”“待定”等描述作为讨论记录保留，不覆盖本节；未确认候选仍不实施。

### 最终范围与精确契约

- instances：不声明时保留 Single/Config/Disabled；裸 instances 和 instances() 为 Single/Config/Fixed(Sequential)。位置参数按 single/multiple、config/handle、执行策略三类解析；省略模式默认 single，省略策略默认固定 sequential。multiple 必须显式指定 config/handle，single 不接受关联项。selectable(default) 恰好一个默认执行策略。原 NameValue 长写继续严格校验原有必填项，禁止在同一分组混合长短写；不新增由 config/handle 推断 multiple 的别名。
- lease：裸 lease 和 lease(required) 等价于旧 activation = required；lease(disabled) 与 lease(selectable(required)) 分别等价于旧 disabled/selectable。lease()、lease(selectable)、等号式 lease 及 selectable(disabled) 不加入本次语法。旧 resource/configuration 长写、成对要求及与参数角色的冲突规则保持。node 中同一 feature 的裸标记/长写/短写不得重复。
- format：固定单格式 format(hex)，多格式 format(utf8, hex, base64, selector = "数据格式")。六种既有格式均支持；单格式禁止 selector，多格式必须有 selector，第一格式默认。新短写位置项在前、selector 在后，旧长写具名项次序继续兼容；禁止一组内混用 formats = [...] 与位置格式项。batch 保持裸标记，可与新旧 format 交换外层选项次序，不改记录语法、槽位布局、执行和输出聚合。
- 不实施 output 并入 operation、operations 契约 impl、集中 inputs、新的租约默认 Direct 或任何运行时/ABI/协议修改。参数的 config/input/instance/lease 标记保持就地声明。

### 组件与文件范围

基线 HEAD 为 6c81bf0，根 0.27.0、macro 0.23.2。本次公开宏语法新增：macro 升至 0.24.0；根框架公开重导出并采用新宏，升至 0.28.0，内部依赖改为精确 =0.24.0。ABI 保持 crate 0.16.0 / v9，租约协议/资源代理保持 v2，其他运行库/工具不升版。

解析修改限定 macro/src/node/arguments.rs、node/instances.rs、operation/format.rs 及实际需要的内部辅助模块；保持现有 NodeArguments、LeaseDeclaration、InstancePolicy、Format 和生成桥接不变。node 入口辨别裸标记/分组；instances 先区分长短语法再归一；format 在既有 annotation 中归一新旧声明。

同步根及宏 Cargo.toml、根 Cargo.lock、两组件带日期 CHANGELOG、双语根 README、macro/README、宏 Rustdoc、根 API 说明、docs/宏驱动开发.md、docs/instances.md、docs/config-identity.md、docs/architecture.md 及新 docs/releases/0.28.0.md。独立 format/batch/instances/lease 运行库 README 中旧语法仍有效，本次不改这些组件发行内容；新宏语法集中在根/宏所属指导文档，避免无关组件升版。现有独立烟测夹具和相邻 Node 依赖已发布版本，不升级、不使用本地覆盖，其锁文件核对后保持原样。新版本文档明确为未发布源码，不把 0.28.0 写成可直接安装版本。

### 实施、验证与证据

1. 先修改版本和解析入口，为旧写法保留原诊断；新增声明归一到现有中间结构，无运行时新分支。
2. 单元测试覆盖完整策略矩阵、新旧展开等价、默认值、未知/重复/混用/缺项及格式 selector 错误；公开集成测试验证 descriptor、GetFunctions/GetInitConfig、真实 Runtime 调用、批量格式转换、async 与现有 ABI 入口。
3. 每个测试二进制只导出一个节点，避免多个 no_mangle C ABI 冲突；增加可复用 fixture 和独立测试目标，现有旧语法回归保留。
4. 编译失败测试覆盖裸/短写的 feature 门禁和角色要求，旧 trybuild 快照保持或仅按确实受影响的诊断修订，不掩盖错误。
5. 先跑宏单元和直接相关目标，再执行工作区默认/全特性测试与 Clippy、格式检查、release 构建、Rust 1.89 全目标全特性编译、根/宏严格 Rustdoc；单独验证 format、batch、instances、lease 的直接相关独立组合。命令及结果在后续执行记录中填入。
6. 核对版本门禁 python3 tools/check_versions.py --base HEAD，检查差异，明确暂存本次文件，通过 --staged 钩子后提交。核对根 miresidian-v0.28.0 和组件 miresidian_macro-v0.24.0 不可移动标签；不推送或发布 crates.io。

当前状态：已完成本节确认范围的解析、指导文档、示例和验证，准备执行提交及标签核验。本次没有 DLL/真实设备交付要求；Linux 回归和编译不表述为 Windows 加载或硬件通信验证。

实施补充：租约长短语法作为独立概念拆至 macro/src/node/lease.rs，arguments.rs 保留节点入口；不改生成层。新增 examples/macro_shortcuts.rs 作为启用四项 feature 的内存回显 cdylib 示例，由根 Cargo.toml 的 required-features 限定；它是框架工作区内部示例，不是独立子节点。新增 tests/short_instances.rs、tests/short_format.rs 以及短写 UI 样例，现有 Config/Handle/广播/租约测试选择部分改用短写，其余旧长写回归继续保留。版本影响仍仅 root/macro。

### 实际验收记录

| 需求 | 实施与证据 | 结果 |
| --- | --- | --- |
| 裸 instances 与省略标记有区别 | arguments/instances 归一化；short_instances 单对象顺序多通道及替换；原 singleton 的拒绝多通道测试 | 通过 |
| single/multiple、config/handle、固定/可选策略 | 宏单元完整 18 组新旧展开比较、默认及次序；config_instances、handle_instances、broadcast、短写 UI pass/fail | 通过 |
| 裸 lease 默认固定租约 | arguments/lease 归一化；lease_roles 断言 Required、GetInitConfig 无选择器、共享资源身份；lease_final_close 最后引用关闭 | 通过 |
| 可选租约与多 feature 配置布局 | handle_leases 使用新实例和租约标签，覆盖 Handle/访问方式/平台/执行方式顺序及共享调用；旧 lease_macro/初始化失败回归保留 | 通过 |
| 固定/多格式及 batch/async | 六种格式解析与元数据对照；short_format 比较旧/新字段和结果、批量格式列、异步固定格式及完整 JSON ABI；旧 format_feature/batch 保留 | 通过 |
| 旧语法及非法/缺少 feature 诊断 | 所有既有 trybuild 快照未改写；只增加短写的错误快照和 pass fixture | 通过 |
| 指导与版本 | 双语根/宏 README、Rustdoc、手册、发行说明、可编译示例；root 0.28.0 / macro 0.24.0，工作区锁由 Cargo 更新 | 通过 |

执行命令及结果（均使用 --locked --offline，不安装或升级依赖）：

- `cargo test -p miresidian_macro`：14 项通过（包含在工作区回归中）。
- `cargo test --workspace --all-targets`：124 项通过，2 项原有忽略。
- `cargo test --workspace --all-targets --all-features`：248 项通过，3 项原有忽略。两个 Pack 模板集成测试依赖已发布旧版及 registry 缓存，未解禁；lease_process_helper 是仅供父测试以子进程启动的入口。
- `cargo clippy --workspace --all-targets -- -D warnings` 及同命令增加 `--all-features`：通过。新测试内部 Rust 方法名从 new 改为 convert_short 后重跑失败的 Clippy；平台显式操作名不变。
- 独立 instances：short_instances/config_instances/handle_instances/macro_compile_fail 共 13 项通过；独立 lease：lease_roles/lease_final_close/macro_compile_fail 共 3 项通过；独立 format：short_format/format_feature/format_compile_fail 共 5 项通过；独立 batch：batch 共 6 项通过。独立组合覆盖缺少其他 feature 时的编译诊断。
- `cargo +1.89.0 check --workspace --all-targets --all-features`：通过。
- `RUSTDOCFLAGS='-D warnings' cargo doc -p miresidian -p miresidian_macro --all-features --no-deps`：通过；根 format 模块文档保持可生成。
- `cargo build --workspace --release`：通过；`cargo fmt --all -- --check` 与 `git diff --check`：通过。
- 本次 Markdown 的 141 个本地目标、21 个本地锚点：通过。
- `python3 tools/check_versions.py --base HEAD`：通过；独立烟测夹具锁文件和相邻 Node 未修改。暂存、提交和根/宏不可移动标签核验作为后续交付步骤执行，不推送或发布。

补充断言裸 lease 的 Required 与无选择器元数据后，重跑独立 lease_roles 测试和该目标全特性 Clippy，均通过。首次批量对照的 Base64 期望修正为既有 `base64:` 输出前缀；未修改编解码行为。新增 UI 首次运行产生的 wip 快照经逐项核对后加入，原快照保持不变。

临时原始输出位于 `/tmp/miresidian-shortcuts-{default-tests,all-tests,clippy-default,clippy-all,instances,lease,format,batch,msrv,doc,release}.log`；稳定证据为上述测试代码、快照和本文命令记录。未验证 Windows DLL 加载或真实设备通信；没有新旧声明迁移、版本标签移动、外部发布或未选定功能。

## 目标、范围与授权（以下为方案讨论记录）

用户要求提供宏标签简化方案。本任务交付设计文件和新旧语法对照，不实施宏、运行时、节点或依赖修改，不发布或提交。

目标是减少标签数量、重复键名和嵌套层数，使短写精确等价于现有声明。保持现有业务方法、参数位置、名称、描述、格式选项顺序、默认选择、实例身份和租约语义。保留现有长写；不新增 op/in/out 等同义宏，不自动暴露方法，不隐式合并 instance 与 lease 的字段角色，不引入宿主新协议。

当前是待评审方案，不表示新语法已可用，也不表示用户已授权后续实现。

### 评审记录：2026-11-11

用户表示 execution = selectable(sequential) 的形式可以考虑，并要求继续提供其他宏标签优化建议。这仅表示该形式进入待选范围，未确认其他语法，也未授权实施。下列原设计保留为候选，不作为已批准实现范围。新增候选见文件末尾；实施前须按最终选择收敛本文件。

随后用户明确否决集中声明输入参数名称：参数标签必须继续直接标在对应 Rust 参数上，这是既有设计要求，后续不再提出集中 inputs 候选。用户表示 format = hex 可以考虑，并要求兼容其他语法、保持各 feature 的标签风格接近。为满足这一要求，将原设计二的扁平多格式配置收回，改为仅增加固定单格式短写；现有复杂配置继续保留 feature(...) 分组。候选 B 仅留否决记录，候选 C 合并到设计二。这些调整仍只涉及方案，不代表实施授权。

最新要求：将 batch 与固定单格式写法及旧语法兼容纳入计划，继续分析，尤其缩短 lease/instances。已将组合与兼容验收明确写入设计二，并在设计三提出同层键值候选；用户没有授权宏实现。

用户随后明确否决将 execution 提到 node 同层：这样无法从配置位置看出其 feature 归属。撤回同层 instances/execution 拆分；设计三改回 instances(...) 分组内缩写。后续简化必须保留 feature 边界，不能用减少嵌套换取归属不清。之前的同层提议仅作为此处的修订历史，不再作为候选或实施依据。

用户继续指出租约短写遗漏默认项，并否决随后提出的 lease = selectable(lease)。撤回裸 selectable 短写及 feature 名重复充当默认值的方案。以下统一规则是后续评审依据；修正后的租约具体拼写仍是待选，不表示用户已接受。

最新决定：用户选择类型级 lease 也采用与参数级 lease(key) 一致的括号形式，即 lease(required)、lease(disabled)、lease(selectable(required))；这确认了语法方向，不是开始实现的指令。用户同时质疑 instances 简写仍保留 execution =，本轮分析改为 instances(config, selectable(sequential)) 或 instances(handle, parallel)，后者仍待评审。前述等号式 lease 候选全部撤回；集中参数声明和 node 同层 execution 仍禁止。

用户进一步建议：裸 instances 默认 single、顺序执行，多实例可写 instances(multiple, config)。据此调整短写默认规则，替代此前“短写必须声明实例方式和执行策略”的候选。默认值仅适用于新的裸标记/位置参数短写；不添加 instances 时仍禁用多通道，现有完整长写的校验不变。multiple 的 config/handle 关联仍显式声明，不另外猜测默认关联方式。本轮仍只更新方案，不实现。

用户明确认可 format(utf8, hex, base64, selector = "数据格式")。设计二收敛为括号内直接列格式：单格式 format(hex)，多格式第一项默认且 selector 必填；不再新增 format = hex 等号式别名。既有 format(formats = [...], selector = ...) 完整兼容。本次仍为计划更新，未授权开始实现。

用户增加裸 lease 的默认规则：node 中写 lease 即固定使用租约，等价于 lease(required) 和现有 lease(activation = required)，不发布资源访问方式选择器。不写 lease 则仍走现有普通 Direct 路径。此次只新增裸标记，不把 lease() 或独立 #[lease] 自动当作有效别名。

| 项目 | 当前状态 |
| --- | --- |
| format(hex)、format(utf8, hex, base64, selector = "数据格式")，与 batch 组合 | 多格式形式已获用户认可，按同一规则收敛；保留旧语法，未实施 |
| lease、lease(required/disabled)、lease(selectable(required)) | 用户已选择语法方向；裸 lease 默认固定租约，未实施 |
| instances 默认 single/sequential；instances(multiple, config/handle) | 按用户建议纳入新短写设计，待整体收敛，未实施 |
| node 同层 instances/execution 拆分 | 已否决，不实施 |
| lease 省略默认值、lease = selectable(lease) | 已撤回，不实施 |
| output 并入 operation、契约 impl 属性 | 先前候选，未确认，不默认纳入首版实施 |
| 集中 inputs 声明 | 已否决，不再提出或实施 |

## 现状与资料依据

只读核对本检出框架 0.27.0、宏 0.23.2，依据为各自 Cargo.toml 和以下源码：

- macro/src/operation/naming.rs：operation 仅接受可选的单个字符串。
- macro/src/operation/definition.rs：读取独立 output 标签及参数来源，生成输出数量校验。
- macro/src/operation/format.rs：格式声明为 format(formats = [...], selector = "...")；多格式必须显式提供 selector，单格式禁止 selector。
- macro/src/node/instances.rs：实例长写要求 mode、execution；multiple 要求 binding，selectable 要求 default_execution。
- macro/src/node/arguments.rs：lease 要求 activation = ...。
- docs/宏驱动开发.md：固定入口、参数自动来源、平台名称、预填与运行时默认值规则。

现有参数来源推断、操作名称推断和默认输出名称已经存在，不作为本方案新增收益。资料为本地源码事实；本任务不判断 registry 发布或 docs.rs 可用状态，也不选择节点依赖版本。

## 统一语法规则

1. feature 的内部配置必须留在该 feature 分组内，例如 instances(multiple, config, selectable(sequential))。不能把执行策略提升到 node 同层。
2. 裸标记可表示启用能力并使用明确约定的默认值：batch 保持当前语义，instances 短写展开为 single + 固定 sequential，lease 展开为固定 required。新带值语法采用 feature(...) 分组；lease 的括号形式与 format 内直接列格式的形式已由用户选定。format = hex 仅留评审历史，不新增为别名。
3. 有运行时可选策略时，selectable(default) 的 default 必须显式写出，并复用该配置固定形式的取值。例如执行策略固定 sequential 对应 selectable(sequential)；租约候选固定 required 对应 selectable(required)。不能把 feature 名当成其策略取值。
4. 不同 feature 的取值域保持独立。默认值必须逐项约定和列出等价展开，不能从其他 feature 推导；instances 的 single/sequential 默认来自用户建议。format 保留既有单格式固定、多格式首项默认的规则，不增加 selectable 包装，不自动启用 Cargo feature。
5. 当前已实现的完整语法保留；新短写的组合、默认值和旧展开必须逐项列出等价关系。未确认的拼写不是实施授权。
6. input/config 标签及其可选能力继续直接附着对应参数，不集中搬到方法标签中。

## 设计一：普通输出名称并入 operation

现有：

```rust,ignore
#[miresidian::operation("测量")]
#[miresidian::output("电压", "电流")]
fn measure(&mut self) -> MiresidianResult<(f64, f64)> { /* 业务实现 */ }
```

短写：

```rust,ignore
#[miresidian::operation("测量", output = ["电压", "电流"])]
fn measure(&mut self) -> MiresidianResult<(f64, f64)> { /* 业务实现 */ }
```

单输出可写 output = "电压"。操作名称可以省略，例如 operation(output = "电压")，继续使用当前 Rust 方法名转换规则。初始化可采用同样短写，仅命名 Self 之外的业务输出；关闭及探测的固定输出契约不接受此选项。

operation 参数先解析为结构化声明，再将输出名称接入现有输出数量检查和元数据生成流程。普通业务输出继续与原类型编解码对应，不改变 format、batch 和 async 的处理方式。

有逐项输出格式声明时继续使用现有独立 output 标签；第一版不在 operation 内增加一套复杂输出 DSL。同一方法不能同时使用 output = ... 和独立 output 标签，无论内容是否相同均编译报错。空数组、重复 output 选项、非法元素和输出数量不匹配均编译报错。不写 output 的现有行为保持不变。

收益：普通具名输出由两个方法属性减少为一个，不搬移参数声明或业务代码。

## 设计二：format 直接列格式与现有 feature 语法兼容

现有：

```rust,ignore
#[miresidian::input("数据", format(formats = [hex]))]
data: Vec<u8>
```

短写：

```rust,ignore
#[miresidian::input("数据", format(hex))]
data: Vec<u8>
```

相同短写适用于独立 output 标签；batch 仍只允许普通业务 Input，继续使用已有独立标记，组合可写 input("数据", batch, format(hex))。新短写不隐含启用 Cargo feature，不隐含 batch，不改变参数来源或转换支持范围。六种格式标识符 ascii、gbk、utf8、utf16le、hex、base64 均沿用既有定义。

多格式现有写法：

```rust,ignore
#[miresidian::input("数据", batch,
    format(formats = [utf8, hex, base64], selector = "数据格式"))]
data: Vec<u8>
```

用户认可的新短写：

```rust,ignore
#[miresidian::input("数据", batch,
    format(utf8, hex, base64, selector = "数据格式"))]
data: Vec<u8>

#[miresidian::output("结果",
    format(utf8, hex, base64, selector = "结果格式"))]
```

### 语法与默认值

- 一个格式为固定转换，不生成格式选择 Input，也不接受 selector。
- 两个及以上格式为运行时选择，selector 必填；格式第一项仍是默认项，声明顺序仍是平台选项顺序。
- 新短写的格式位置项在前，selector = "..." 在后；selector 继续使用既有非空名称及保留分隔符校验。除 selector 外不增加其他具名选项。
- 空 format()、未知格式、重复格式、非标识符格式项、重复 selector 或其他非法组合均编译报错，不猜测编码默认值。
- input/output 的格式语法一致；格式仍只用于普通业务 Input/Output，不扩展到 Config 或固定生命周期入口。
- 外层名称字符串仍位于增强选项之前；batch 与 format(...) 在名称之后可交换顺序，槽位顺序不受标注选项顺序影响。

### 旧语法与解析兼容

- 当前已实现的 format(formats = [...], selector = "...") 和固定单格式 format(formats = [hex]) 全部保留，包括已有合法的具名选项次序。
- format 调用内部用 formats = ... 区分旧长写，用格式标识符位置项区分新短写，二者都归一为相同 Format 中间结构，复用既有检查和代码生成。
- 不允许在一个 format(...) 内混用 formats = [...] 和格式位置项，不允许同一参数或输出重复声明 format，不通过优先级覆盖。
- format = hex、format = [hex]、format = [utf8, hex] 及外提 selector 的中间候选都尚未实现，本轮不新增这些别名；它们不属于需要继续支持的现有合法语法。
- 新旧等价声明生成相同的格式选择器名称、位置、候选文本和默认项。输入选择器仍跟随对应数据，输出选择器仍追加在方法 Input 末尾。
- 业务描述不生成运行时默认值；基础类型解析、转换支持范围和 feature 门禁不变。

收益：固定格式删除数组和重复键名；多格式删除 formats = 和数组包装，所有格式配置留在 format(...) 内。旧声明和合法 feature 组合不要求迁移。

### batch 与 format 的兼容验收矩阵

| 声明 | 含义及验收 |
| --- | --- |
| input("数据", batch) | 现有按原类型展开，保持不变，不依赖 format feature |
| input("数据", format(hex)) | 新固定格式短写，等价于 format(formats = [hex])，不隐含 batch |
| input("数据", batch, format(hex)) | 新组合，等价于 batch, format(formats = [hex]) |
| input("数据", format(hex), batch) | 与上一行等价，选项次序不影响槽位或展开方式 |
| input("数据", batch, format(formats = [utf8, hex], selector = "数据格式")) | 已有多格式组合继续合法，长度对齐与逐条转换不变 |
| input("数据", batch, format(utf8, hex, selector = "数据格式")) | 与上一行等价，首项默认和格式列表/记录广播保持不变 |
| output("结果", format(hex)) | 新固定输出格式，等价于旧单格式输出标签；不增加选择槽位 |
| output("结果", format(utf8, hex, selector = "结果格式")) | 与旧多格式输出等价，选择器仍追加于方法输入末尾 |
| output("结果", batch) | 仍非法，不因新短写开放输出 batch |

验收分别覆盖只启用 batch、只启用 format、两者组合，以及结合 instances/lease 的合法声明。对比解析值、执行次数、输出聚合、格式槽位和缺失 feature 的诊断；普通 Vec、空记录、转义与范围语法不改写。不新增 batch = true 或从 format 隐式推导 batch。

## 设计三：保留 feature 分组的实例与租约短写

现有：

```rust,ignore
#[miresidian::node("设备",
    instances(mode = multiple, binding = config,
        execution = selectable, default_execution = sequential),
    lease(activation = selectable)
)]
```

当前组合形式（lease 已选定，instances 更短写法待评审）：

```rust,ignore
#[miresidian::node("设备",
    instances(multiple, config, selectable(sequential)),
    lease(selectable(required))
)]
```

实例模式、关联方式与执行策略继续放在 instances(...) 内，既不提升到 node 同层，也不强制重复 mode/binding/execution 键名。模式 single/multiple、关联 config/handle 和执行策略是三个不重叠的类别；selectable(...) 在本分组中明确指执行方式，无需运行时猜测。lease 使用用户选定的括号形式，默认项 required 表示 Lease 路径；需要更多配置时保留完整长写。

### 新短写的默认规则

- 出现裸 instances 或空分组 instances()：显式启用该能力，展开为 single + 固定 sequential，允许多通道输入，仍要求 Cargo instances feature。
- 位置参数短写省略模式时默认 single，省略执行策略时默认固定 sequential。
- 多实例必须明确写 multiple 和 config/handle；不根据 config/handle 自动推导 multiple，也不为 multiple 额外默认一种关联方式。
- single 不接受 config/handle 关联参数，沿用现有单实例模式约束。
- selectable(...) 依然要求显式默认策略，不把 selectable() 或裸 selectable 当成固定顺序执行。
- 不写 instances 的默认节点保持 InstancePolicy::DEFAULT，即 single + ExecutionPolicy::Disabled，不支持多通道；不能将它改成新裸标记的行为。

这些默认值只归一化到现有 Single/Multiple、Config/Handle 和 Fixed/Selectable 类型，不新增运行时策略。

### lease 裸标记默认规则

```rust,ignore
// 新裸标记
#[miresidian::node("设备", lease)]

// 新显式短写：与上面相同
#[miresidian::node("设备", lease(required))]

// 现有完整写法：与上面相同
#[miresidian::node("设备", lease(activation = required))]
```

裸 lease 归一化为 LeaseActivation::Required，固定使用租约，不生成 Lease/Direct 平台选择器。它仍要求 Cargo lease feature，以及新参数角色形式至少一个 lease(key)，不改变资源键、配置比较或共享节点规则。不写 lease 保持原有 Direct 路径，lease(selectable(required)) 才发布选择器并默认租约；不能将裸标记解释为 selectable。

实例方式映射：

| 短写 | 长写 |
| --- | --- |
| instances 或 instances() | instances(mode = single, execution = sequential) |
| instances(single) | instances(mode = single, execution = sequential) |
| instances(single, sequential) | instances(mode = single, execution = sequential) |
| instances(parallel) 或 instances(single, parallel) | instances(mode = single, execution = parallel) |
| instances(selectable(sequential)) | instances(mode = single, execution = selectable, default_execution = sequential) |
| instances(multiple, config) | instances(mode = multiple, binding = config, execution = sequential) |
| instances(multiple, handle) | instances(mode = multiple, binding = handle, execution = sequential) |
| instances(multiple, config, parallel) | instances(mode = multiple, binding = config, execution = parallel) |
| instances(multiple, handle, selectable(broadcast)) | instances(mode = multiple, binding = handle, execution = selectable, default_execution = broadcast) |

执行与租约映射：

| 短写 | 等价含义 |
| --- | --- |
| instances(..., sequential / parallel / broadcast) | 固定对应执行策略 |
| instances(..., selectable(sequential)) | 原 execution = selectable, default_execution = sequential；仍提供原有全部执行选项 |
| instances(..., selectable(parallel)) | 原可选执行策略，默认 parallel；不限制为单一可选项 |
| lease | lease(activation = required)，固定租约，不生成选择器 |
| lease(required) | lease(activation = required)，固定租约模式 |
| lease(disabled) | lease(activation = disabled)，固定 Direct 模式 |
| lease(selectable(required)) | lease(activation = selectable)，可在 Lease/Direct 间选择，默认 Lease |

短写按模式、关联、执行三类各至多一个解析，按上述规则填充省略的 single 与固定 sequential；显式 multiple 的关联项不得缺失。推荐顺序为模式、关联、执行策略，解析可按不重叠的类别识别顺序，不能用后一项覆盖前一项。node 同层不接受执行策略。所有显式 instances 新写法的 feature 要求与原显式声明保持一致。lease 与实例配置彼此独立，可单独声明。

当前源码的 LeaseActivation::Selectable 不携带默认值，默认 Lease。本轮语法等价范围只确认 required 这一默认项可归一到现有类型；selectable(disabled) 若要表示默认 Direct，属于额外的默认策略能力，不能声称当前类型已经支持。其元数据、缺省值解析和公开类型设计须在用户选定范围后另行完善，未完成前不列入本次可直接实施的短写集合。

### 长写兼容和冲突规则

- 当前合法的 instances(mode = ..., binding = ..., execution = ..., default_execution = ...) 与 lease(activation = ...) 全部保留；旧调用和 feature 组合不要求迁移。
- 原 lease(activation = ..., resource = "...", configuration = ["..."]) 保持原有成对校验与参数角色冲突校验。需要旧字段列表时继续使用完整长写，不自动与新的参数级角色混用。
- 不同能力可分别采用新旧形式，例如旧 instances(...) 配合 lease(selectable(required))；同一个能力不得重复声明，即使值相同也报重复配置。
- 裸 lease 与任何 lease(...) 同时出现均报重复声明；不以长写覆盖裸标记。裸标记仅为 node 的子项，不恢复独立 #[miresidian::lease] 属性；未声明支持空 lease()。
- instances 位置参数短写与 mode/binding/execution/default_execution 长写不能在同一分组混用。模式 single/multiple、关联 config/handle、执行策略各至多一个；重复相同值也报错。固定策略与 selectable(...) 都属于执行策略，二者并存同样报错。
- 已实现的完整实例长写继续支持 execution = selectable 配合 default_execution = ...。新短写中的 selectable(...) 不再附加 execution/default_execution。此前未实现的 execution = selectable(...) 中间候选不自动作为额外别名加入首版，减少同义写法。
- 不依赖属性参数声明次序；先解析并记录语法来源，再统一校验组合和重复项，错误指向冲突选项。
- selectable(...) 在 instances 内恰好接收一个合法执行策略作为默认值，不增加策略子集功能；参数缺失、未知或类型错误均编译报错。multiple 缺少 config/handle、single 携带关联参数、未声明 multiple 却写关联参数均报错；短写省略执行策略则按已约定的 fixed sequential 展开。
- 完整 NameValue 长写仍按现有要求校验必填 mode/execution/multiple 的 binding/selectable 的 default_execution，不借新短写默认规则静默补齐缺失字段。裸 instances 和全位置项分组明确走新短写归一化路径。
- 租约短写同样要求 selectable(...) 显式提供默认策略；lease(selectable) 以及已撤回的等号式 lease 短写不新增为合法形式。旧 activation = selectable 继续保留其既有默认行为，不追溯要求旧声明迁移。

此前提出的同层 instances = config 与 node 同层 execution 已被否决，不实施。instances(config, ...) 等省略 multiple 并由关联方式推导模式的中间候选，由本轮显式 multiple + 关联的形式替代，不默认作为额外别名加入。lease、lease(required)、lease(disabled)、lease(selectable(required)) 已获用户对语法方向的选择；尚未开始实现。现有完整长写全部保留。

参数级 instance(key/ignore)、lease(key/ignore) 保持现状。两种身份可能不同，禁止通过一个裸 key 标签或启用 lease 就自动绑定两者；不为减少数个字符丢失字段含义。

### 实现方法与验收

node 参数入口为 instances 增加裸 Path 形式，分组形式保留。解析器先区分完整 NameValue 长写和新位置参数短写；短写识别三类项：模式 Path(single/multiple)、关联 Path(config/handle)、执行 Path(sequential/parallel/broadcast) 或 List(selectable(default))。仅在短写路径补 single 和 Fixed(Sequential)，显式 multiple 校验关联必填；归一化为现有 InstancePolicy 并沿用 feature 门禁。保留 span 用于重复、冲突及缺项诊断，不按字符串长度或 Rust 方法语义推断。

node 入口为 lease 增加裸 Path 子项，将其构造成与 lease(required) 相同的 LeaseDeclaration。lease 分组解析器接受现有 NameValue 长写，或单个 Path(required/disabled)/List(selectable(required)) 短写，归一化到现有 LeaseActivation。裸标记/分组重复、策略重复、旧 activation 与新策略混用均报错。租约构建、实例路由、配置规范化与运行时不另建短写分支；不将尚未设计完成的默认 Direct 能力夹带进宏简化。

比较新裸标记、显式 single、multiple/config、multiple/handle 和固定/可选执行策略的新旧等价声明，包括 Lease/Direct、选择器追加位置、Handle、平台适配与执行方式 Config 顺序；验证初始化、复用、冲突与关闭结果一致。单独证明不写标记仍拒绝多通道，而裸 instances 接受单对象顺序多通道；两者固定查询均不新增执行方式选择器。覆盖短写默认策略、裸/空分组等价、新旧跨能力组合、同能力重复、multiple 缺关联、single 带关联、裸关联、策略冲突、旧长写缺必填项、node 同层执行策略及 feature 未启用的诊断。

租约另比较裸 lease、lease(required)、旧 activation = required 的 descriptor、GetInitConfig、资源请求及初始化/关闭结果完全一致；验证不追加选择器，不写 lease 时仍为 Direct，裸 lease 与实例/格式/batch 组合及旧长写跨能力混用正常。覆盖缺少 Cargo lease feature、缺少资源键、裸标记与 lease(...) 重复的诊断。

收益：常用声明删除 mode、binding、execution、activation、default_execution 五个中间键名；默认单对象顺序多通道缩为裸 instances，多对象顺序执行只写 multiple 与关联方式。保留 feature 分组和 selectable 的显式默认值，配置归属直接从位置看出。

## 平台契约与生命周期

以上仅改变宏输入形式，不改变 Config/Input/Output 槽位、GetFunctions/GetInitConfig 文本及顺序、框架选择器、错误分类、ABI 或租约协议。参数 instance(key/ignore)、lease(key/ignore) 保留原写法，保持两套角色独立。

同一新旧语法对必须生成等价的公开元数据和行为；本方案没有新的运行时状态、通信协议、锁、超时、取消或资源生命周期。

## 实施文件与版本评估

后续实施文件：macro/src/operation.rs、operation/naming.rs、operation/definition.rs、operation/format.rs、node/arguments.rs、node/instances.rs，以及实际需要的内部解析模块；测试位于既有宏编译测试和根框架集成测试目录。

文档同步：macro/README.md、宏 Rustdoc、根 README.md/README.en.md 中受影响内容、docs/宏驱动开发.md、docs/instances.md、docs/config-identity.md、features/format/README.md、features/batch/README.md、features/lease/README.md 中实际受影响的标签指导，以及对应组件 CHANGELOG 和迁移说明。只更新实际受影响的示例，不要求独立 Node 改写或升级；指导内容变化是否构成对应组件独立发行在实施时按实际打包范围评估。

- 本次只新增方案文件：版本影响无；没有产品、指导性发行文档或运行行为变更，不升版、不生成版本标签。
- 后续实施新增公开宏语法，按仓库 pre-1.0 公开 API 门禁提升宏次版本。若以上述源码为基线，候选宏版本为 0.24.0。
- 根框架采用新宏并公开该语法时同步更新精确内部依赖，按公开能力扩展评估根次版本；若仍以上述源码为基线，候选为 0.28.0。实施前重新核对 HEAD 与各组件实际版本。
- ABI、租约、codec、format、batch、instances 运行库及工具没有独立变化，不联动升版。需要同步所有实际受影响锁文件和带日期的组件 CHANGELOG。

## 实施步骤与验收

1. 实施前按用户最终选定范围收敛方案；lease 括号形式与 format 直接列格式已选定语法方向，但本轮仍为方案讨论。未确认的 output 合并、契约 impl、实例分组短写不得因出现在本文就自动实施，已否决的 node 同层 execution 和等号式 feature 别名不纳入实现。在选定的解析入口建立新旧语法的统一中间表示，确保错误定位指向用户选项；不改业务调用生成逻辑。
2. 仅在用户选用设计一时实现 operation 输出短写，验证单输出、多输出、初始化、自动操作名、独立 output 冲突、错误数量及带格式的旧标签兼容；否则跳过并保留原标签。
3. 实现 format 位置参数短写，验证六种固定格式、新旧多格式等价、输入/输出、选择器位置、格式顺序与默认项、旧具名选项次序、batch 选项交换顺序、async、feature 门禁、重复声明及新旧混用诊断。
4. 仅在选定节点短写方案后实施，验证 single/config/handle、固定与可选执行策略、各租约模式、旧资源字段分组、新旧跨能力组合、缺失/重复/冲突及 feature 门禁，复用现有策略验证。
5. 对每组等价新旧声明比较 descriptor、GetFunctions、GetInitConfig 和正常/错误调用结果，覆盖默认 feature 及直接涉及的 instances/lease/format/batch 组合；旧声明测试继续通过。
6. 同步实际版本、精确依赖、锁文件、指导文档与修订归档；执行受影响范围的格式、编译、测试和版本门禁，再按仓库规则提交并核对不可移动标签。

验收重点为标签数量和语法嵌套确有减少、现有调用契约逐项一致，不以代码展示换行数作为收益证明。此次方案交付仅完成源码调研与设计；全部实现和运行验收均未执行。

## 后续候选 A：契约 impl 块补齐 operation 标记

拟新增可选属性 miresidian::operations，用于显式选择一个固有 impl 块作为契约区域。块内所有方法均为平台方法；未写 operation 的方法由外层属性补上现有 operation，已写 operation 的方法保留其显式名称和选项。固定入口仍按原有 Rust 方法名识别，辅助方法放在未标记的普通 impl 中。块内非方法项第一版明确拒绝，不猜测是否参与平台契约。

```rust,ignore
// 当前
impl Device {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> { /* 原实现 */ }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> { /* 原实现 */ }
    #[miresidian::operation]
    fn reset(&mut self) -> MiresidianResult<()> { /* 原实现 */ }
}

// 候选
#[miresidian::operations]
impl Device {
    fn init_device() -> MiresidianResult<Self> { /* 原实现 */ }
    fn close_device(&mut self) -> MiresidianResult<()> { /* 原实现 */ }
    fn reset(&mut self) -> MiresidianResult<()> { /* 原实现 */ }
}
```

该属性只补齐既有方法宏，不负责生成另一套注册或节点汇总，不恢复历史 node_impl 机制；现有逐方法写法、多个 impl 和一个模块一个具体节点的规则保留。不自动导出其他 impl 的方法。已有三类参数/输出标签、cfg/cfg_attr 及 sync/async 均交给现有 operation 处理。

收益边界：N 个无参数 operation 标签减少为一个块标签；显式中文操作名仍需写 operation("名称")，不承诺省掉这部分标签，也不从注释猜平台名。

实现范围：宏公开入口新增 operations、独立的 impl 属性展开模块、对应 Rustdoc/README/手册及编译测试。验收比较手写与自动补齐的固定入口、操作名、顺序和元数据，覆盖多 impl、cfg、async、显式名称及错误签名；普通 impl 的辅助方法不得出现在 GetFunctions。

## 已否决的候选 B：集中声明输入名称

此前曾提出在 operation 中集中声明 inputs，用户明确否决，尚未实施。

当前约束：input/config 标签直接标在对应参数上，名称、描述及可选能力就地声明，保持对应关系清晰；不实现、不推荐集中 inputs 列表。保留此段只记录评审结论，不作为候选或实施步骤。

## 已合并的候选 C：固定单格式简写

早期曾讨论 format = hex；最新已收敛到设计二的 format(hex) 和多格式位置参数形式。等号式写法未实施，不再新增，不维护另一套格式语法候选。

## 新候选的版本与执行边界

候选 A 未确认，B 已否决，C 已合并到设计二；均未实施。当前仍只有方案文件变化，版本影响无。后续若选用，公开宏语法、根框架依赖、指导文档、锁文件和版本门禁沿用前文规则；没有独立运行时或 ABI 变更，不联动无关组件。
