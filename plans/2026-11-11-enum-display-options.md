# 2026-11-11 枚举实例驱动的平台选项与输入解析

## 2026-11-11 追加任务：枚举成员显示值长短标注（当前执行方案）

本节针对前述版本完成后的新增需求，独立于下文已完成的发布工作。用户要求在 NodeEnum 枚举成员上声明显示值，并明确要求简写；据此实施、验证及按仓库规则本地提交/标签。本次不推送、不发布，不改独立 Node、skill 或工具版本。

### 目标、设计与契约

- `#[node_enum(display = "空闲")]` 与 `#[node_enum("空闲")]` 等价，只能标注无字段枚举成员，每个成员至多一个显示值；未标注时仍以去除 r# 的成员名同时作为实际值与显示值。允许单个末尾逗号。
- 在 macro/src/lib.rs 注册 derive helper，在 macro/src/node_enum.rs 解析；只改变生成的 EnumOption.display。实际值、NodeInput 解析/规范化和 NodeOutput 输出仍使用成员名，数字 discriminant 不参与。无需新依赖，不读源码文件，不增加反射。
- 空白显示值、未知键、非字符串、重复/混用长短写、多余参数和枚举级误用在编译期拒绝。显示值原文保留；TB 分隔符、首尾空白、重名或与其他实际值冲突仍由 TB 适配器在发布候选/规范化时校验，不把平台语法限制加进通用派生。成员 cfg/cfg_attr 继续按实际启用结果生成。
- 参数仅写名称即可自动得到显示值候选；已有手写非空描述优先。TB 将显示值映射为实际值后再解析；其他平台使用相同结构化元数据决定自己的格式与输入。覆盖 Config、初始化/探测/业务 Input、Option/Vec、batch 和多通道槽位。直接 Rust codec 仍仅接收实际值。
- 资源所有权、状态、并发、取消及生命周期不变；新增内容仅为编译期字符串元数据和既有平台转换链。C ABI v10、配置租约/资源代理 v2、MSRV 1.89 保持。

### 文件与版本评估

- 新增公开宏能力：根 miresidian 0.30.2 → 0.31.0，miresidian_macro 0.26.0 → 0.27.0。根精确依赖及 Cargo.lock 同步；其他组件没有实现/契约变化，不跟升。独立夹具和相邻 Node 锁文件仍依赖原已发布版本，不改写。
- 实现：macro/src/lib.rs、macro/src/node_enum.rs。测试：新增 tests/enum_display.rs 与 tests/ui/node_enum_display_invalid.rs/.stderr，沿用现有 enum_codec/enum_platform/enum_manual_config 和 macro_compile_fail。
- 示例及真实 ABI：更新内部 examples/enum_platform_dll.rs、tools/verify_enum_dll.py，将长短两种标注贯通 TB 与另一平台的显示值输入和实际值输出；该脚本是内部验证夹具，不是独立发布工具。
- 指导与归档：根双语 README/CHANGELOG、macro README/CHANGELOG/Rustdoc、docs/宏驱动开发.md、docs/c-abi.md、docs/releases/0.31.0.md；同步实际新语法、边界、迁移和未发布源码状态。

### 步骤与验收

1. 方案落盘后先修改两个实际组件版本及根精确依赖，再实现 helper 解析和结构化显示值生成。
2. 添加集成与 UI 回归：长短写/未标注等价元数据、直接 codec 实际值边界、TB/自定义 JSON 双平台候选和输入、同步/异步及 Config/Option/Vec/batch/多通道、非法语法诊断、cfg 成员与平台分隔符校验。
3. 更新内部 DLL 示例和验证脚本、双语 README/日期 CHANGELOG、公开 Rustdoc、节点手册/平台契约/迁移说明，生成工作区锁文件并核对无第三方依赖漂移。
4. 只做相关验证：根默认与全 features 的枚举集成测试、宏单元/UI 编译测试；根与宏相关目标严格 Clippy、Rustdoc/示例；一次内部 debug DLL 真实加载检查。复用无关运行时/租约已有验证，不做全工作区 Release 或其他操作系统/硬件验证。
5. 格式、diff 和版本 base/staged 门禁通过后提交，核对根与宏独立不可移动标签及 require-tag；在本节记录实际证据，最终报告版本、提交、标签和检查结果。

### 本次执行与验收证据

- 实现完成：NodeEnum 注册 helper 并同时解析长写/简写，仅改变 display 元数据；默认成员、数字 discriminant、手写非空描述、条件编译和直接 codec 的实际值边界均保留。新增公开能力对应根 0.31.0、macro 0.27.0，Cargo.lock 仅这两个版本记录变化，其他锁文件及独立 Node 保持原依赖。
- 默认与全 features 的 enum_display、enum_codec、enum_platform、enum_manual_config 各 10 项通过；补充 cfg_attr 显示名后对应两种配置的 enum_display 各 3 项再次通过。覆盖长短写、默认值、Config/初始化/探测/同步/异步 Input、Option/Vec/batch/多通道、双平台候选与显示值输入、手写描述优先和平台歧义/分隔符校验。
- 宏单元测试 15 项通过，含长短写末尾逗号及平台文本原文保留；默认和全 features 宏 UI 集合通过，新增用例锁定 10 类非法显示值标注诊断。全 features 首次使用 TRYBUILD=overwrite 生成新快照，审查确认未改旧快照；默认配置不设置 overwrite 验证快照通过。追加 cfg_attr 用例时格式检查发现多余括号，修正后全部相关检查通过。
- 宏 all-targets 与根相关 lib/四个枚举测试/UI 驱动/内部示例的全 features Clippy 均以 -D warnings 通过；根与宏全 features Rustdoc 以 -D warnings 通过；根 2 个 doctest 通过。
- debug 内部 enum_platform_dll 已实际构建并由 tools/verify_enum_dll.py 加载通过：ABI v10 单 JSON 导出、默认/显式平台、长短标注生成的中文候选、TB/另一平台的显示值转实际值及错误请求均验证。不声称 Windows、其他架构或硬件验收。
- 根双语 README/日期 CHANGELOG、宏 README/日期 CHANGELOG/Rustdoc、节点手册、平台契约、迁移说明和可运行示例同步；6 份变更指导的 118 个本地文档链接有效。版本 base 门禁、diff 检查通过；提交前再执行格式和 staged 门禁，提交后执行根/宏 require-tag，结果在交付回复报告。
- 本次实现与上述验收已完成；不推送、不发布，不重复全工作区 Release、无关租约回归或独立 Node 构建。版本提交/标签的最终证据为本次 Git 提交及不可移动标签。

状态：用户已明确授权 NodeEnum 派生宏，并要求平台由 DLL 调用确定。以下最新修订为唯一有效实施方案；更早草案和撤回记录保留，不作为执行依据。
执行依据：用户最新明确选择 #[derive(miresidian::NodeEnum)]，并要求“转换要根据平台适配来进行，配套的，平台适配不该让用户在运行时决定，而是直接由在调用 dll 时就决定”。授权实施、验证及本地版本提交/标签；不推送、不发布。

## 2026-11-11 验证范围收敛（最新执行约束）

用户质疑本次耗时及过度验证后，停止扩展和重复整套验证。已完成默认工作区 128 项、全特性工作区 252 项通过，直接复用；不再重跑。旧平台槽位/旧 ABI 测试迁移中的失败及修正如实保留在最终结果，不把每次排错视为新的验收要求。

