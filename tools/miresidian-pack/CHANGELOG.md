# Changelog

## [Unreleased]

## [0.14.0] - 2026-11-13

- build/pack 在所选目标编译成功后，自动提取 operation 方法的字面 Rustdoc 与声明，合并到项目节点说明的受管区域；pack 交付同一 Markdown，无需 AI 另行整理。
- 保留区域外手写内容，重复生成幂等，跟随方法修改/删除；异常标记、解析或编译失败不覆盖原文，写入采用同目录临时文件替换。静态参考保留条件声明，明确不展开宏/cfg、动态文档及属性别名，不推算 DLL 平台契约；旧方法缺注释如实提示。
- 模板增加中文业务 Rustdoc 和自动文档维护说明；新增解析、合并保护与真实 registry Node 的 DLL 构建/交付回归。当前为未发布源码候选，验证证据见根任务方案。
- 验证：30 项 Pack 单元、1 项真实 registry/Windows GNU x64 构建与打包文档回归、严格 Clippy/Rustdoc、Rust 1.89 及版本检查通过；未进行 Windows 原生加载或硬件验证。
- English: Generate business method Markdown from Rust documentation comments during build/pack, preserve hand-written sections, and ship the generated guide with the DLL. No AI post-processing is required.

## [0.13.6] - 2026-11-12

- 修复 new/init 默认选择最新正式框架后仍强制 ABI v7 的失败路径；按真实 registry 解析结果支持 ABI v7–v10，创建成功信息显示实际 ABI。
- 调用说明拆分通用内容及旧三参数/v10 单 JSON 契约，记录默认平台、ABI 校验、内存所有权与完整回显调用；未知 ABI 拒绝生成签名，未完成项目不保留错误的 v7 指导。
- 增加已发布 ABI7–10 和默认最新版 new/init 的真实生成、编译、动态库查询/初始化/回显/关闭回归，保留旧版及覆盖保护测试；同步自身 README 与模板 README。
- 验证：24项Pack单元测试、4项显式registry集成（含6个原生动态库回显项目）、严格Clippy/Rustdoc及Rust1.89检查通过；未在本地执行Windows原生或硬件验证。
- English: Fix latest-version scaffolding with ABI-aware v7–v10 calling guides; verify published-version and default new/init paths through native library calls. Unknown ABIs fail explicitly.

## [0.13.5] - 2026-11-12

- 框架库清单识别 miresidian_instance；继续对旧 miresidian_instances 执行精确依赖、crates.io 解析/锁来源及覆盖检查，旧节点无需迁移。
- 更新 README 和新旧包名来源测试；模板 ABI v7、命令与打包行为保持。未发布补丁。
- English: Recognize miresidian_instance while retaining all source checks for legacy miresidian_instances. Templates and packaging behavior remain unchanged; unpublished.

## [0.13.4] - 2026-11-11

- 生成的 AGENTS 指导不再硬编码 DeviceNode，改为引用项目实际版本的框架节点接口名称。
- 模板业务代码、已发布依赖选择与仅支持 ABI v7 的边界保持；同步 README。

## [0.13.3] - 2026-11-07

- 文档分别说明 new 的新目录要求与 init 的已有目录及冲突规则，不再混写目录前置条件。
- 在新建命令旁明确现有 ABI v7 限制；示例显式选择已发布框架 0.25.2，说明自动选择到 v8 后的未完成状态及处理方式。模板、解析和打包实现保持不变。
- 验证：23 项单元测试及严格 Rustdoc 通过；通过 CLI 实测 0.25.2 的 registry new/init、保留原文件、编译和项目检查。默认忽略的两项 0.14.0 夹具未重复运行。

## [0.13.2] - 2026-11-06

