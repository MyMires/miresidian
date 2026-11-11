# 2026-11-11 框架设备限定命名残留审计

状态：公开类型更名、指导同步及全部计划验证完成；版本提交与标签以 Git 记录和交付核对结果为准。
执行依据：用户再次强调框架不能用狭隘的设备描述，并要求仔细检查 DeviceInput、DeviceOutput、DeviceConfig、DeviceFunction 等遗漏。本阶段交付完整事实清单，不把未实施的更名描述为已完成。

## 目标与方法

- 核实实际存在的设备限定公开类型、方法、宏生成代码、平台字符串、模板、指导文档及相邻节点引用，不只查单个类型。
- 框架以 git ls-files 枚举全部受跟踪文本，包含隐藏配置、工具脚本、模板和编译诊断快照；串口目录同样检查 Git 文件，TCP 目录本次无可用 Git 元数据，使用 rg --files --hidden 枚举可见文件。
- 大小写不敏感检查 device、dveice、设备；二进制及 Cargo.lock 不计入文本命名统计；plans、docs/releases、docs/history 与 CHANGELOG 单列为历史，不将旧发行文档当成当前 API。
- 数量表示命中文本，并非都要改名：真实串口/TCP 通信、硬件实例及历史事实可能合理提到设备，需要按职责审查，不能全文盲替换。
- 全量逐行证据：/tmp/miresidian-device-complete-audit.json；源码标识符清单：/tmp/miresidian-device-symbol-audit.txt；本文件固化核心发现，临时文件不是长期交付依赖。

## 已确认的六个公开类型

| 类型 | 定义位置 | 当前职责 |
| --- | --- | --- |
| DeviceInput | codec/src/lib.rs:129 | Config/Input 文本解析与规范化 |
| DeviceOutput | codec/src/lib.rs:141 | 返回值编码 |
| DeviceCollectionElement | codec/src/lib.rs:150 | 一维集合元素编解码 |
| DeviceScalar | codec/src/value.rs:5 | 通用标量值 |
| DeviceValue | codec/src/value.rs:25 | 通用空值、标量、数组 |
| DeviceNode | src/runtime/node.rs:11 | 节点与运行时适配接口 |

这些职责均不是设备专属，却仍以 Device 命名。根 src/lib.rs:100、113、129 仍重导出这些旧名称；宏、运行时与功能 crate 仍实际使用，并非只剩过时注释。

用户点名的 DeviceConfig、DveiceConfig、DeviceFunction、DveiceFunction 及 device_config、device_function 在本次框架文本和相邻节点可见内容中均未检出；不能把相似职责推断为存在同名类型。

## 同时残留的方法、生命周期及平台名称

- codec 公共 trait 方法：parse_device_text、normalize_device_text、into_device_output、parse_device_collection、format_device_collection。
- Rust 生命周期：init_device、close_device，包含 trait、运行时调用、宏识别、宏生成实现、节点模板、示例、测试及相邻节点。
- 导出的生命周期常量：INIT_DEVICE_OPERATION、CLOSE_DEVICE_OPERATION、CLOSE_ALL_DEVICES_OPERATION。
- 平台操作字符串：InitDevice、CloseDevice、CloseAllDevices；src/runtime/contract.rs 定义，descriptor 发布，适配器/工具执行器及头文件指导均依赖。
- get_port_no / GetPortNO 不含 Device，但以物理端口命名，同属框架抽象是否仍有设备假设的审查点；不能在更名设计中漏审，也不能只凭关键词为零就认定抽象已完成。

## 传播路径和自动再引入点

- macro/src/node/generate.rs 生成 DeviceNode 实现、init_device / close_device 等，macro/src/operation/dispatch.rs 生成 DeviceInput 调用，definition.rs 识别旧生命周期方法名。
- features/format/src/conversion.rs 使用 DeviceInput/DeviceOutput；features/batch/src/{records,outputs}.rs 使用输入、输出和集合 trait。
- tools/miresidian-pack/templates/node.rs.in 生成旧生命周期；AGENTS.md.in 指导不手写 DeviceNode；README.md.in、node.md.in、smoke.testflow.in 与打包/CLI/Windows 流程工具指导及执行器仍引用旧操作名。
- tests/ui 的 .rs 与 .stderr、examples、框架内工具烟测夹具、Python DLL 验证脚本均含引用。独立烟测夹具依赖已发布框架，不能为了通过新 API 验证改为本地覆盖。
- README、codec/README、macro/README、架构图、宏驱动手册、C ABI、配置身份等当前指导继续使用这些名称。
- src/lib.rs 首行仍将框架定义为“供测试平台加载设备节点的运行时框架”；codec/src/error.rs:111 和 abi/api/miresidian.h:72 仍以“设备尚未初始化”描述通用生命周期错误。租约失效等消息及运行时注释同样需要按通用资源/节点语义审查。
- skill 中仍有设备协议和设备动作措辞，应区分真实通信场景提示与通用指导的限定；若修改，须按 skills/miresidian-node/AGENTS.md 同步完整安装副本，不能只改 SKILL.md。
- 相邻串口 src/node.rs 仍实现 DeviceInput；TCP src/transport.rs 仍导入并实现 DeviceInput。其采用新 API 需在新框架发布后迁移，不得用 path/git/patch 绕过。