剩余只做：一次全特性严格 Clippy、四个实际公开接口组件的严格 Rustdoc、一个 debug 内部 DLL 的真实单 JSON/双平台枚举加载检查，以及格式、文档链接和提交版本门禁。没有新依赖或 MSRV/工具链变化，不重复 MSRV 构建，不做全工作区 Release 编译、其他 DLL 模拟或额外平台测试。本节覆盖下文更早的扩大验证清单；交付不声称已执行被取消的检查。

## 2026-11-11 追加：唯一 JSON 调用入口（覆盖下文旧入口保留条款）

用户进一步明确“abi 只需要 miresidian_invoke_request 这个了，其他调用函数都不需要了，将这个改名为 miresidian_invoke”。本次按同一 ABI v10 实施，不另设过渡 ABI。

- 唯一业务调用导出为 const char *miresidian_invoke(const char *request_json)，接受完整 JSON 的 function/config/input/platform；移除原三参数 miresidian_invoke、四参数 miresidian_invoke_with 和旧名 miresidian_invoke_request，不提供别名。
- miresidian_abi_version 是独立版本查询，保留供宿主在调用前区分同名旧/新签名。头文件只声明这两个导出；旧宿主不能直接用旧函数指针调用 v10。
- macro/src/node/ffi.rs、src/interop/ffi_runtime.rs 统一为单请求入口；删去分参数 JSON 数组解析及旧分发 helper。请求解析/错误字段归 request_json，请求对象约束、响应字段、缓存与所有权保持。
- tests/c_abi_smoke.c 改为单参数函数指针，FFI 测试通过真实 JSON 请求调用，验证旧符号不存在；其他并发/租约/配置业务回归可用明确的测试专用 JSON 构造 helper，不能在生产代码保留旧 ABI 包装。
- 更新本仓库原生 DLL 验证脚本为 v10 单请求函数签名，新增内部 examples/enum_platform_dll.rs 及 tools/verify_enum_dll.py，实际加载测试本地库的导出、派生枚举候选及所选平台输入转换。独立旧 Node/夹具的 API 和已发布依赖不改。
- Windows TestFlow 执行器需在调用前查询 ABI，以正确调用旧 v7-v9 三参数或新 v10 单参数入口；支持旧已发布节点是工具兼容能力，不是 v10 DLL 保留旧导出。工具独立 0.12.0 → 0.13.0，同步 README/日期 CHANGELOG 与宿主模拟/生成器测试，不声称 Linux 上验证了 Windows 原生执行。
- CLI/套件 0.3.4 → 0.3.5，内嵌 TestFlow 最低版本同步 0.13.0，Pack 仍为 0.13.4；同步指导和日期归档。不为本次候选修改仅支持旧 ABI v7 的 Pack 模板，README 明确其边界。
- 其余版本沿下文：根 0.30.0、macro 0.26.0、codec 0.18.0、ABI 0.17.0/v10，相关 codec 依赖组件补丁同步；精确依赖及锁文件重新核对。新版本安装与发布边界保持未发布候选。
- 为避免旧 Config 选择器辅助 API 被误用，本次同时删除 PlatformRegistry::selector_definition、NodeDescriptor 两个 *_with_platform 原始描述 helper 及 PLATFORM_ADAPTER_DEFINITION；保留通用注册表查找和诊断名称。平台元数据统一使用显式适配器的 *_for 方法，原始无平台描述 helper 不自动渲染候选。
- 当前指导需把三入口叙述改为唯一 JSON 入口，并解释 v9→v10 的签名、默认平台、Config 布局迁移；旧发行归档仅链接，不改写。后续实施与验收同时覆盖本追加内容。

## 2026-11-11 最终修订：派生 codec 与调用时平台适配（当前有效）

### 范围与接口

- 枚举保留标准 Rust 定义，仅加用户选择的 NodeEnum 派生；不增加成员标签、参数 options 列表、定义宏、模块标签移动或业务类型的框架反向依赖。
- 派生读取非空、无泛型的纯无字段枚举，按声明顺序生成 NodeInput、NodeOutput、NodeCollectionElement 及候选。普通成员名（去掉 r#）为实际/默认显示值；整数 discriminant 不参与。条件编译成员的候选和分支一致；携带数据、结构体成员、空元组、空枚举、泛型、非枚举输入编译报错。
- codec 提供 EnumOption { value, display } 及 NodeInput::OPTIONS / NodeCollectionElement::OPTIONS，默认 &[]；Option/Vec 转发候选，不收紧原有集合约束。纯枚举 codec 只处理实际值和标准集合文本；未知、空白和完整候选文本不回退首项，规范值为成员名。派生不要求 Clone/Copy/Default。
- ParameterDescriptor 新增 enum_options 静态切片，原 definition 保留。普通参数从原 Rust 类型读取候选；format 业务转换参数和输出描述不额外要求 NodeInput，候选为空。已有非空冒号后缀优先用于平台展示；enum_options 仍保留类型信息，供相同平台配套解析使用。业务 default_value 仍 None。
- 平台适配器新增 format_parameter 与 normalize_parameter 两个配套方法。前者生成当前平台描述，后者把该平台的枚举输入转换成 codec 接收的规范实际值/集合文本。无枚举的默认实现保留原文；平台遇到枚举而未实现支持时明确 NotSupported，不静默继承 TB 格式。
- TB 负责 名称:实际值-显示值/实际值 生成、手写描述的已有显示值映射、名称前缀和候选值规范化；显示值相同时只写一次。自动生成时校验 TB 保留分隔符、空白与候选值冲突；通用 EnumOption 不含 TB 语法约束。其他平台可以实现完全不同的候选和输入表示。
- 参数生成/codec 不拼接平台候选格式。新增显式平台描述方法 definition_for/signature_for/function_signatures_for/init_config_definitions_for；已有不带平台的描述方法保留为原始元数据查询，不自动猜测平台。

### 平台在 DLL 调用时确定

- miresidian_invoke_with 的非零 platform 与完整请求中的非零 platform 明确指定已注册平台；0 或省略选择注册表的固定默认平台。miresidian_invoke 无平台字段，固定使用默认平台。
- 不再发布、消费或保留 InitDevice 中的“平台适配”选择器。NodeRuntime 默认入口同样固定使用默认平台，不自动沿用之前自定义平台；若实例存活且本次平台不同，仍按原绑定约束报错。0 不触发用户选择，也不表示“继续使用上一次选择”。
- C ABI 升至 v10，ABI crate 0.16.0 → 0.17.0。常量更名为 MIRESIDIAN_PLATFORM_DEFAULT（0），同步 Rust/头文件/元数据与调用说明；现有 C 函数签名、稳定平台编号、JSON 结构、内存有效期保持。v9 的 Config 平台选择用法必须迁移，不能宣称完全兼容。
- 删除根运行时的 selector 布尔传递和 Config 平台选择解析，调用时已决定的平台贯穿元数据、初始化与业务调用。旧平台注册表按标识符查找/原始描述辅助方法可以保留用于显式检查，但实际 DLL 查询与执行不走用户 Config 选择。
- 历史 ConfigLayout.platform 字段仅作为执行方式起始索引保留，文档明确此处不再占用平台参数；数值布局保持，避免无关实例 API 更名。framework “平台适配”名称常量只用于旧显式描述辅助/诊断，不发布选择器。

### 调用与资源链路

