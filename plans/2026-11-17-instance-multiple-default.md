# 框架 0.34 / macro 0.30：实例与格式宏简写

日期：2026-11-17。

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