## 检索统计

- 框架受跟踪文本：含历史共 189 文件 / 1116 行；排除上述历史归档后 164 文件 / 1039 行。
- 串口受跟踪文本：含历史 10 文件 / 52 行；排除历史后 7 文件 / 43 行。
- TCP 可见文本：11 文件 / 66 行；该范围不同于 Git 跟踪清单，不混报为同一口径。
- 这些计数覆盖设备限定名称、协议字符串、普通英文 device 和中文设备说明，不等于同等数量的 API 或缺陷。

## 原因与后续完整清理的验收设计

当前事实是类型定义、重导出、生成器、模板、执行器和指导文档均仍带有旧命名，说明“改部分可见名称”没有形成跨组件验收。仅重命名 DeviceInput 无法完成用户的通用框架要求；模板与生成器还会持续再引入旧名。本次没有足够证据断言某次历史修改的具体动机。

实施前的方案必须确定每个类型/方法的新名称及平台协议映射；逐项标明职责归属、兼容性、影响文件和版本，而非直接全仓替换。关于是否更改平台固定操作名的范围问题先前已发出，用户尚未给出明确的协议兼容选择；这不影响本次只读审计完成。

完整清理至少验收：

1. 活跃框架公开 API、重导出与内部通用命名符合统一术语；旧名不得通过未声明的兼容别名继续暴露。
2. 宏生成实现、模板新建项目、默认与可选功能构建、UI 诊断和运行时调用使用相同新名；模板验收必须实际展开/编译，不能只确认源码字符串替换。
3. Rust 生命周期与平台固定操作分别核对；平台若保留旧名，必须归属明确的平台兼容边界；若改名，必须同步宿主、流程工具、头文件、元数据、协议版本评估与迁移文档。
4. 通用错误消息和指导不把节点限定为物理设备；真实通信示例按业务保留准确描述，历史发行记录不被重写。
5. 保存允许保留的旧平台/历史/真实通信命中清单；对除此以外的 Device/device/Dveice/设备 命中逐项审查。验收覆盖模板和文档，不能只对 src 执行 rg。
6. 原枚举候选方案仍未实现，其后续设计须采用统一后的术语，不能继续传播旧 API。

## 版本评估与本阶段验证

- 本阶段仅形成审计方案记录：版本影响无，不改变可构建或可调用行为，不升版、不改依赖/锁文件、不建立新标签。
- 实际清理涉及公开 Rust API，需对真正受影响的 root、codec、macro、format、batch 等组件分别作次版本评估；工具仅在实际变化时按自身规则升版，其他组件不跟升。ABI、租约协议、独立 Node 分别评估，不能用 crate 升版代替协议判断。
- 本阶段运行文本审计、报告结构/空白及 git diff --check、版本门禁；不运行与只读调查无关的编译或硬件测试。不提交尚未确定的功能清理。

## 2026-11-11 修订：公开类型更名实施方案

执行依据：用户明确要求“需要把公开类型的 Device 改为符合框架的名称”。该指令授权实施及仓库规定的本地验证、提交和标签；无需再次确认。先前平台固定操作名的范围问题不阻碍本次类型更名，本次不修改平台协议。此前审计章节保留为更名前证据。

### 目标与接口设计

| 旧公开类型 | 新公开类型 |
| --- | --- |
| DeviceInput | NodeInput |
| DeviceOutput | NodeOutput |
| DeviceCollectionElement | NodeCollectionElement |
| DeviceScalar | NodeScalar |
| DeviceValue | NodeValue |
| DeviceNode | Node |