1. DLL 入口确定具体平台；适配器 decode 处理传输表示。
2. 运行时用同一个适配器和原始类型元数据规范化 Config、初始化 Input、探测 Input、普通 Input 及多通道各槽位中的枚举输入；只处理 enum_options 非空的参数。基本类型和未声明枚举元数据的旧自定义类型沿用既有路径。
3. GetFunctions/GetInitConfig 使用本次真实适配器渲染，不能仅按编号从注册表取另一个对象；直接 invoke_with 的未注册适配器也生效，无首个平台查询的缓存污染。
4. 规范化输入交给现有 codec/宏桥，类型配置身份与租约编码仍复用 NodeInput::normalize_text，保证显示值/实际值定位同一对象。显示值表由手写参数描述提供时原样保留，平台先映射再交给纯枚举 codec。
5. 原生 invoke_command 接收已规范化的结构化命令；元数据查询使用已绑定/默认注册适配器。直接 Rust 调用业务方法仍使用 Rust 枚举值。
6. 直接 invoke_with 参数改为 &dyn PlatformAdapter，普通引用调用无需变动；旧显式 turbofish 调用需移除。适配器借用只用于前置 decode/规范化/元数据，后台执行只携带既有平台标识，不保存借用或新增共享可变状态。
7. 元数据/候选均为静态只读数据，每次渲染返回独立字符串；不新增线程、I/O、取消或租约流程。共享节点契约签名纳入操作的结构化候选，防止候选不同但裸描述相同的节点静默共用；租约与资源代理线协议仍为 v2，旧/新命名契约不一致时正常拒绝共用。

### 文件和版本

- 根 0.29.0 → 0.30.0：descriptor、platform、TB、service/routing/management 生命周期平台传递、共享契约、ffi_runtime、root 重导出及相关测试。
- codec 0.17.0 → 0.18.0：新 options.rs、默认候选常量和集合/可选值转发；必要的结构候选规范化 helper 不解析平台描述。
- macro 0.25.0 → 0.26.0：新 node_enum.rs、公开派生入口、参数候选元数据生成；复用现有 cfg 及框架依赖重命名辅助，不扫描源文件。
- ABI 0.16.0 → 0.17.0 / v10：lib、header、Cargo metadata、组件测试及指导。
- format 0.4.0 → 0.4.1、batch 0.3.0 → 0.3.1、instances 0.16.7 → 0.16.8、lease 0.17.2 → 0.17.3：分别同步已有 codec 精确依赖；instances 额外修正文档中的旧平台槽位说明，lease 不改线协议。
- 内部精确依赖与工作区锁文件由 Cargo 同步；第三方依赖、独立节点/烟测夹具、工具及 skill 保持。Pack 直接复制所选节点的 ABI 头文件、不内嵌本仓库头文件，本次不跟升；既有工具未支持新候选 ABI 的边界如实说明，不虚构支持。
- 各升版组件 README、Rustdoc、带日期 CHANGELOG（已有双语则同步），根双语指导/归档、节点开发手册、平台/C ABI、实例 Config 布局、架构及 docs/releases/0.30.0.md 同步。旧发行归档不重写；当前文档旧平台选择描述需逐处核对并改成 DLL 调用选择。

### 实施和验收

1. 本修订落盘后启用现有钩子，先更新实际组件版本，再实现 codec 元数据、派生和参数描述字段。
2. 实现平台成对渲染/规范化与显式平台描述入口，再接入运行时，确保业务 codec 不依赖平台字符串。同步手写描述测试字段。
3. 改成调用时确定平台，删除 Config 选择逻辑和 selector 传递，同步 ABI v10、0/省略默认语义和旧平台测试；旧 Config 选择项明确报错，不偷偷当作执行策略。
4. 测试同一业务枚举在 TB 与另一种自定义平台的不同候选格式、不同输入表示及实际业务结果，覆盖默认/0/显式编号/直接未注册适配器、查询前后无缓存串扰、未实现枚举平台明确失败；确认没有“平台适配”Config。
5. 覆盖无后缀/空后缀自动、手写默认/候选原样、Config/初始化 Input/探测/同步/异步、别名/限定路径/Option/Vec/batch、实际/显示值身份一致、非法值不执行业务、空值/缺参不默认首项、条件编译、派生不支持形状 UI。自定义 FormatInput 不额外要求 NodeInput。
6. FFI 实际请求覆盖自动候选、平台输入规范化、v10/默认0、旧 Config 拒绝、参数错误/未知平台和共享响应缓存；现有生命周期/租约/并发测试继续通过。
7. 完成文档/版本/锁文件，执行默认/全特性工作区测试、严格 Clippy/Rustdoc/文档示例、Rust 1.89 编译、必要 Release 编译、链接/格式/残留和版本门禁。确认公开 Device 类型未重新引入，独立依赖未绕过 registry。
8. 通过 base/staged 检查后提交，创建并核对根 miresidian-v0.30.0 及实际升版组件标签；已有标签不移动。报告版本、提交、标签和实际验证，不推送/发布，不声称 Windows/真实硬件验证。

---

以下均为已被最新修订替代的历史讨论与撤回记录。

## 2026-11-11 更正：不增加任何枚举标签（当前约束）

- 不增加 NodeEnum、成员显示名或参数候选列表标签；普通参数应继续走现有统一参数转换入口，平台格式仍归适配器。
- 复核现有代码：node 宏只接收一个 ItemStruct；operation 宏只接收一个 ImplItemFn，二者均无法从参数类型路径取得任意外部枚举定义。基础类型之所以可转换，是 codec 已提供 NodeInput 实现，而不是 Rust 自动为所有类型生成转换。
- 在保持现有宏作用范围、不读取源文件、不要求任何类型实现的条件下，普通 Rust 枚举没有稳定反射可供列举成员或构造任意成员，不能声称已经可以无标签自动实现。
- 能拿到枚举定义的框架生成入口才可生成同源转换，例如让既有 node 标注覆盖模块；这会改变现有标注作用位置，尚未获得用户对该结构调整的选择，不能擅自实施。另一类生成入口也必须明确其构建/布局影响，不能以隐蔽的源码扫描替代设计说明。
- 本轮仅实施了版本/精确依赖预备更新、EnumOption/OPTIONS 元数据草稿及不存在的 NodeEnum 重导出，用户中断时尚未实现派生或平台渲染。现在撤回这些本轮未提交的预备修改及新建 options.rs，恢复已提交 0.29.0 的可构建状态；保留本方案记录，不覆盖之前完成的公开类型更名提交或标签。
- 撤回后验证 git diff --check、版本门禁及框架默认编译，不进行无关全量验证；本轮无实现提交/新版本/标签/发布。后续设计须先解决无新标签时的类型定义可见范围，再修订目标、文件、版本与验收后实施。

---

下方 NodeEnum 派生与更早 options 草案均已失效，仅作讨论记录。

## 2026-11-11 修订：类型候选与平台渲染分离（已撤销派生入口）

### 目标与关键决策

- 用户后续已选择简单的无数据枚举：外部参数没有非空默认/候选描述时，根据 Rust 枚举成员自动生成候选；已有非空描述原样保留，同时支持原有实际值/显示值映射。
- 不实施原草案的参数级 options(...)、带字符串载荷实例或固定 TB 字符串生成器。沿用已完成的 NodeInput/NodeOutput 等通用命名，不重新引入 Device 类型。
- Rust 不能反射任意类型；新增一次性的 #[derive(miresidian::NodeEnum)]，从枚举定义生成结构化候选、NodeInput、NodeOutput、NodeCollectionElement 实现。参数仍只写现有 input/config 名称，不重复成员，也不新增成员显示名属性。
- 派生仅支持非空、无泛型的无字段枚举，按声明顺序使用去除 r# 的成员名作为实际值和默认显示值；显式数字 discriminant 不作为平台实际值。携带数据、多字段、空元组、结构体成员、空枚举或泛型在编译期清晰拒绝。条件编译成员对应的元数据与 match 分支同步启用。
- 业务元数据只存结构化值，不在 codec 或宏中拼接冒号、斜线、连字符等平台语法；渲染发生在每次查询所选平台的适配器中，不缓存第一次平台生成的字符串。

