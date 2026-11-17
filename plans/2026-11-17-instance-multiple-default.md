# 框架 0.34 / macro 0.30：实例与格式宏简写

日期：2026-11-17。

## 实施决议与 encoding 名称统一（2026-11-17，优先于下方方案编写记录）

用户明确要求“先执行当前已写入计划的内容”，随后将 format 改名为 encoding，并明确要求 crate 名称一致。按该授权实施本方案的 instance 与格式简写，以及本节名称迁移；此前“仅计划、未授权实现”描述保留为历史记录，不覆盖本节。不推送、不发布，不加入此前讨论的错误增强、输出样式、编码组合或共享选择器。

| 原入口 | 本次统一入口 |
| --- | --- |
| Cargo feature format、宏分组 format(...) | encoding、encoding(...) |
| miresidian_format、features/format/ | miresidian_encoding、features/encoding/ |
| miresidian::format | miresidian::encoding |
| FormatInput / FormatOutput | EncodingInput / EncodingOutput |
| from_format_bytes / into_format_bytes | from_encoding_bytes / into_encoding_bytes |
| decode_formatted / decode_formatted_batch / encode_formatted | decode_encoded / decode_encoded_batch / encode_encoded |
| select_format / select_format_value | select_encoding / select_encoding_value |

- 不保留旧 feature、宏分组、crate 重导出或公开 API 别名；旧 Node 保持自己已发布版本的旧用法。下方 format 语法设计按 encoding 入口实施，formats 长写字段、selector 字段与平台“格式”显示文本保持，自动名称仍为“参数名 + 格式”。
- 基础 codec 的 ByteTextFormat、parse_byte_text_format、文本/字节通用转换函数，以及 Rust format!、平台 format_parameter 等通用概念不属于本能力重命名，不修改其 API 或语义。
- 移动宏内部 operation/format.rs 为 operation/encoding.rs，测试目标、UI 目录、feature 门禁、CI 命令及实际相关当前指导引用同步。历史 CHANGELOG/发行说明/方案保留版本事实，仅修正源码移动后失效的相对链接。
- 实施版本：根 0.34.0、macro 0.30.0、新包 miresidian_encoding 0.5.0（原 miresidian_format 0.4.1 的 API/包名迁移）；batch 0.3.3 仅同步组合指导，不改变批量行为。Pack 0.14.2 将新包纳入来源清单并保留旧 miresidian_format 的严格来源校验；CLI/套件 0.3.10 同步最低 Pack。其他组件、ABI v10、租约协议及 MSRV 不变。
- 修改工具清单和 tools/check_versions.py 时同时识别新旧包名；历史快照的库清单按其实际旧包名映射，不能让独立 Node 绕过 crates.io 来源约束。独立 Node 与独立烟测夹具的清单/锁文件保持已发布依赖，不用本地覆盖采用候选。
- 每个实际升版组件同步 README、日期 CHANGELOG、精确依赖和 Cargo 生成锁文件；根双语 README/CHANGELOG、Rustdoc、相关手册、新 0.34.0 迁移页同步。Pack/CLI 补丁实施范围追加至原 tools 0.14 / 0.3 方案，不重复建案。
- 用户特别要求检查 skill、文档与注释：审查仓库和已安装 Node skill 的完整内容及一致性；如无旧名引用则保留版本无关流程，不为改名写入固定 API。确有受影响的内容时按 skill 维护规则同步完整双副本并验证，不能只改 SKILL.md。
- 验收采用下方矩阵的 encoding 新名称，增加旧 format 入口拒绝、新旧包来源约束、encoding 转换回归、Pack/CLI 单元与进程测试。涉及多个组件，执行工作区默认/全特性测试与 Clippy、release 构建、MSRV all-targets 检查及严格 Rustdoc；不声称 Windows 原生加载或真实硬件验证。
- 提交前执行工作区和暂存区版本门禁；提交后核对 miresidian-v0.34.0、miresidian_macro-v0.30.0、miresidian_encoding-v0.5.0、miresidian_batch-v0.3.3、miresidian_pack-v0.14.2、miresidian_cli-v0.3.10 标签，既有标签不得移动。记录实际结果及未完成项后交付。

## 状态、目标与执行依据

- 所属项目为 Miresidian 框架及过程宏；目标版本为根框架 0.34.0、miresidian_macro 0.30.0。同一目标主/次版本后续新增需求继续修订本文件，保留决策和实施记录。
- 用户提出用 multiple(config) 表达多实例与关联方式的从属关系，并要求 multiple 不带括号时默认 config；随后明确要求“写入下一个版本更新计划”。本轮授权为方案落盘，不开始产品实现、升版、推送或发布。
- 目标：新增 multiple(config/handle) 短写，并让裸 multiple 默认采用 Config 关联；保留现有平铺短写及完整长写。
- 2026-11-17 追加目标：用户确认 format 无 selector 时自动生成名称，以及 format("名称", utf8, hex) 字符串首项简写，并要求“加入计划”。该追加同属根 0.34 / macro 0.30，继续修订本文件，仍仅授权更新方案。
- 本轮不纳入 input 省略名称、batch 分隔符调整、显式默认格式或其他宏简化，也不改变实例键选择、对象生命周期、多通道调度、租约、ABI 或协议。

## 现状与依据