- 编解码 trait 的关联方法同步改为 parse_text、normalize_text、into_output、parse_collection、format_collection，消除同一公开编解码接口内部的设备限定。不保留旧类型别名或旧方法转发。
- 对应类型身份、方法行为、泛型约束、数据布局、默认与错误语义、所有权、并发、超时、取消和资源生命周期保持原实现；只更名，不引入新功能或额外共享状态。
- Node 的 init_device/close_device 与宏对应生命周期标注、INIT_DEVICE_OPERATION 等平台操作常量及 InitDevice/CloseDevice/CloseAllDevices 字符串保持，属于本次未修改的平台生命周期映射；不将它们报告为已清理。C ABI v9、JSON、错误文本及租约/资源代理协议 v2 均不改。
- 枚举自动候选功能仍未获实现授权，本次不实施；先前枚举草案中的旧类型名属于历史设计记录，不作为新接口使用指导。

### 文件、组件版本与指导同步

- 根 0.28.0 → 0.29.0：src/ 重导出、Node 泛型约束、运行时与 FFI Rust 桥；根测试及直接受影响示例；双语 README/CHANGELOG、架构图、节点开发手册、共享资源/C ABI 手册与迁移说明 docs/releases/0.29.0.md。
- codec 0.16.0 → 0.17.0：lib/scalar/collection/value、组件测试、README 与日期 CHANGELOG。新公开编解码名称及迁移示例在本组件 README 定义。
- macro 0.24.0 → 0.25.0：生成 Node 实现和 NodeInput::normalize_text；Rustdoc/README/日期 CHANGELOG。宏保持节点作者无需手写 Node 的规则。
- format 0.3.2 → 0.4.0、batch 0.2.2 → 0.3.0：改用新 codec 公共接口，精确依赖同步，组件 README/日期 CHANGELOG 同步。batch 的公开泛型约束使用新 trait。
- instances 0.16.6 → 0.16.7、lease 0.17.1 → 0.17.2：仅同步其现有 codec 精确依赖至 0.17.0，避免框架出现两套不兼容的 codec；不修改自身 API/协议，实现与自测保持。分别更新 README 和日期 CHANGELOG；这是实际依赖变化的独立补丁评估，不是随框架同号升版。
- Pack 0.13.3 → 0.13.4：模板 AGENTS.md.in 将硬编码 DeviceNode 改为“框架节点接口（名称以实际依赖版本为准）”，兼容其仍仅支持已发布 ABI v7 的模板；模板 Rust 生命周期与安装选用版本不改。README/日期 CHANGELOG 说明本次仅消除指导硬编码。
- CLI/套件 0.3.3 → 0.3.4：内嵌 toolset 的 Pack 最低版本同步 0.13.4；README/日期 CHANGELOG 同步。TestFlow 工具不变。
- 逐个同步实际精确依赖与工作区 Cargo.lock；检查另外两个独立夹具锁文件，保留已发布依赖及 checksum，不为了更名手工伪造锁记录或迁移夹具。
- 不修改独立串口/TCP 项目，也不修改其使用的已发布旧 API；框架发布后方可另行迁移。当前技术指导和测试改用新 API，历史 CHANGELOG/发行归档/既有方案不重写，只增加本次迁移记录。
- 本次不改 skill：它不包含六个旧类型名或五个旧编解码方法名，无须为了组件升版触发无关 skill 同步。不改 ABI 组件、公共头文件或版本门禁工具。
- 所有新版本仅为本地源码候选，安装示例不改成未发布版本的可直接安装命令。组件 README 中需要示例时明确为框架内部源码/迁移示例，外部项目只选真实已发布版本。

### 实施步骤

1. 本节先落盘，确认现有 hooksPath 为 .githooks 后执行 git config core.hooksPath .githooks；不覆盖其他自定义钩子。
2. 对活跃 Rust 源码、测试、当前指导按上表进行精确标识符替换；保留平台生命周期、历史归档和独立项目。宏与根重导出同次修改，避免新旧接口混用。
3. 同步组件版本与精确依赖，由 Cargo 离线重新解析工作区锁文件；仅使用已有缓存，不引入新依赖或改变第三方依赖版本。同步组件 README、日期 CHANGELOG、根双语说明及迁移示例。
4. 新增一个框架内集成测试，使用没有设备含义的枚举/业务节点验证自定义 NodeInput/NodeOutput/NodeCollectionElement、NodeScalar/NodeValue 及宏生成的 Node 实现共同可用。补充旧公开类型导入失败的 UI 诊断，防止别名重新引入；旧名只允许出现在此负向测试及迁移/历史记录中。
5. 验证组件与工作区默认/全特性测试、格式、严格 Clippy、严格 Rustdoc、Rust 1.89 编译；执行 Pack/CLI 现有模板和清单测试，检查模板不再硬编码旧 trait。依赖版本与后端探测测试预期只在实际报错时精确同步。
6. 对活跃文件复查六个旧类型和五个旧方法，逐项确认命中仅属于明确迁移、负向测试或历史；检查不存在兼容别名。核对 C ABI/租约协议、头文件、平台入口和独立项目无变更。
7. git diff --check 与 python3 tools/check_versions.py --base HEAD 通过后暂存本次相关文件，由钩子执行 staged 门禁并提交；提交后核对根标签 miresidian-v0.29.0 及所有实际升版组件不可移动标签，执行 --require-tag。不推送、不发布。