```rust,ignore
#[derive(miresidian::NodeEnum)]
enum ReceiveMode { IdleTimeout, Terminator, Length }

// 无非空后缀时自动候选；名称、空冒号后缀、空白后缀均适用。
#[miresidian::input("接收方式")]
mode: ReceiveMode,

// 显式描述原样保留，已有 codec 把显示值映射为枚举实际值。
#[miresidian::input("接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length-固定长度")]
mode: ReceiveMode,
```

### 接口、职责和调用链

1. codec 新增 EnumOption { value: &'static str, display: &'static str }，及 NodeInput::OPTIONS / NodeCollectionElement::OPTIONS 关联常量，默认空切片；旧自定义输入/集合实现无需补写常量。Option<T> 转发输入候选，Vec<T> 从元素集合契约转发候选，不收紧既有 Vec 的 trait 边界。
2. 派生生成 NodeInput::parse_text / normalize_text：去除首尾空白后按成员名精确匹配，区分大小写；未知/空白返回现有 InvalidArgument，规范值恒为成员名。生成 NodeOutput::into_output 输出实际值；集合实现复用字符串集合转义语法逐项解析/格式化，不要求 Clone/Copy/Default。batch 使用已有集合/调用逻辑，format 仍须自己的 FormatInput/FormatOutput，不隐式转换。
3. ParameterDescriptor 新增 enum_options: &'static [EnumOption]。参数宏仅在描述无非空冒号后缀、非 format 业务转换参数、非框架显式默认选择器时读取 <T as NodeInput>::OPTIONS；有手写默认/候选描述或输出标签时为 &[]。保留原始 definition，业务 default_value 仍 None。普通类型默认无候选，不改变既有描述。
4. PlatformAdapter 新增 format_parameter(&ParameterDescriptor) -> MiresidianResult<String>。默认实现对无自动枚举候选返回原始 definition；发现自动枚举候选时返回 NotSupported，要求该平台明确实现渲染，禁止无声套用 TB 语法。
5. TB 实现负责自动候选的 名称:实际值-显示值/实际值 语法，显示值相同只写一次。平台渲染时校验空值、首尾空白、候选实际/显示值冲突与 TB 保留分隔符；这些约束不放进通用枚举派生，其他平台可定义不同表示。手写描述保持原样，不新增其语法校验。
6. ParameterDescriptor::definition_for、OperationDescriptor::signature_for、NodeDescriptor::function_signatures_for / init_config_definitions_for 提供显式平台渲染入口，错误原样传播。已有不带适配器的 definition/signature/function_signatures 等方法保留为原始描述查询，不自动生成任何平台候选，文档明确两者差异。
7. NodeRuntime 的 GetFunctions/GetInitConfig 使用本次真实适配器调用平台渲染：显式编号、默认/零值、已选平台、invoke_with 均遵循实际适配器对象；不能仅凭编号重查注册表，因为 invoke_with 支持未注册的调用方适配器。command_async 为元数据带入可选适配器借用，结构化 invoke_command 在元数据查询时使用已选或默认注册适配器。版本查询不依赖候选渲染。
8. 为直接传递调用方适配器，invoke_with 的参数类型由泛型 &A 改为 &dyn PlatformAdapter；一般调用写法不变，旧显式 turbofish 调用需移除。此 Rust 源码变更纳入根次版本及迁移说明，不改变其平台选择和业务调用语义。适配器借用仅用于同步元数据生成，不被保存或带入后台任务；执行中的平台身份仍通过既有值传递。
9. 原始/结构化候选均为静态只读数据；每次渲染返回独立 String，平台查询之间无共享可变缓存，不新增线程、I/O、租约或动态资源。错误发生在返回元数据或执行业务前，不改变对象生命周期、超时/取消和清理。

### 默认、显示值与兼容性

- 候选列表不是业务缺参默认值；不取首项、不调用 Default、不改变缺参/Option 空白/Vec 空集合和多实例空身份提前跳过规则。
- 显式的 名称:默认值 或 名称:实际值-显示值/... 优先；原始描述字节及既有显示值规范化、集合转义、配置身份/租约纯规范化链保持。带数据枚举仍可手写 NodeInput，但不能使用本派生。
- 自动列表默认实际值与显示值相同；要中文显示值可以继续写现有参数描述。手工 NodeInput 实现也可通过 OPTIONS 提供结构化实际/显示值，但需自行保证 parse_text/normalize_text 与它们一致，不假定元数据能替代业务解析。
- 新增结构字段需要现有手写 ParameterDescriptor 补 enum_options: &[]；宏使用者不手写此字段。现有手写平台适配器无自动枚举时继续工作，有自动候选时需覆盖新渲染方法。
- 生成格式的平台绑定限定本次新增自动枚举候选；既有框架平台选择/执行方式/租约选择器及函数签名外层语法不在本次重构范围，仍按既有契约处理，文档不能声称已实现所有元数据的任意平台序列化。
- C ABI 导出、JSON、内存所有权与 ABI v9、租约/资源代理协议 v2、MSRV 1.89 均不变。不修改 abi/api/miresidian.h，不升级独立 Node/烟测夹具或 skill，不使用本地覆盖；新版本均先作为未发布源码候选。

### 文件与版本评估

- 根框架 0.29.0 → 0.30.0：新元数据字段、平台渲染 API、runtime 真实平台路由、公开重导出；src/runtime/descriptor.rs、src/platform.rs、src/platform/tb_lv_test_platform.rs、src/runtime/lifecycle/service.rs、src/lib.rs；对应手写描述测试、平台/FFI/枚举集成测试。
- codec 0.17.0 → 0.18.0：EnumOption 结构、两个 trait 的默认关联常量及 Option/Vec 转发；codec/src/lib.rs、新增 codec/src/options.rs、组件测试与 Rustdoc。
- macro 0.25.0 → 0.26.0：新增 macro/src/node_enum.rs 负责枚举校验和派生、lib.rs 入口和参数元数据生成；使用已有 framework::rewrite_paths，支持框架依赖重命名，不扫描源文件或共享宏展开状态。
- format 0.4.0 → 0.4.1、batch 0.3.0 → 0.3.1、instances 0.16.7 → 0.16.8、lease 0.17.2 → 0.17.3：只同步已有 codec 精确依赖至 0.18.0，API 和实现不改；各自 README/日期 CHANGELOG 独立说明。其他 crate 和工具不升版。
- 根精确依赖同步实际组件版本；Cargo 生成工作区锁文件并核对其他锁文件不变。各升版组件 README、Rustdoc 与带日期 CHANGELOG，双语根指导/CHANGELOG、节点开发手册、平台扩展/C ABI 手册、架构及新 docs/releases/0.30.0.md 同步。平台扩展的权威契约在现有 C ABI 手册，不散落重复定义。
- 如果执行中发现额外公开契约/版本/职责变化，先修订本文件再实施，不事后掩盖偏离。

### 步骤与验收