- 当前清单为根框架 0.33.0、miresidian_macro 0.29.0、miresidian_instance 0.17.0。版本以各自 Cargo.toml 为准；当前 README 将这些源码标为候选，本轮不查询或推断 crates.io/docs.rs 的实时发行状态。
- macro/src/node/arguments.rs 将 instance 分组交给 macro/src/node/instance.rs。后者区分长短写，归一为现有 InstancePolicy；生成和运行时继续消费同一策略。
- 当前短写要求 multiple 与 config/handle 分别声明，裸 multiple 报错；multiple(config/handle) 尚不支持。single 禁止指定关联方式，省略执行策略时为固定 sequential。
- 当前 format 由 macro/src/operation/format.rs 解析；单格式固定且不接受 selector，多格式必须显式写 selector = "名称"，第一项决定默认格式。字符串首项尚不支持；macro/src/operation/definition.rs 已检查业务 Input 与所有格式选择输入的名称冲突，可复用该检查。
- 现有规则及依据见[实例手册](../docs/instances.md)、[实例名称迁移](../docs/releases/0.33.0.md)与[方案管理规范](../docs/coding-standards.md#01-按更新范围管理方案并严格按方案实现)。本变更是本地宏语法扩展，不引入第三方 API 或依赖。

## 对外语法与精确契约

下表是待实现目标，不能当作当前已支持用法。

| 目标写法 | 归一后的完整声明 |
| --- | --- |
| instance(multiple) | instance(mode = multiple, binding = config, execution = sequential) |
| instance(multiple(config)) | instance(mode = multiple, binding = config, execution = sequential) |
| instance(multiple(handle)) | instance(mode = multiple, binding = handle, execution = sequential) |
| instance(multiple, parallel) | instance(mode = multiple, binding = config, execution = parallel) |
| instance(multiple(config), broadcast) | instance(mode = multiple, binding = config, execution = broadcast) |
| instance(multiple(handle), selectable(sequential)) | instance(mode = multiple, binding = handle, execution = selectable, default_execution = sequential) |

推荐组合：

```rust,ignore
// 待实现的 0.34.0 / macro 0.30.0 语法。
#[miresidian::node(
    "设备",
    instance(multiple, selectable(sequential)),
    lease
)]
struct DeviceNode;

// 需要 Handle 关联时显式指定：
// instance(multiple(handle), selectable(sequential))
```

1. 裸 multiple 在整个分组解析完毕且未提供关联方式时，才补默认 config；原 multiple, handle 必须继续得到 Handle 关联，不能提前补默认导致误判重复。
2. multiple(...) 恰好接受一个 config 或 handle，可带单个末尾逗号；不接受空括号、未知值、具名字段或两个关联值。multiple() 不作为裸 multiple 的别名。
3. 继续兼容 multiple, config、multiple, handle 及它们原有的合法排列次序；保留完整长写。推荐示例采用裸 multiple 或 multiple(config/handle)。
4. 同一分组至多声明一个模式和一个关联方式。multiple(config), config、multiple(handle), config、multiple, multiple(config)、single, multiple(handle) 等重复或冲突均报错，同义重复也不接受。
5. execution 仍位于 instance 分组中，与 single/multiple 并列，适用于两种实例模式；固定 sequential/parallel/broadcast 及 selectable(default) 的取值、默认和互斥规则保持。
6. single 仍禁止关联方式；不新增 single(config/handle)，孤立 config/handle 也不推断为 multiple。
7. 不写 instance 时仍为单实例且禁用多通道；裸 instance、instance()、instance(single) 仍为单实例、固定顺序多通道。
8. 默认 config 只改变短写的关联方式省略规则：有 instance(key) 时使用显式业务主键，否则排除 instance(ignore) 后采用业务 Config。归一化、空身份、同键配置冲突等规则保持。
9. 完整长写保持原必填检查，mode = multiple 缺 binding/execution 仍报错；同组不混合长短写。重复 instance 声明及未开启根 instance feature 的用法仍拒绝。

### single 与执行策略保持现状

- 用户在计划编写期间核实 single 的执行方式，并在理解现有并发边界后要求继续完成计划；本轮不修改调度语义，也不移除 single 可用的任何执行策略。
- instance(single, sequential/parallel/broadcast) 及 instance(single, selectable(sequential/parallel/broadcast)) 中的斜线表示分别可选的值，不是宏字面语法；省略 single 的现有等价短写继续有效。
- 执行策略继续与实例模式并列，例如 instance(single, parallel) 和 instance(multiple(handle), parallel)，不放入 multiple(...) 内。
- 依据 src/runtime/multi_channel.rs 与 src/runtime/lifecycle/instance.rs：Parallel 并发调度通道 Future，但同对象的完整业务调用持有状态锁，包括 async 等待期间；不承诺同对象业务重叠或单实例吞吐提升。结果按输入位置排列，业务执行顺序不保证，等待各通道结束后按输入位置报告首个错误，已发生的副作用不回滚。
- instance 模式决定对象数量与关联；一次多通道请求仍只定位一个对象。不得把 multiple 或 parallel 的新示例描述成自动将通道分配给多个对象。

### format 自动命名与字符串首项简写

以下为待实现的新增契约；格式名使用 utf8，用户示例中的 uft8 视为笔误，不引入别名。

| 目标写法（参数显示名为“发送数据”） | 归一结果 |
| --- | --- |
| format(hex) | 固定 HEX，无选择器、无额外 Input |
| format(utf8, hex) | 两种格式，选择器自动命名为“发送数据格式”，默认 UTF8 |
| format("输入编码", utf8, hex) | 两种格式，选择器命名为“输入编码”，默认 UTF8 |
| format(utf8, hex, selector = "输入编码") | 保留既有短写，等价于字符串首项写法 |
| format(formats = [utf8, hex], selector = "输入编码") | 保留完整长写，等价于字符串首项写法 |
| format(formats = [utf8, hex]) | 与省略 selector 的短写统一，自动命名为“发送数据格式” |

```rust,ignore
// 新语法示意；Input 和 Output 使用同一规则。
#[miresidian::input("发送数据:平台描述", format(utf8, hex))]
// 自动生成“发送数据格式”，不包含冒号后的描述。
#[miresidian::output("接收数据", format(utf8, hex))]
// 自动生成“接收数据格式”；输出格式选择仍属于方法 Input。
#[miresidian::input("数据", batch, format("数据编码", utf8, hex))]
// batch 保持独立，格式选择仍支持既有的逐条对齐与单值广播。
```

1. 无 selector 的自动命名仅用于多格式。名称采用外层 input/output 显示名称在首个冒号之前的部分，按现有名称规则去除首尾空白，再追加“格式”。不采用 Rust 变量名，不猜测输入/输出方向，也不自动加编号。
2. 固定单格式不生成选择器，因此 format("名称", hex) 与 format(hex, selector = "名称") 均报错；format() 和 format("名称") 也报错。
3. 字符串名称只能作为 format 分组的首项，之后列出至少两种合法且不重复的格式；允许单个末尾逗号。不接受多个字符串、格式项之后的字符串、非字符串名称或字符串首项与 selector 同时声明，同值重复也拒绝。
4. 显式及自动生成的选择器名称均沿用现有有效性约束：不得为空或全空白，不得包含冒号、斜线或竖线。自动名称无效时提示显式指定合法名称；显式名称不额外改变原有保留文本规则。
5. 自动名称和显式名称都进入现有方法级 Input 名称冲突检查，包含业务 Input、输入格式选择器、输出格式选择器。任何冲突均编译报错并提示显式指定不同名称，不靠声明次序或自动编号消解；同一名称位于不同方法时不构成冲突。
6. 保留既有 selector = "..." 短写和 formats = [...] 完整长写及其合法次序；不混合字符串首项与 formats 数组，也不混合位置格式项与 formats 数组。仅“多格式缺 selector”从错误变为自动命名，其他既有非法用法不放宽。
7. 第一种格式继续作为默认格式；缺失、空白或 batch 格式空项沿用该默认值，未知/不允许格式仍报错。不新增 default = ...、selectable(...)、格式别名或格式自动探测。
8. 输入选择器仍紧随对应数据 Input；输出选择器仍按输出声明顺序追加到方法 Input 末尾。自动命名与显式写出同名选择器时，元数据、槽位、默认与运行结果必须一致。
9. 自动名称属于对外平台契约；修改外层参数显示名称会同步改变自动名称，需要固定名称的节点采用字符串首项或既有 selector 写法。单格式、batch 记录/格式列规则、format 转换 trait、严格编码错误和 Cargo feature 门禁保持。

## 实现边界与文件范围

- 解析集中在 macro/src/node/instance.rs：在短写分支识别 Meta::List 的 multiple，复用模式和关联方式的重复校验，解析结束后补默认关联。无需新建通用解析层或运行时策略分支。
- 调用链保持 node 参数解析 → instance 长短写归一 → InstancePolicy → 现有宏生成 → 实例容器和调度。相同策略的新旧声明应生成等价策略及平台契约。
- format 解析集中在 macro/src/operation/format.rs：按字符串首项、位置格式项和具名长写区分入口；先检查格式列表与显式名称，再在外层 annotation 获得唯一 input/output 名称后补齐自动 selector。完成归一后保持现有 Format 结构及生成/转换路径，只有固定单格式的 selector 为 None。
- format 调用链保持 operation 标注解析 → 格式与名称归一 → definition 中 Input 名称冲突检查 → 参数元数据与 dispatch → 既有 format/batch 选择和转换函数。macro/src/operation/definition.rs 仅按必要程度补充冲突诊断，不扩大为无关命名规则重构。
- format 回归使用 tests/short_format.rs、tests/format_feature.rs、tests/format_compile_fail.rs、tests/ui-format/、tests/ui-format-feature/，补充自动命名与字符串首项成功/失败用例；macro/src/operation/batch.rs、operation/dispatch.rs 和 features/format/src/conversion.rs 作为行为核对依据，无需新增运行时分支。
- 本变更不建立连接、持有资源或创建异步任务；资源所有权、共享状态、引用计数、正常/失败关闭、并发、超时和取消沿用现有实现，不新增生命周期路径。
- 宏单元测试覆盖归一化和错误；公开回归使用 tests/short_instance.rs、tests/macro_compile_fail.rs、tests/ui-instance-pass/、tests/ui-instance-fail/、tests/ui-instance-feature/ 和 tests/ui/ 中实际受影响的用例及快照。
- 同步 examples/macro_shortcuts.rs 等实际受影响的当前示例；保留必要旧写法用例以证明兼容，不批量改写历史发行说明或所有旧测试。
- 实施时同步根 Cargo.toml、macro/Cargo.toml、根精确宏依赖及 Cargo.lock；核对其余锁文件，不手改 Cargo 解析记录。独立 Node 与独立烟测夹具保留其已发布精确依赖，不用 path/git/patch 覆盖采用候选。
- 技术指导同步根 README.md/README.en.md、macro/README.md、src/lib.rs 与 macro/src/lib.rs 实际相关 Rustdoc、docs/宏驱动开发.md、docs/instances.md、docs/config-identity.md、docs/architecture.md、docs/README.md 的相关入口，以及新 docs/releases/0.34.0.md；只修改实际受影响内容。

## 版本与兼容性

- 未来实施新增公开宏语法及省略规则：根框架 0.33.0 → 0.34.0，miresidian_macro 0.29.0 → 0.30.0；根精确依赖改为 =0.30.0。
- miresidian_instance、batch、format、lease、codec、ABI crate、开发工具和独立 Node 无实现变化，不因根升版跟升；C ABI v10、租约/资源代理协议 v2 和 MSRV 1.89 保持。
- 已合法的平铺短写与长写继续有效；新增用法从编译失败变为合法，不改变旧合法声明的行为。此前包含裸 multiple 的失败快照须按新契约拆分或修订，不能删除其他非法输入断言。
- format 新增的自动命名与字符串首项同属上述根/宏次版本，不额外创建另一份版本方案；旧合法 selector 声明的名称、默认与槽位保持。原“多格式缺 selector”失败用例改为自动命名成功及名称冲突等针对性断言，不删除未知格式、重复、混用和 feature 门禁检查。miresidian_format 的转换 API 和实现保持，无需跟升。
- 实施提交同步根双语 CHANGELOG 和 macro/CHANGELOG.md，归档实际提交日期及版本；核对包内文档和源码入口。候选未发布前不得把新版本写成可直接安装的独立节点依赖。
- 本次纯方案提交版本影响：无；不修改源码、当前指导契约、依赖、锁文件或产品交付行为，保持现有产品版本，不创建或移动版本标签。

## 实施步骤与验收

1. 开始实施前复核当前版本、工作区及同版本追加需求；若目标版本已变化，先修订本方案。按本文件确定组件升版、精确依赖与文档范围，再修改解析。
2. 实现裸 multiple 默认 config 和 multiple(config/handle)，保留旧短写/长写校验。先用宏单元测试验证新旧策略等价，覆盖两种关联、三种固定策略与三种 selectable 默认值；验证省略执行策略和合法次序。
3. 实现 format 字符串首项解析与多格式自动命名，复用现有名称检查和 Format/生成结构。宏单元测试覆盖全部六种编码、Input/Output、单格式不生成 selector、长短写与自动/显式同名等价、冒号描述剥离、默认项与候选顺序保持。
4. 补充成功及编译失败用例。instance 覆盖空括号、未知关联、多关联、重复模式/关联、嵌套与平铺重复、single 带关联、孤立关联、长短混用、长写缺项及缺少 feature；format 覆盖空格式、单格式附名称、空白/非法名称、名称位置错误、多个名称、两种命名同时出现、格式重复/未知（含 uft8）、新旧格式列表混用、名称冲突及缺少 feature。仅更新确因新规则改变的快照，逐项审核，保留其余非法断言。
5. 通过公开集成测试验证裸 multiple、显式 Config、显式 Handle 的元数据及 init → invoke → close；覆盖同键复用、Config/Handle 定位、默认顺序和 selectable。确认新旧写法的 GetInitConfig 布局及运行结果一致，不新增平台字段；保留 single 在所有执行策略下的宏归一化与现有运行时契约回归。
6. format 公开回归比较自动命名、字符串首项、selector 短写和完整长写的同名元数据、Input 槽位、输出及默认格式；覆盖输出逐项格式、业务 Input 与选择器/多个选择器冲突、同步与 async、格式缺失/空白/非法、固定格式、batch 数据/格式广播与不等长拒绝，以及生成的 JSON ABI 调用路径。用户数据与格式选择仍由现有运行时处理，不改变 ABI。
7. 同步上述版本、示例、指导文档与日期 CHANGELOG。先运行 cargo test -p miresidian_macro --locked，再运行 cargo test -p miresidian --test macro_compile_fail --test format_compile_fail --locked，以及 cargo test -p miresidian --features instance --test short_instance --test macro_compile_fail --test config_instance --test handle_instance --test instance_execution --test broadcast --locked。format 独立和组合分别运行 cargo test -p miresidian --features format --test short_format --test format_feature --test format_compile_fail --locked 与同命令的 --features format,batch 组合；增加 cargo build -p miresidian --example macro_shortcuts --all-features --locked 检查与 instance/lease 的组合。新增集成目标加入对应检查，保留旧语法用例作为兼容证据。
8. 按实际改动执行受影响目标 Clippy、格式检查及严格 Rustdoc。若修改范围超出解析/策略归一或出现未覆盖公共行为，再按编码规范扩大测试；无需为纯语法扩展构建无关工具或声明 Windows DLL/真实硬件验收。
9. 完成产品实施时执行 python3 tools/check_versions.py --base HEAD，通过暂存检查后提交；核对 miresidian-v0.34.0、miresidian_macro-v0.30.0 不可移动标签及 --require-tag。记录实际命令、结果和提交，不推送或发布，除非另有对应授权。

完成标准：目标语法与默认规则全部覆盖；旧合法声明行为及长写必填规则保持；非法组合有明确诊断；受影响文档/版本/锁文件一致；验收证据归档于本文件。

## 本轮记录与后续状态

- 2026-11-17：完成现有解析、约束、文档、目标版本方案检索；未发现根 0.34 / macro 0.30 的既有方案，建立本文件。
- 用户中途要求暂停并讨论 single/parallel，随后明确要求“继续完成计划”；补充上述执行策略与同对象互斥边界，保持本轮仅编写和提交方案的范围。
- 方案检查已通过：三个本地引用文件存在，代码围栏配对，无行尾空白，清单版本仍为根 0.33.0 / macro 0.29.0；python3 tools/check_versions.py --base HEAD 通过。补充 single 边界后须在暂存时重新检查最终文本。
- 本轮仅写入计划；产品实现、组件升版、Rust 编译测试和发行均未执行。提交前执行工作区/暂存版本门禁及 git diff --check，提交时保留正常钩子；提交后核对纯计划未生成版本标签、既有版本标签未移动，提交结果在交付回复报告。
- 首次方案已于 eb3ed20 提交；文档检查、工作区/暂存区版本门禁通过，既有框架与宏标签未移动。纯计划不升版，--require-tag 因 HEAD 没有对应新版本标签而提示失败，未为此创建或移动标签。
- 2026-11-17 追加 format 自动选择器名称与字符串首项简写：沿用同一目标版本，扩充解析设计、兼容/错误契约、受影响文件与验收矩阵。保留原 instance 决策与实施记录；本轮版本影响仍为无，仅更新方案，检查与提交结果在交付回复报告。

## 2026-11-17 实施与验收记录

本节按上方实施决议执行。实现和验证已完成，提交及不可移动标签核验作为交付收尾；不推送、不发布。历史的纯方案状态保留，不代表当前产品实施状态。

| 需求 | 实现与稳定证据 | 结果 |
| --- | --- | --- |
| 裸 multiple 默认 Config，嵌套关联与旧声明兼容 | macro/src/node/instance.rs 策略等价矩阵；config_instance、handle_instance、short_instance 与 UI 正反例 | 通过 |
| 自动名称、字符串首项、具名声明生成同名契约 | macro/src/operation/encoding.rs；short_encoding 的元数据、默认、槽位、同步/async 和 JSON ABI 回归 | 通过 |
| 单编码、非法/重复/混合配置、选择器冲突、缺少 feature | 宏单元；ui-encoding、ui-encoding-feature、ui-instance-fail/feature；已逐项审查新的诊断快照，旧合法用法没有被改成错误 | 通过 |
| batch 与自动/显式选择器组合 | short_encoding、encoding_feature：逐条格式、空项默认、单值广播、不等长拒绝、输出聚合与 async | 通过 |
| encoding 全入口一致，无旧别名 | Cargo metadata 与源代码检查；miresidian_encoding 转换矩阵、自定义 EncodingInput 用例与旧 format 宏入口拒绝 | 通过 |
| 新旧包均执行严格来源约束 | Pack 全组件来源矩阵，Python 版本门禁涵盖 encoding/format 与 instance/instances | 通过 |
| 指导、skill、组件版本与锁文件 | 六个升版组件 README/日期 CHANGELOG、双语根指导、Rustdoc、手册、示例及 0.34.0 迁移；skill 完整五文件副本一致 | 通过 |

实际版本为根 0.34.0、macro 0.30.0、miresidian_encoding 0.5.0、batch 0.3.3、Pack 0.14.2、CLI/套件 0.3.10。Cargo 自动更新的锁文件仅涉及上述组件及旧 format 包名替换，第三方依赖不变。运行时、基础 codec、ABI、instance/lease 实现和独立夹具清单/锁文件没有修改；没有以本地依赖升级独立 Node。

执行结果（Cargo 命令均带 --locked --offline；首次 Cargo 更新锁文件的宏检查除外）：

- cargo test -p miresidian_macro：17 项通过，包含六种编码的固定/多格式归一、名称规则、全部实例执行策略及错误矩阵。
- cargo test --workspace --all-targets：144 项通过，5 项原有忽略。
- cargo test --workspace --all-targets --all-features：271 项通过，6 项原有忽略。
- 独立 instance 的 short_instance/macro_compile_fail/config_instance/handle_instance/instance_execution/broadcast：24 项通过；独立 encoding 的 short_encoding/encoding_feature/encoding_compile_fail：6 项通过；encoding,batch 同目标组合：16 项通过。
- cargo clippy --workspace --all-targets -- -D warnings 及增加 --all-features 的组合：均通过。
- cargo build --workspace --release、cargo build -p miresidian --example macro_shortcuts --all-features：通过。
- cargo +1.89.0 check --workspace --all-targets --all-features：通过。
- RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps：通过；本地根 encoding 模块、miresidian_encoding crate、EncodingInput 和 decode_encoded 页面已生成。
- cargo test --workspace --doc --all-features：2 项通过。
- python3 -m unittest discover -s tools/tests -p test_check_versions.py：30 项通过。
- cargo fmt --all -- --check、git diff --check、python3 tools/check_versions.py --base HEAD：通过；暂存及提交钩子仍须核对最终快照。

忽略项为原有需要 registry/平台环境的 Pack 模板集成测试，以及全特性中的租约子进程辅助入口。没有新增忽略、没有运行 Windows 原生 DLL 加载或真实设备通信，也没有验证候选的 crates.io/docs.rs 发行状态。

提交前审查发现缺 feature 短写夹具曾因输入/输出选择器重名提前报错，已改为不同名称。补充同一组夹具在 encoding 开启时必须编译成功，关闭时必须命中缺失模块/feature 符号的诊断；encoding_compile_fail 两侧重新验证均通过。此收尾只修改夹具和测试入口，工作区源码验证结果仍适用。

skill 审查：仓库与安装副本完整五文件一致，不含 format/FormatInput 等旧能力 API 引用，已有流程要求按目标 Node 的实际版本查官方资料；无需修改或重新绑定 skill 版本。实际技术指导由本次组件文档负责。最终 Markdown 本地文件目标检查 251 项通过。历史发行页只修正因目录移动失效的链接，不改写旧版本行为。

临时原始日志位于 /tmp/miresidian-034-*.log；稳定证据为测试代码、UI 快照及本节命令结果。提交及根/组件标签结果在交付回复报告。

## 2026-11-17 审核补丁：依赖别名门禁与残留注释

用户要求修复再次审核发现的两项问题；沿用本次版本方案，原 0.34.0 实施及标签保留。本补丁为根 0.34.1、encoding 0.5.1、batch 0.3.4；宏、工具、运行时、ABI 与协议保持，不推送、不发布。

- tools/check_versions.py 在 check_node_sources 中按声明的真实 package 名检查全部已知新旧框架包，不再依赖字面键名 miresidian。覆盖 dependencies/dev-dependencies/build-dependencies 及 target 下同类表；workspace = true 按当前清单的 workspace.dependencies 解析，无法解析时明确报错，不以空版本通过。
- 统一要求 =X.Y.Z，并确认 Cargo.lock 包记录中存在所声明的精确版本及合法 crates.io 来源/校验和。不同目标或别名声明不同精确版本时逐条核对，不把合法多版本解析误判为同包只能有一个版本。第三方依赖不强制使用框架精确版本规则，现有 path/git/patch/replace/paths 拒绝保持。
- 移除独立 Node 分支仅按键名 miresidian 检查的重复路径；根工作区内部 crate 仍按自身现有版本规则校验，不更改框架内部 path 依赖政策或独立 Node 清单。
- 用 tools/tests/test_check_versions.py 的真实临时 Git 仓库复现别名范围/缺版本/锁文件不符，补充正确精确别名、开发/构建/target、workspace 继承及不同版本并存用例；保留已有来源与暂存/历史/标签回归。夹具仅用于门禁测试，不构建独立 Node 或声称虚构版本已发布。
- 更正 encoding/src/lib.rs、encoding/src/conversion.rs 和 batch/src/lib.rs 中四处旧 format 能力名；明确通用 ByteTextFormat 与参数名 format 不属于重命名。不改变任何转换实现、trait 或宏契约。
- 先更新上述三组件版本、根精确依赖与 Cargo 生成锁文件，同步各自 README/带日期 CHANGELOG、根双语记录与节点开发手册/版本规范实际受影响内容。Node skill 的版本无关流程不变，完整双副本复核即可，不写入固定 API 或版本表。
- 验证范围：先以新增 Python 回归证明原问题，再运行全部门禁测试；编译/测试 encoding 与 batch 组件、严格 Rustdoc、相关 Rustdoc 示例及版本门禁。运行时和宏实现没有变化，复用 0.34.0 的 144/271 项及工作区 Clippy/MSRV/release 结果，不重复无关全量构建。完成后本地提交，核对根及两组件新标签和既有标签未移动。

### 补丁实施与验证结果

- 已在统一子节点检查中实现真实 package 名解析、精确版本及锁记录匹配，覆盖三类依赖和 target 表，兼容旧 dev_dependencies/build_dependencies 拼写；当前清单的 workspace 继承先解析再校验，第三方范围依赖不受影响。合法的不同版本别名分别匹配锁文件，不要求同包只有一个版本。
- 新增回归先在原实现上复现失败，修复后 python3 -m unittest discover -s tools/tests -p test_check_versions.py 全部 36 项通过，包含别名范围/缺版本/过期或缺失锁记录、目标平台、继承、多版本及第三方对照。
- cargo check -p miresidian --all-features --offline 通过并由 Cargo 更新锁文件；锁差异仅根、encoding、batch 三个版本，无第三方依赖变化。
- cargo test -p miresidian_encoding -p miresidian_batch --locked --offline：8 项通过；RUSTDOCFLAGS='-D warnings' cargo doc -p miresidian -p miresidian_encoding -p miresidian_batch --all-features --no-deps --locked --offline：通过；根 all-features Rustdoc 示例 2 项通过；cargo fmt --all -- --check 通过。
- 四处遗漏注释均已修正；去掉 Rust 注释后与 0.34.0 提交的对应源码逐行一致，宏/运行时/ABI/独立夹具没有修改，故复用其已通过的全量测试、Clippy、MSRV 与 release 证据。
- 受影响 Markdown 的 80 个本地文件目标检查通过；Node skill 完整 5 文件双副本一致，无需改动版本无关流程。python3 tools/check_versions.py --base HEAD 与 git diff --check 通过，最终暂存和提交钩子按正常门禁执行。
- 本补丁未执行推送、发行、Windows 原生加载或硬件验证；提交和 miresidian-v0.34.1、miresidian_encoding-v0.5.1、miresidian_batch-v0.3.4 标签核对结果在交付回复报告，旧标签保持。

## 2026-11-17 推送与发布授权

用户明确要求“推送和发布”，本节取代本任务此前不推送/不发布的执行边界，保留原候选措辞的冻结时点事实。中断时补丁仍暂存，先完成 0.34.1 提交及根/encoding/batch 标签核验，再推送 master 的待推送历史和本任务明确列出的版本标签，不 force、不全量推送所有标签，不升级独立 Node 或创建额外 GitHub Release。

- 本次发行目标为 miresidian_macro 0.30.0、miresidian_encoding 0.5.1、miresidian_batch 0.3.4、miresidian 0.34.1、miresidian_pack 0.14.2、miresidian_cli 0.3.10；官方 crates.io 精确版本预检六者均返回 404。不补发被替代的框架 0.34.0、encoding 0.5.0、batch 0.3.3，不重复发布已有依赖版本。
- 各包从自身不可移动标签的干净 detached worktree 发布：macro/Pack/CLI 使用 2376665 的对应标签，根/encoding/batch 使用补丁提交的对应标签，保持包内 .cargo_vcs_info.json 与其版本标签一致。先核对冻结源码与标签，再按依赖顺序逐包执行正常 cargo publish，保留打包及编译验证，不用 --allow-dirty 或 --no-verify。
- 发布前确认 ABI/codec/instance/lease 精确依赖的发行状态；先发布 macro、encoding、batch，registry 可解析后发布根，独立工具按 Pack → CLI 顺序发布。registry 的成功或错误均以精确版本查询确认，网络中断后不盲目重复上传。
- 发布后查询精确版本及 yanked 状态，下载官方 .crate 验证 SHA-256/checksum、源码修订及非 dirty 状态、原始清单/README/关键 API；核对 GitHub 精确源码入口与 docs.rs 精确 README/API 页面。工具为 binary crate，核对包说明和源码页面，不要求不存在的库 API。
- 用独立临时目录从 crates.io 精确安装 Pack/CLI，并核对 --version 与实际进程调用；该验证不使用本地路径替代新框架依赖。复核完整 Node skill 双副本与推送内容，观察 GitHub CI；记录未完成或外部排队状态，不把上传成功当作在线文档/CI 通过。
- 本地测试结果可复用于未改变的冻结源码；发布需新增正式 Cargo 包验证、registry 安装和在线材料检查，不重跑无关硬件验证。将实际版本、源码/记录提交、标签、包 checksum、在线状态和 CI 结果归档于本文件，发布记录以纯文档提交收尾，不移动发行标签。

## 2026-11-17 正式发布与在线验收结果

已按本次授权完成六包正式发布；crates.io 精确版本均存在且未撤回。master 的产品源码提交为 001d79eea0ac495c02fcab7d25f30a4d8f34a010，macro/Pack/CLI 保持自身 23766652ca9e63cd1adf87d9a92543dc41dd76bb 标签快照。正常 cargo publish --locked 完成各包打包、编译验证与上传，根包增加 --all-features；没有绕过校验或改用本地独立节点依赖。

| crate | 正式版本 | 不可移动标签 | 源码提交 | 官方包 SHA-256 |
| --- | --- | --- | --- | --- |
| [miresidian_macro](https://crates.io/crates/miresidian_macro/0.30.0) | 0.30.0 | miresidian_macro-v0.30.0 | 2376665 | 7ef01f8e223bacf458f534d02436f4e6b8bcd6130aa961ee2dda34389bbd23f9 |
| [miresidian_encoding](https://crates.io/crates/miresidian_encoding/0.5.1) | 0.5.1 | miresidian_encoding-v0.5.1 | 001d79e | 39b7c680070fe323347cd9527ecfb2de34e2ed9f0717e413888d7ca2ef5f8d77 |
| [miresidian_batch](https://crates.io/crates/miresidian_batch/0.3.4) | 0.3.4 | miresidian_batch-v0.3.4 | 001d79e | 3132ba76c3f4fd2827487328c4e1ba037df9b709793e3e4ddfcba1bc2abc63ea |
| [miresidian](https://crates.io/crates/miresidian/0.34.1) | 0.34.1 | miresidian-v0.34.1 | 001d79e | a7fab6b6d59d0a8e194ef92fdd42201065fc81d703658ab77592c7f12ac370d9 |
| [miresidian_pack](https://crates.io/crates/miresidian_pack/0.14.2) | 0.14.2 | miresidian_pack-v0.14.2 | 2376665 | 78a32e8173c72aa73be88ab9ad8b967e8a691433ba9554b28f543cec586f9c71 |
| [miresidian_cli](https://crates.io/crates/miresidian_cli/0.3.10) | 0.3.10 | miresidian_cli-v0.3.10 | 2376665 | 9f5d8ad9f0be88438d02c3fdcb582abe4fd56ba8ae1da795197862fb88169954 |

实际验收结果：

- 逐包下载官方 .crate，SHA-256 与 registry checksum 一致；.cargo_vcs_info.json 的提交、非 dirty 状态和包路径与各自标签一致。原始 Cargo.toml、README 与包内所有可对应的源码文件逐字节匹配冻结树；根包 encoding feature 存在、旧 format feature 不存在，macro/encoding/batch 精确依赖正确。
- 六个精确版本的 docs.rs README 均返回 200，且没有跳转到其他版本；关键 API 和源码入口如下。binary 工具核对 README 与源码，不宣称存在库 API。

| 在线核对项 | 精确版本入口 |
| --- | --- |
| miresidian_macro README | [https://docs.rs/crate/miresidian_macro/0.30.0](https://docs.rs/crate/miresidian_macro/0.30.0) |
| miresidian_encoding README | [https://docs.rs/crate/miresidian_encoding/0.5.1](https://docs.rs/crate/miresidian_encoding/0.5.1) |
| miresidian_batch README | [https://docs.rs/crate/miresidian_batch/0.3.4](https://docs.rs/crate/miresidian_batch/0.3.4) |
| miresidian README | [https://docs.rs/crate/miresidian/0.34.1](https://docs.rs/crate/miresidian/0.34.1) |
| miresidian_pack README | [https://docs.rs/crate/miresidian_pack/0.14.2](https://docs.rs/crate/miresidian_pack/0.14.2) |
| miresidian_cli README | [https://docs.rs/crate/miresidian_cli/0.3.10](https://docs.rs/crate/miresidian_cli/0.3.10) |
| macro attribute | [https://docs.rs/miresidian_macro/0.30.0/miresidian_macro/attr.node.html](https://docs.rs/miresidian_macro/0.30.0/miresidian_macro/attr.node.html) |
| encoding trait | [https://docs.rs/miresidian_encoding/0.5.1/miresidian_encoding/trait.EncodingInput.html](https://docs.rs/miresidian_encoding/0.5.1/miresidian_encoding/trait.EncodingInput.html) |
| encoding decode | [https://docs.rs/miresidian_encoding/0.5.1/miresidian_encoding/fn.decode_encoded.html](https://docs.rs/miresidian_encoding/0.5.1/miresidian_encoding/fn.decode_encoded.html) |
| root encoding | [https://docs.rs/miresidian/0.34.1/miresidian/encoding/index.html](https://docs.rs/miresidian/0.34.1/miresidian/encoding/index.html) |
| batch decode | [https://docs.rs/miresidian_batch/0.3.4/miresidian_batch/fn.decode_batch.html](https://docs.rs/miresidian_batch/0.3.4/miresidian_batch/fn.decode_batch.html) |
| Pack source | [https://docs.rs/crate/miresidian_pack/0.14.2/source/framework-crates.txt](https://docs.rs/crate/miresidian_pack/0.14.2/source/framework-crates.txt) |
| CLI source | [https://docs.rs/crate/miresidian_cli/0.3.10/source/toolset.toml](https://docs.rs/crate/miresidian_cli/0.3.10/source/toolset.toml) |

- 在隔离 /tmp 安装前缀从 crates.io 执行 cargo install miresidian_pack --version =0.14.2 --locked 和 cargo install miresidian_cli --version =0.3.10 --locked，均安装成功。cargo-miresidian-pack --version 为 miresidian_pack 0.14.2；cargo-miresidian 与 cargo-mires 两入口均为 miresidian_cli 0.3.10。加入该前缀 PATH 后调用 cargo-miresidian check --help，真实 CLI → Pack 子进程转发返回 0，并输出 Pack 用法。未修改用户全局安装。
- GitHub 远端 master 与产品提交一致；六个发行标签的 annotated tag 对象及 peeled commit 与本地冻结标签一致。Node skill 的主文档、引用、UI 与维护规则完整五文件和已安装副本、产品提交冻结内容逐字节一致；远端相同提交哈希保证同一内容已推送。
- 产品提交的两套 GitHub Actions 均 completed/success：[独立工具套件](https://github.com/MyMires/miresidian/actions/runs/43992243641)；[CI](https://github.com/MyMires/miresidian/actions/runs/43992243648)。
- 同批推送了历史 miresidian-v0.34.0、miresidian_encoding-v0.5.0、miresidian_batch-v0.3.3 标签，仅保留已提交历史，不补发这三个被替代版本；已有 ABI/codec/instance/lease 版本没有重复发布。未创建额外 GitHub Release。
- 发行包 README 中的候选说明保留其源码冻结时点；当前 registry、在线文档和安装的实测状态以本节为准。发行后不为改写候选措辞而重写标签或修改已发布包。

本节与工具方案的结果记录为纯文档变更，版本影响：无；不改变产品源码、依赖、API、默认值或交付契约，不创建或移动版本标签。产品快照此前的测试及本节新增发行验证均保持适用；记录提交采用 [skip ci]，沿用上述产品提交的 CI 结果。记录提交仍执行工作区、暂存区与历史版本门禁；--require-tag 在产品发行 worktree 核对，记录 HEAD 不绑定新产品标签。记录提交哈希及最终远端一致性由 Git 历史和交付回复确认。