- Node 模板按实际框架版本查证身份、能力及参数布局，移除硬编码的首项 Config 主键要求；要求业务指导随节点实际契约更新，skill 独立维护。
- README 区分源码与可安装版本，安装示例查询发行记录，补充同版本文档与 Windows GNU 构建入口；不改变运行时 API、打包布局或支持的目标。
- 验证：23 项单元测试、2 项真实 registry 模板初始化/编译集成、格式及源码包归档通过；未重复 Windows DLL 加载或硬件测试。

## [0.13.1] - 2026-11-05

- 补齐 batch、format 的框架来源校验，统一组件清单并验证其与工作区框架库集合一致；逐组件覆盖精确版本、别名、覆盖和锁记录。
- 验证：对应组件与工作区默认/全特性回归、严格 Clippy、Rust 1.89 和 Release 构建通过；完整范围见根 CHANGELOG 0.22.0。

## [0.13.0] - 2026-11-03

- 新增 --init，在已有目录创建节点模板；预检全部模板文件及锁文件，拒绝覆盖冲突，保留其他文件。
- 模板项目规则要求业务编码前提交方案和计划并等待确认；方案尽量附 UML 类图。

## [0.12.1] - 2026-11-01

- 新建 Node 项目默认生成根目录 AGENTS.md，填入节点名称、包名、选定框架版本和调用说明路径，提供精简的依赖、宏契约、features、租约、生命周期和验证交付规则。
- README 增加开发规则入口；不要求节点拥有相邻框架源码、专用检查脚本或钩子，继续拒绝覆盖任何已有项目目录。
- 验证限定为模板单元测试、生成内容与现有目录保护、工具格式及版本门禁；不重跑框架或设备验证。

## [0.12.0] - 2026-11-01

- 增加独立 --build 与显式 --pack、--node 和 --arch；保留旧位置参数、--new、--check 用法。
- 初始化支持 --miresidian-version/--mires-version，默认由 Cargo 查询最新未撤回正式版，选定后精确锁定；离线须显式选版。
- 模板按入口/业务职责拆分并补齐关键注释，生成有引号参数与配置选择的烟测流程；构建使用 --locked。
- 明确目标缺失诊断、构建和交付职责及残留架构提示；不修改框架 API。
- 验证：格式、默认/全特性 Clippy 与工作区测试、显式 registry 模板编译、Rust 1.89、Release 和 Rustdoc 均通过。
- 端到端实测默认最新版节点的 Linux 原生 C ABI 回显，以及 Windows x64/x86 构建、导出、manifest 与 SHA-256；未在本地 Windows 原生加载。

## [0.11.0] - 2026-10-30

- 新增 --new 最小单实例回显模板；默认依赖已发布框架 =0.14.0，Cargo 生成锁文件，拒绝覆盖既有目录。
- 新增 --check 结构检查，核对精确 registry 依赖、锁文件、cdylib、节点声明与交付文档；不构建或加载 DLL。
- 按命令解析、节点发现、校验、构建、文件收集和清单生成拆分私有模块，保留原位数/节点名命令及清单格式。
- 依赖契约按节点实际解析图确定；收集产物使用 Cargo target_directory，支持自定义输出目录。
- 新增 TOML 直接依赖，复用工作区已锁定的 1.1.8 清单解析器；避免手写清单解析，已纳入 MSRV 与静态检查。
- 验证：19 项单元测试与显式 registry 模板集成测试；工作区格式、Clippy、默认/全特性测试、Release、MSRV 和文档检查；实际双位数 Windows 打包与导出检查、清单及校验和核对、Linux 原生 ABI 回显。Windows 原生加载未在本地执行。
- 版本影响仅为 miresidian_pack 0.10.0 → 0.11.0；框架、宏、实例、ABI 及其他工具保持原版本；不发布 crates.io。

## [0.10.0] - 2026-10-30

- 已发布版本；与公开 0.10.0 的运行时代码一致，不因根框架升版而发布新版本。
- 此组件独立管理语义版本，只在实际变化时升版及单独发布；历史统一列车编号不构成联动要求。