1. 本修订先落盘；启用已核对的 .githooks。实现 codec 结构候选/默认关联常量，再实现枚举派生与参数静态元数据字段；更新本仓库手写 ParameterDescriptor，不碰独立项目。
2. 实现平台渲染默认契约、TB 实现和显式平台描述方法，接入实际调用适配器，包括不在注册表中的 invoke_with 对象；保持版本查询与业务执行边界。
3. 集成测试同一派生业务枚举在 TB 与不同语法的自定义平台生成不同候选；覆盖默认/零值/显式平台、直接对象、选型后查询、先查询一种平台再查询另一种平台无缓存串扰，以及未实现平台明确报错。
4. 覆盖 Config/Input/初始化 Input/探测和普通同步/异步、类型别名、限定路径、Option/Vec、batch、自定义实际/显示值、原显式描述不覆盖、基本类型不生成候选、缺参/未知/完整候选文本拒绝、配置实际/显示值身份一致、条件编译成员一致。UI 覆盖带数据/空/泛型/结构体/空元组拒绝，并复用现有 trait 冲突诊断。不依赖具体通信硬件。
5. TB 平台测试候选保留字符与交叉冲突报错，手写描述无改写，纯结构候选可由其他平台表达。FFI 请求测试由真实查询返回自动候选，并检查 ABI v9、错误响应和平台选择路径。
6. 完成组件版本、精确依赖、锁文件、日期归档和语义文档。运行默认/全特性工作区测试、严格 Clippy、严格 Rustdoc/文档示例、Rust 1.89 编译、必要 Release 编译、文档链接/格式及版本门禁。确认不引入 Device 公共类型，不修改 ABI/协议或独立依赖。
7. 通过 --base HEAD、staged 门禁后本地提交；核对根 miresidian-v0.30.0 及所有实际升版组件标签和 --require-tag，验证既有标签不移动。归档实际验收，明确无 Windows 原生宿主及真实硬件验证；不推送、不发布。

---

## 原草案（已被上述修订替代，保留设计讨论记录）

## 目标、现状与依据

- 使用 Rust 枚举实例本身携带显示文字，同时支持不带数据的成员；不要求成员显示名标签、枚举派生宏或手写 DeviceInput。
- 一份候选声明同时生成平台描述、输入校验与枚举构造、配置身份规范化，避免三处维护映射。
- 当前源码：根框架 0.28.0、macro 0.24.0、codec 0.16.0、C ABI v9。业务描述字符串原样发布，宏生成的业务 default_value 为 None；自定义枚举目前由节点实现 DeviceInput。
- 接入点：macro/src/operation/{config,format,definition,descriptor,dispatch}.rs；descriptor 保存静态字符串，init_config 等仍为关联静态常量。
- Rust Reference：枚举构造规则 https://doc.rust-lang.org/reference/items/enumerations.html；过程宏输入边界 https://doc.rust-lang.org/reference/procedural-macros.html 。本方案基于宏输入的语法生成代码，不读取用户源文件、不解析 Debug 输出、不声称具有类型反射。
- 首版范围为标量枚举 Config/Input，包括初始化、探测、关闭和普通同步/异步方法。数组、Option、枚举输出、动态候选、batch 枚举和 format 组合不在本次范围；既有自定义 trait 路径继续可用。

## 声明形式与关键选择

```rust,ignore
enum DataFormat {
    ASCII,
    HEX(&'static str),
    UTF8(&'static str),
}

#[miresidian::operation("选择格式")]
fn choose(
    &mut self,
    #[miresidian::input("数据格式", options(
        DataFormat::ASCII,
        DataFormat::HEX("十六进制"),
        DataFormat::UTF8("文本"),
    ))]
    format: DataFormat,
) -> miresidian::MiresidianResult<()> {
    // 业务使用 format。
    Ok(())
}
```

- Config 同样使用 options(...)，可以与初始化 instance(key/ignore)、lease(key/ignore) 角色组合。options 不改变 Config/Input 来源或索引，不新增输入槽位。
- 每项只接受限定成员路径（至少 Type::Variant）或限定成员路径加一个字符串字面量的构造表达式；路径保留用于 Rust 构造，末段标识符去除 r# 后作为实际值。
- 无数据项的显示值等于实际值；带数据项的字面量是显示值。相同的实际值与显示值输出一次，否则输出 actual-display。顺序严格来自候选声明。
- 枚举可以定义更多成员；本参数只接受列出的候选。同一成员不可列出多个显示值；别名与跨版本稳定实际值的独立配置另行设计，成员重命名会改变平台实际值。
- 支持模块限定路径和类型别名；不推断类型定义、不遍历未声明的枚举成员。生成的 Rust 构造表达式赋值给声明参数类型，由 rustc 验证路径解析、字段形状、字段类型与参数类型。宏只保证语法契约，类型别名等价性由编译器判断。
- 在生成代码中增加不可执行的模式检查：无数据项使用对应成员路径模式；带数据项使用对应成员路径(_) 模式，并检查候选表达式中的载荷为 &'static str。这能使普通函数伪装成元组成员或字段类型不符的情况在编译时报错，不依赖单独的 enum 扫描。
- 首版列表必须内联在参数标注中，显示值必须是字符串字面量。前次讨论的独立 OPTIONS 常量可供运行时代码遍历，但参数过程宏只拿到常量路径，无法直接取得初始化内容；本方案选择继续编译期生成既有静态元数据。因此不支持 options(OPTIONS)、计算表达式、const 显示值或宏调用。跨参数复用如成为需求，先修订方案选择声明宏或元数据提供接口，不读取源码猜测常量。

## 元数据、解析与错误契约

生成的 definition 为：

```text
数据格式:ASCII/HEX-十六进制/UTF8-文本
```

1. 没有 options 时，旧描述字符串与 DeviceInput 路径原样保留。只有显式 options 才生成描述；名称允许纯名称或一个末尾冒号，拒绝已有非空冒号后缀与 options 同时出现，避免覆盖手写候选。参数名中的单位 / 保留既有规则。
2. 调用时继续按通道位置取值、去除参数名前缀，并沿用 codec 的显示值映射。生成的 options 解析再按 trim 后的实际值或显示值精确匹配，区分大小写，不用数字 discriminant 或声明序号作为实际值。
3. 匹配 ASCII 返回 DataFormat::ASCII；匹配 HEX 或十六进制返回 DataFormat::HEX("十六进制")。载荷来自声明，不直接借用请求字符串；每次调用构造新值，无须 Clone/Copy/Default。
4. 未知值、显式空白、完整候选列表和完整 actual-display 文本均返回现有 InvalidArgument，包含参数名。缺参沿用现有无默认值规则；实例空身份提前跳过规则不变。
5. options 是有穷选择约束，列表外值报错；旧字符串描述仍只映射、不自动限制合法集合。不得改变旧 u32 等输入可接受候选外值的契约。
6. 参数宏为初始化 Config 生成同源的纯规范化分支：实际值、显示值和带名称前缀的等价输入得到同一实际值。未知值报错。实例组合键、单实例配置比较、租约编码均通过既有回调使用它；选项文字不进入身份。该分支不要求枚举实现 DeviceInput，也不创建业务资源。
7. 不为枚举类型生成全局 DeviceInput 实现，以允许同一枚举在不同参数中声明不同候选子集/显示值，并避免与用户已有实现冲突。显式 options 的参数由其局部解析器负责；未标注参数仍按既有 trait 处理。
8. 首版不支持 options 与 batch/format 同时标注，宏报明确诊断。options 标量出现在其他参数使用 batch 的方法中仍允许，沿用既有非 batch 参数逐次构造规则。output 上的 options 明确拒绝。

### 候选校验

- 空列表、重复 options、非限定成员路径、非字符串载荷、多字段/结构体/嵌套构造语法均在宏展开期拒绝。
- 实际值、有效显示值分别唯一，且不同候选的实际值与显示值之间也不能交叉冲突；同一项实际值等于显示值允许。
- 实际值与显示值都非空，不允许首尾空白；拒绝破坏现有描述、签名和集合语法的字符：冒号、斜线、连字符、竖线、反斜线、英文逗号、圆括号及控制字符。raw identifier 按去除 r# 后的实际文字检查。
- 上述检查仅针对新增 options，不对旧手写描述施加新校验或改写。