### 验收与证据

- 新名称可通过根重导出和 codec 原生公开入口使用；自定义输入、输出、集合、配置规范化和业务调用往返保持行为；Node 由宏生成。
- 旧六个公开类型不再可导入、旧五个编解码方法不再可调用；正向集成与负向 UI 测试分别提供证据。
- 默认/全特性工作区、相关工具测试、严格静态/文档检查及 MSRV 有实际结果；未执行的目标平台/硬件测试明确记录，不声称 Windows 或真实硬件验证。
- 版本、依赖、锁文件、组件指导、日期归档、提交与标签在同次交付中一致。实施结果追加在下方，不事后改写上述事前设计。


## 公开类型更名实施结果（提交前）

- 完成六个类型及五个编解码方法的更名，根/codec 重导出、运行时泛型约束、宏生成的 Node 实现和配置规范化桥均已同步；旧名称没有兼容别名。
- 当前技术文档、架构图、自定义业务类型示例、双语根指导、各升版组件 README/日期 CHANGELOG 和 0.29.0 迁移说明已同步。Pack 模板指导引用实际版本的框架接口名称，CLI/套件最低 Pack 版本同步。
- 工作区锁文件由 Cargo 解析，差异只有九个实际升版组件的版本号，第三方依赖未改变。ABI 目录/公共头文件、独立烟测清单和锁文件、skill 及相邻独立 Node 均未修改；没有使用本地覆盖。
- 新增 tests/node_type_names.rs：没有设备含义的 WorkflowState 经 NodeInput/NodeOutput/NodeCollectionElement 完成标量、集合、实际值/显示值输入、类型规范化配置查找和非法值拒绝；验证 NodeValue/NodeScalar 的根与 codec 类型一致，Node 由宏生成。
- 新增 tests/ui/removed_device_type_names.rs/.stderr：六个旧类型均无法从根导入，编译诊断已审阅；既有默认 UI 诊断没有被批量改写。
- 全特性初次检查发现测试辅助文件中嵌套结构体 Node 被名称扫描误认为同层冲突，导致 trait 匿名导入后无法用于泛型约束；已按实际作用域恢复具名 trait 导入，全特性重跑通过。生产实现没有因此改变。

验证证据：

| 检查 | 实际结果 | 日志 |
| --- | --- | --- |
| 默认全工作区全目标测试 | 125 passed，0 failed；包含工具模板/套件测试及 29 个 UI 用例 | /tmp/miresidian-node-rename-default.log |
| 全特性全工作区全目标测试 | 249 passed，0 failed；包含 42 个 UI 用例及实例/租约/批量/格式组合 | /tmp/miresidian-node-rename-all-features.log |
| 默认严格 Clippy | 通过，-D warnings | /tmp/miresidian-node-rename-clippy-default.log |
| 全特性严格 Clippy | 通过，-D warnings | /tmp/miresidian-node-rename-clippy-all.log |
| 全特性严格 Rustdoc | 通过；六个新类型及可选 format 模块共 7 个精确页面存在 | /tmp/miresidian-node-rename-rustdoc.log |
| 全特性工作区 doctest | 通过 | /tmp/miresidian-node-rename-doctests.log |
| Rust 1.89 全工作区全目标全特性 check | 通过 | /tmp/miresidian-node-rename-msrv.log |
| 格式/差异/版本门禁 | cargo fmt --all -- --check、git diff --check、--base HEAD 通过 | 本地执行结果 |
| 当前文档文件链接及标题锚点 | 236 处通过；新增租约英文版本段落不增加本地链接 | 本地执行结果 |
| 活跃文件残留复查 | 六个旧类型及五个旧编解码方法零命中；仅允许历史、迁移及负向测试 | 本地执行结果 |

本次实际验证为 Linux 框架和工具模拟、编译及本地 API 文档；未执行 Windows 原生宿主加载或真实硬件测试。不推送、不发布。旧平台操作名、Rust 生命周期和中文错误消息的设备描述仍属于本次类型更名以外的已审计事项，不能把本次交付表述为所有 Device/设备命名已清零。

本方案随版本提交归档验证证据；实际提交号和不可移动标签通过 Git 记录及 --require-tag 核对，不在提交内预写自身尚未生成的哈希。