## 默认值、所有权与兼容性

- 首项决定候选展示顺序，不引入运行时默认值。default_value 仍为 None；空白、缺参、未知值不自动回退首项，也不调用枚举 Default。
- 首个业务 Config 的既有宿主预填规范继续适用：候选顺序必须有业务依据，调用说明写清具体预填项，宿主仍须提交具体值。本次不新增默认项语法；如需明确指定而非排序表达，须先修订方案与平台描述设计。
- &'static str 载荷为程序内静态显示文字；原始请求生命周期不传播到枚举，不新增共享可变状态、锁、线程、I/O 或资源租约协议。
- 宏仍产出现有 ParameterDescriptor 静态 definition；Rust 描述结构、C 导出、JSON、内存所有权与 C ABI v9 均不变。不修改 abi/api/miresidian.h。
- 旧标注不改变语义；用户主动迁移 options 的参数将新增列表约束、区分大小写及规范化行为，迁移时需核对原手写解析接受的别名/大小写/默认规则。枚举内部任意业务载荷不按显示值解释，只有显式 options 中的单字符串构造属于本契约。

## 文件与版本评估

- 本次仅新增本方案：版本影响无，属于未实施设计，不提升组件版本、不改锁文件、不创建发行标签。
- 实施时预计 macro 0.24.0 → 0.25.0（公开宏语法新增），根框架 0.28.0 → 0.29.0（对外开放新参数能力并同步精确宏依赖）；其他 crate 不跟升。版本以正式实施前重新核对的源码基线为准，不能直接当作已发布版本。
- 实现集中在 macro/src/operation/options.rs（新增，解析与生成同源候选契约）；definition.rs 存储选项；config.rs 与 format.rs 的现有标注入口接收并验证 options；descriptor.rs 复用生成的 LitStr；dispatch.rs 统一固定/普通/异步参数解析与 Config 规范化分支。若需拆分现有标注解析职责，必须先在本文件补充具体范围。
- 不修改 codec、abi、instances、lease、batch、format 功能 crate 或独立 Node；复用 codec 的取值与选项映射公共入口。仅 macro/根版本与实际精确依赖影响 Cargo 清单及锁文件；核对所有锁文件，未受影响不制造更新。
- 实施时同步根及 macro README/Rustdoc、各自带日期 CHANGELOG；在 docs/宏驱动开发.md 定义完整语法，docs/c-abi.md 补充显式结构化选项生成的例外并保留旧字符串原样规则，docs/config-identity.md 说明枚举规范值，docs/releases/ 新增实际版本迁移。其他手册仅在实际受影响处更新，避免重复定义。
- 独立 Node 的采用须待新框架发布后另行执行；不得用 path/git/patch 测试未发布框架。框架内部集成测试不属于独立 Node。

## 实施顺序与验收

1. 用户确定采用方向后，依据届时实际基线更新版本评估与执行授权，再进入实施。
2. 实现 options 语法解析、候选校验和静态 definition 生成，保留旧标注测试。新增 macro 级测试与 tests/ui 编译诊断，覆盖空列表、冲突、字符串分隔符、错误构造、错误类型、显式 options 与旧后缀/batch/format 冲突。
3. 集中生成候选解析分支，接入固定入口及普通同步/异步调用；集成测试用无 DeviceInput/Clone/Copy/Default 的混合枚举，验证三种参数文本输入与方法收到的实际载荷、非法值不执行业务、缺参/空白/完整描述拒绝、子集约束及模块限定路径。
4. 接入初始化 Config 同源规范化；在框架内验证实际值/显示值/名称前缀复用同一实例，租约规范参数一致，未知值不取得资源、既有空身份规则不变。覆盖标量 options 与其他参数 batch 组合。
5. 核对 GetFunctions/GetInitConfig 的精确文本与顺序，以及旧描述逐字节不变；确认无新增 ABI/槽位/错误类别。同步受影响文档、精确依赖、锁文件与日期归档。
6. 运行相关宏/UI/集成测试、cargo fmt --all -- --check、必要默认及 instances/lease/batch 特性组合、对应 clippy 与文档检查。硬件和 Windows DLL 交付不在本次范围；纯方案阶段不运行构建测试。
7. 实施提交前执行 python3 tools/check_versions.py --base HEAD；暂存后由钩子检查 --staged。提交后分别核对根/宏不可移动标签和 --require-tag；实际测试及版本/提交/标签归档到本方案。不推送、不发布，除非另获授权。

## 本次设计阶段结果

- 完成现有宏、codec、静态描述、Config 规范化及版本规则只读调研。
- 编写本草案；所有新语法均为拟议设计，当前框架尚不支持。
- 功能实施、组件升版、集成测试、提交和发行均未执行。


## 最终实施与验收结果（提交前归档）

- 完成 NodeEnum 派生、codec 结构候选、参数类型元数据和平台成对 format_parameter/normalize_parameter；普通参数标注不重复成员，显式非空描述优先，缺参/空白不取首项。派生拒绝六类不支持形状，配置显示值/实际值复用同一对象，Option/Vec、别名、限定路径、cfg/batch 和 format-only 类型均有实际回归。
- 唯一业务导出改为 miresidian_invoke(request_json)，旧 invoke_with 与旧 invoke_request 导出均移除；保留 ABI 版本查询。ABI 0.17.0/v10 的 Rust 常量、头文件、元数据及 C 函数指针同步。平台由本次 JSON 调用指定；0/省略固定默认，不发布或消费 Config 选择器，也不隐式沿用先前自定义平台。
- 运行时使用本次真实适配器渲染和解析，直接未注册对象同样生效；结构化 native 调用接收规范值。移除 selector 传递及旧描述 helper，保留 ConfigLayout 历史索引字段并澄清不占平台槽位。共享契约纳入操作候选，租约线协议保持 v2。
- Windows 执行器先查询 ABI，再选择 v7-v9 的旧签名或 v10 的单 JSON 签名，未知版本调用前拒绝。工具 0.13.0 与 CLI/套件 0.3.5 同步；Pack 及其仅支持旧 v7 的模板保持。独立 Node/夹具/skill 未修改，不使用 path/git/patch 绕过 registry。
- 迁移过程中旧测试仍向 Config 填平台项、期望旧元数据长度或 UNKNOWN_PLATFORM_ADAPTER，已按新契约改为调用平台和 INVALID_ARGUMENT；没有为迁就旧测试保留运行时选择器。首次自定义平台测试使用只有 TB 的 NodeRuntime::new 导致未知平台，改为显式传入节点注册表，生产构造规则未被擅改。
- 同步所有实际升版组件 README/日期 CHANGELOG、根双语指导、节点/实例/平台/架构手册和 0.30.0 迁移。工作区锁文件仅有十个实际组件的版本更新，没有第三方依赖变化；其他锁文件及旧发行归档保持。

| 验收 | 实际结果 | 证据 |
| --- | --- | --- |
| 默认工作区全目标测试 | 128 passed，0 failed | /tmp/miresidian-enum-default.log |
| 全特性工作区全目标测试 | 252 passed，0 failed | /tmp/miresidian-enum-all.log |
| 派生诊断及双平台定向回归 | 通过；新 UI 诊断已人工审阅 | tests/ui/node_enum_invalid.stderr、/tmp/miresidian-enum-focused.log |
| 一次全特性严格 Clippy | 通过，-D warnings | /tmp/miresidian-enum-clippy.log |
| 四个公开接口组件严格 Rustdoc | root/codec/macro/ABI 通过 | /tmp/miresidian-enum-rustdoc.log |
| 一个 debug 内部 DLL 构建与实际加载 | 通过，真实 C ABI v10 | /tmp/miresidian-enum-dll-build.log、/tmp/miresidian-enum-dll-load.log |
| 原生加载覆盖 | 旧符号不存在，默认/0/显式平台，双平台候选/转换，旧 Config 拒绝，非法请求 | tools/verify_enum_dll.py |
| 当前文档链接/标题锚点 | 224 处通过 | 本地检查结果 |
| 格式与版本门禁 | fmt/diff/base 检查通过，提交时再执行 staged | 本地执行结果 |

按用户要求复用已有测试，不重复 MSRV 或全工作区 Release 检查，不增加其他 DLL 模拟。没有可用的 PowerShell/.NET 编译环境，Windows 内嵌执行器仅有生成器检查，未宣称原生 Windows 加载或真实硬件验证。所有候选均未推送、未发布。

本方案随版本提交归档；提交号与十个不可移动版本标签由 Git 和 --require-tag 的实际结果核对，不在提交内预写自身哈希。旧方案均仅为历史讨论，不再指导实施。

## 2026-11-11 交付资料复查与补齐方案

执行依据：用户询问 skill、注释和文档是否完善。只核查并修复实际遗漏，不重跑上一提交已通过的工作区/原生库验证。

- 发现 docs/coding-standards.md 两处仍把当前 ABI 描述为 v9/三入口/Config 选平台；改为引用当前 C ABI 技术手册，不在编码规范复制会过期的入口和版本表。
- 根 descriptor Rustdoc 仍把诊断用历史平台名称写成“自动追加的选择器”，且 raw definition/signature 与 *_for 的区别不够明确；只补准确注释，不改函数或行为。
- ABI 公共源码的泛化描述仍有设备限定，改为业务节点/资源/业务操作；不变更状态码、函数签名或协议。
- 框架 0.30.0 → 0.30.1、ABI crate 0.17.0 → 0.17.1，仅为随组件发行的注释/指导修正；ABI 协议保持 v10，其他组件不升版。同步 root 精确 ABI 依赖、Cargo 解析锁文件、两个组件 README/日期 CHANGELOG、根双语指导及 docs/releases/0.30.1.md；旧发行归档不改。
- Skill 维护规则禁止固定 API/ABI 适配表，不因组件升版要求同步。主流程和 review 已覆盖实际版本查询、生成契约职责、输入输出/显示值、ABI及调用验证，本次无需复制 NodeEnum 或 v10 语法到 skill。仓库与安装完整 5 文件已逐字节核对一致，无需修改任何一份。
- 验收仅做注释对应实际实现、当前文档残留和本地链接、两组件严格 Rustdoc、fmt/diff 与 base/staged 版本门禁；核对运行时代码 token 不变、锁文件仅两组件版本变化。完成后提交并核对 root/ABI 不可移动标签，不推送、不发布，不重复编译测试和硬件验证。

复查补齐结果：两条旧规范及元数据/ABI 注释已修正；root 0.30.1、ABI crate 0.17.1（协议仍 v10）。完整 skill 5 文件与安装副本字节一致，无修改。两组件严格 Rustdoc、196 处本地链接/锚点、fmt/diff/base 检查通过；两个 Rust 文件移除 Rustdoc 后与前版完全一致，锁文件仅 root/ABI 两版本变化。未重复功能测试，复用前次已通过结果；未推送、未发布。

## 2026-11-11 推送与正式发布阶段

用户明确授权“推送和发布”，本节覆盖此前仅本地交付边界，授权本任务相关分支/标签推送、当前组件正式发布及实际发行验收；不授权独立 Node 升级、硬件操作或部署。

### 预检与版本影响

- 工作区干净，发行根/ABI 快照为 f21e8a1（框架 0.30.1、ABI 0.17.1/v10）；远端 master 为 677e833，待推送历史包括本任务及先前 Node skill/feature 简写提交。非强制快进，不覆盖远端修改。
- crates.io API 精确查询确认当前 11 个版本均 404：root 0.30.1、ABI 0.17.1、codec 0.18.0、macro 0.26.0、format 0.4.1、batch 0.3.1、instances 0.16.8、lease 0.17.3、Pack 0.13.4、TestFlow 0.13.0、CLI 0.3.5。预检证据 /tmp/miresidian-release-registry-preflight.json；上传前再核对失败/重试状态，避免重复发布。
- 按各组件不可移动标签发布：root/ABI 为 f21e8a1，Pack 0.13.4 为 0698149，其余当前组件为 24ef546。三个干净 detached worktree 固定发行提交，不用当前纯方案提交作为包 provenance。
- 本阶段仅更新方案与实际发行记录，版本影响无；不再修改产品版本、依赖、README 或已有标签。候选 README 是发行快照原实施状态，正式发布事实记录在本阶段，不为更改候选措辞反复升补丁。

### 步骤与验收

1. 先提交本方案，再 fetch 并核对远端分支与新提交的祖先关系；检查相关标签若远端已存在则必须完全同指向。仅原子推送 master 及待推送提交范围内明确列出的版本标签，不 force、不执行全局 --tags；核对远端解引用结果。
2. 为 f21e8a1、24ef546、0698149 建立 /tmp 下三个干净快照；复用既有功能验证，逐组件 require-tag 并检查包必需文件（README、CHANGELOG、源码、根技术手册/skill、ABI 头文件、宏派生实现、工具内嵌模板/执行器/套件清单）。
3. 正式发布每个当前版本，顺序为 codec、ABI、macro、instances、lease、format、batch、root、Pack、TestFlow、CLI；依赖 registry 可用后再发布依赖方。对有可选能力的库使用 --all-features，均保留 --locked 与 Cargo 正式 Packaging/Verifying/Uploading，不使用 --allow-dirty、--no-verify，不重复等价 dry-run/全量测试。
4. Cargo 发布失败先确认 crates.io 精确版本状态；已经成功则不重传，网络/索引未传播仅在证据支持后重试。凭据由既有 Cargo/Git 配置使用，不读取或输出凭据。
5. 发布后分别核对每个精确版本未撤回、官方 checksum、真实下载包 SHA-256、.cargo_vcs_info.json 的准确 tag 提交与非 dirty、规范化精确依赖/无 path/git、README/日期归档和关键源码；核对远端不可移动标签。
6. 分别检查 docs.rs 精确版本 README/API/源码，根额外核对 NodeEnum、EnumOption、NodeInput、PlatformAdapter 成对方法和 format 可选入口，ABI 核对 v10/单入口；文档排队或失败按实际报告，不用上传成功代替可访问文档。
7. 在本方案归档真实结果，以纯方案提交推送；发行标签仍指向各自原提交，不移动。最终报告实际版本、发行提交/标签、registry 与文档验证以及任何未完成边界。

发布结果待补充；本节不把预检记为已推送或已发布。

## 2026-11-11 GitHub CI 失败修复（发布前调整）

用户报告 GitHub 报错并授权修复。远端 master/标签已成功推送，失败为 CI 43000420396：Windows Rust 1.89 的 shared_resource::complete_transactions_are_serialized_across_independent_clients 在首个 SharedResource::acquire 返回 LocalIpc(os error 5)。Linux MSRV 被矩阵 fail-fast 取消；framework 与 Windows 工具任务通过。日志 /tmp/miresidian-ci-windows-msrv.log。

原因：local_ipc/service.rs 在连接失败后竞争创建监听器，仅接受 AddrInUse。interprocess 2.4.6 使用 FILE_FLAG_FIRST_PIPE_INSTANCE；Microsoft CreateNamedPipeW 文档明确同名管道已存在时返回 ERROR_ACCESS_DENIED (5)，不会返回 Unix 的 AddrInUse。多个线程/进程同时首次连接会走到该路径。

设计与文件范围：

- 将既有连接快速路径后的创建/连接逻辑收为私有 start_and_connect；创建返回 AddrInUse 或 Windows 原生错误 5 时，仅把它视为可能已有监听器，继续既有有限次数连接。只有实际连接成功才返回成功；次数耗尽返回原创建错误，真正权限拒绝不被吞掉。其他创建错误立即返回，不扩大错误匹配。
- 在该模块添加确定性回归：先启动/连接真实服务并保留连接，再直接进入 start_and_connect，强制触发同名监听器创建冲突，验证仍能连接。保留既有共享资源的并行/多进程回归；不序列化测试、不跳过 Windows、不放宽权限或修改管道独占标记。
- 更新服务模块注释、lease 双语 README/日期 CHANGELOG、根双语 README/日期 CHANGELOG、docs/shared-resources.md 与 docs/releases/0.30.2.md。根 0.30.1 → 0.30.2、lease 0.17.3 → 0.17.4，根精确依赖和工作区锁文件同步；没有公开 API、C ABI v10 或租约/代理 v2 变化，其他组件和独立 Node 不改。
- 先运行 lease 定向全特性测试、根 lease 回归、相关严格 Clippy/格式与版本门禁；推送后以 GitHub Windows/Linux Rust 1.89 真实检查为验收，不重复本地全工作区矩阵。
- 新补丁提交及不可移动标签核对后推送。正式发行清单中的 root/lease 改为 0.30.2/0.17.4，并从新提交干净快照发布；不发布已被取代的 0.30.1/0.17.3，不移动先前标签。其他九个发行快照及正式包/registry/docs.rs 验收按前节执行。发布前先确认修复 CI。

修复本地验收：lease 全特性 16 项通过（含确定性同名监听器分支和既有并行/多进程回归）；根 lease_initialization/lease_recovery 通过（子进程辅助入口仍按原约定忽略），lease 严格 Clippy、fmt/diff 与版本门禁通过。证据 /tmp/miresidian-ci-fix-{lease,runtime,clippy}.log。Windows 实测以本次推送后的 GitHub CI 结果为准，尚未将本地 Linux 结果当作 Windows 验收。

## 2026-11-11 正式发行与 GitHub 修复最终结果

GitHub 原始失败已修复并推送：提交 8342cf1bb91e7dd3bac9953c23680203fdb0bfcc，框架 0.30.2、lease 0.17.4。不可移动标签 miresidian-v0.30.2、miresidian_lease-v0.17.4 均指向该提交。原失败为 Windows 并发启动命名管道的 CI 运行失败，Git 推送本身成功。

- [CI 43013459307](https://github.com/MyMires/miresidian/actions/runs/43013459307) 全部成功：framework、Windows/Linux Rust 1.89、Windows 工具端到端；原失败的共享资源并发测试及新增确定性监听器回归在 Windows 真实运行通过。
- [独立工具套件 43013459188](https://github.com/MyMires/miresidian/actions/runs/43013459188) 验证成功：Linux/Windows 两项套件构建、安装与真实进程验证均通过；仅手动分发触发的 draft-release 按既有条件跳过。复用既有工作流，未额外重复本地全工作区矩阵。证据 /tmp/miresidian-ci-fix-github.json、/tmp/miresidian-toolset-fix-github.json。
- master 与本次历史范围内的 25 个明确版本标签已推送，逐一核对远端对象和解引用提交；没有 force、移动已有标签或推送无关标签。
- 以下 11 个精确版本已正式发布至 crates.io，未撤回；逐包执行 cargo publish --locked --all-features，保留正式包编译验证，无 --allow-dirty/--no-verify。ABI 第一次上传 TLS 中断，确认精确版本仍 404 后重试成功，没有重复上传已存在版本。

| 组件 | 已发布版本 | 发行提交 |
| --- | --- | --- |
| [miresidian_codec](https://crates.io/crates/miresidian_codec/0.18.0) | 0.18.0 | 24ef546 |
| [miresidian_abi](https://crates.io/crates/miresidian_abi/0.17.1) | 0.17.1 | f21e8a1 |
| [miresidian_macro](https://crates.io/crates/miresidian_macro/0.26.0) | 0.26.0 | 24ef546 |
| [miresidian_instances](https://crates.io/crates/miresidian_instances/0.16.8) | 0.16.8 | 24ef546 |
| [miresidian_lease](https://crates.io/crates/miresidian_lease/0.17.4) | 0.17.4 | 8342cf1 |
| [miresidian_format](https://crates.io/crates/miresidian_format/0.4.1) | 0.4.1 | 24ef546 |
| [miresidian_batch](https://crates.io/crates/miresidian_batch/0.3.1) | 0.3.1 | 24ef546 |
| [miresidian](https://crates.io/crates/miresidian/0.30.2) | 0.30.2 | 8342cf1 |
| [miresidian_pack](https://crates.io/crates/miresidian_pack/0.13.4) | 0.13.4 | 0698149 |
| [miresidian_testflow_windows](https://crates.io/crates/miresidian_testflow_windows/0.13.0) | 0.13.0 | 24ef546 |
| [miresidian_cli](https://crates.io/crates/miresidian_cli/0.3.5) | 0.3.5 | 24ef546 |

每行的不可移动标签为 `<组件>-v<版本>`。框架/lease 最新修复快照为 8342cf1，ABI 为 f21e8a1，Pack 为 0698149，其余为 24ef546。全部从各自干净 detached worktree 发布，未把方案归档提交写入发行包 provenance。0.30.1 与 lease 0.17.3 被本次修复取代，保留原标签，不额外发布中间版本。

逐包下载官方 .crate，SHA-256 与 registry checksum 一致，.cargo_vcs_info.json 的准确提交、组件路径及非 dirty 状态均与对应标签一致。README、日期 CHANGELOG 和关键源码与发行快照逐字节一致；规范化第一方依赖均为精确版本，无 path/git 覆盖。证据 /tmp/miresidian-release-publish-results.json，逐包日志 /tmp/miresidian-publish-<组件>-<版本>.log。

在线资料验收 36/36 实际可访问：八个库各自精确版本 README/API/源码、三个工具 README/源码，以及根 NodeEnum、EnumOption、NodeInput、PlatformAdapter 成对方法、format 可选入口和 ABI 常量页面。拒绝把重定向到包介绍页计作 API 成功，早期缓存/排队状态在最终核对时消除。ABI 在线常量实为 10，下载头文件仅含 miresidian_invoke(request_json) 业务调用及独立版本查询。证据 /tmp/miresidian-release-docs-results.json；[根精确 API](https://docs.rs/miresidian/0.30.2/miresidian/)、[根源码与 README](https://docs.rs/crate/miresidian/0.30.2)、[ABI 精确 API](https://docs.rs/miresidian_abi/0.17.1/miresidian_abi/)。

本阶段未升级独立 Node、未操作硬件、未修改 skill 或无关组件。最终归档仅修改本方案，版本影响：无；不更改产品契约或发行标签。归档提交保留本地版本门禁，使用 [skip ci] 避免对纯发行记录重复触发整套构建；有效执行验证对应实际发行源码 8342cf1。
