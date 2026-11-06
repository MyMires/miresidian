# 框架注释与文档一致性修复

日期：2026-11-07。状态：注释/文档修复、六组件发行及精确在线资料验收已完成；源码发行提交为 7138801，纯方案归档只推进 master，不移动版本标签。实现授权为用户“注释和文档问题一起修复”，发行授权见后文。

## 目标与设计

将根框架、过程宏及工作区示例中的英文说明统一为中文，并补全宏生成、条件编译、执行器、取消、模块加载引用、初始化回滚及任务生命周期的关键约束。保留 Rustdoc 的标准 `# Safety` 标题、`SAFETY:` 标记、API 名称和协议标识，不用逐行复述代码扩充篇幅。

修复已确认的七类资料问题：发行状态仍停留在候选阶段；CLI/Pack 新建流程未就近说明 ABI v7 限制；架构图遗漏 Direct/Shared 分支；业务默认值变更误记为 0.23.0；new/init 目录要求混写；工作区数量/依赖图过期；instances 示例版本落后。历史 CHANGELOG 与不可移动标签保留，通过新版本勘误及当前手册补充，不改写已发布历史。

实现只改变注释、Rustdoc、指导文档及其必要版本元数据，不改变 Rust/C 签名、业务行为、模板实现、ABI/协议或外部节点。Pack 的 v7 限制如实说明，给出已发布 v7 框架的显式命令示例；本任务不将其扩展为 v8 模板开发。

## 实现方式与影响文件

- 根 `src/**/*.rs`、`macro/src/**/*.rs`、`examples/**/*.rs`：核对英文注释、过简的关键边界及陈旧 ABI 信息；注释按实现补齐，模块加载器的 unsafe 前提逐项说明。
- `README.md`、`README.en.md`、`docs/README.md`、`docs/architecture.md`、`docs/宏驱动开发.md`、`docs/c-abi.md`、`docs/releases/0.26.0.md` 和新 `docs/releases/0.26.1.md`：区分 0.26.0 的已核实发行与本次未发布源码，修复迁移版本、图及导航。`docs/versioning.md` 同步组件表并补记本次评估；`docs/coding-standards.md` 明确中文说明与内容审查要求。
- `abi/README.md`、`macro/README.md`、`features/instances/README.md`、`tools/miresidian-pack/README.md`、`tools/miresidian-cli/README.md`：同步实际版本、组件职责与工具边界；依赖/安装示例明确采用已发布版本，不将本次候选写成已可安装。
- 上述六个组件的 `Cargo.toml`、`CHANGELOG.md`、根双语 CHANGELOG、`Cargo.lock`、`tools/miresidian-cli/toolset.toml`：同步独立补丁版本及内部精确依赖。其他锁文件仅在其实际依赖受影响时由 Cargo 更新，独立节点 registry 依赖不变。
- 不修改 Node skill 的任一副本；本次没有 Node 开发流程或 skill 内容变更。

实施范围补充（修改前记录）：全库英文说明扫描发现 `features/instances/src/dispatch.rs` 仍有广播源规则的英文注释；将其纳入中文说明修复并核对源通道无效时整批跳过的边界。instances 已因 README 升至 0.16.6，无新增组件或版本联动；继续保持非注释代码不变并运行该组件现有测试。

## 版本评估

交付指导及 Rustdoc 修复按仓库门禁分别补丁升级：根 0.26.0 → 0.26.1；macro 0.23.0 → 0.23.1；ABI crate 0.15.0 → 0.15.1；instances 0.16.5 → 0.16.6；Pack 0.13.2 → 0.13.3；CLI/套件 0.3.2 → 0.3.3。ABI crate 仅修 README，C ABI 仍为 v8，头文件及元数据协议号不变。CLI 内嵌清单同步 Pack 0.13.3。codec、format、batch、lease、TestFlow 无实际修改，不跟升；ABI/租约/资源协议和 MSRV 不变。原有外部节点及烟测夹具保持已发布依赖与锁文件。

当前可安装示例采用先前正式发行的根 0.26.0、ABI 0.15.0、instances 0.16.5；Pack 新建示例采用已发布且为 ABI v7 的根 0.25.2。实施中先通过官方 registry 与精确文档/源码复核这些示例。新增源码候选与已发布版本分别说明，本任务不推送、不发布。

## 步骤与验收

1. 确认现有 .githooks 已启用，落盘方案；核对实际版本、关键源码和官方版本入口。
2. 先修当前手册、组件说明与宏/执行器/示例注释，版本与归档同次更新；实施若需改变范围，先修改本方案并说明原因。
3. 用 Cargo 离线更新工作区锁文件，核对仅六个第一方版本变化；运行 `python3 tools/check_versions.py --base HEAD`。
4. 检查 Markdown 围栏、本地链接和锚点；检查根/宏/示例剩余英文说明（排除代码、专名和标准 Safety 标记）；对 Rust 源码去除注释后与 HEAD 比对，证明除版本元数据外可执行代码不变。
5. 运行格式检查、六组件严格全特性 Rustdoc、根 doctest、相关示例编译；运行宏/ABI/实例及工具现有定向检查，复用前轮默认解析/format 回归，无业务改动不扩展成硬件或完整平台验证。CLI 内嵌版本清单已变，须运行其真实路由/清单测试。验证结果记录本方案与本次修订。
   验收补充（执行前记录）：为直接验证新文档中的精确命令，通过本次 CLI/Pack 在 `/tmp` 临时目录各执行一次显式框架 0.25.2 的 new/init，检查生成的精确 registry 依赖、锁文件及保留原文件，再用 Cargo 编译。仅作为框架资料的临时验收，不创建交付 Node、不引入本地依赖或修改现有节点；实现未变，不重复另跑默认忽略的 0.14.0 registry 集成夹具。
6. 核对差异、暂存并通过钩子提交，根由钩子创建新标签；逐一创建另五个组件不可移动标签，再用 `--require-tag` 核对全部六组件。最终报告版本、提交、标签、验收与未发布边界。

完成条件：七类问题都有明确修正，关键中文注释能解释职责及约束，验证通过且仅注释/文档/版本变化；不得将工具 v8 模板、Windows 加载、硬件或本次 registry 发布写成已完成。

## 验收记录

- 七类问题逐项修正，根/宏/实例及示例的中文说明补全；模块加载器逐 unsafe 说明前提，保留 `# Safety` 和 `SAFETY:`。普通业务默认值迁移明确为 0.22.0，图中 Direct/Shared、共享所有者和引用侧租约与当前字段核对。文档索引补齐八库及配置身份专题。
- 2026-11-07 官方 crates.io API 复核 miresidian 0.26.0、0.25.2、miresidian_abi 0.15.0、miresidian_instances 0.16.5，均返回 200 且 yanked=false。先前同日已核对 0.26.0 根 API/format 页面 200；当前指导使用明确已发行示例，新六组件版本均未发布。
- Cargo 正常刷新工作区 Cargo.lock。逐包比较与 HEAD 的锁记录，只有根、宏、ABI、instances、Pack、CLI 的版本字段变化，第三方依赖、其他组件及独立 Node/烟测锁文件不变。`python3 tools/check_versions.py --base HEAD` 通过，暂存钩子及提交后六组件标签检查按步骤执行。
- 30 个修改过的 Rust 文件逐一与 HEAD 比较：去除整行注释及空行后，非注释源码逐字符一致。根/宏/codec/ABI/features 源码与 examples 的说明注释扫描无整句英文残留，排除 Rustdoc 代码围栏、Safety 标题和专名。`cargo fmt --all -- --check` 与 `git diff --check` 通过。
- `cargo test --offline --locked -p miresidian_macro -p miresidian_abi -p miresidian_instances -p miresidian_pack -p miresidian_cli --all-features`：43 项通过，包括 ABI 2、宏 3、instances 5、Pack 23、CLI 双入口单元 6 及真实进程 4。默认忽略的两项 0.14.0 registry 模板夹具未运行，改用下述文档实际命令验证。日志 `/tmp/miresidian-docfix-component-tests.log`。
- 六组件 `RUSTDOCFLAGS='-D warnings' cargo doc --offline --locked ... --all-features --no-deps` 通过；根全特性 doctest 1 项通过，`cargo check --offline --locked -p miresidian --examples --all-features` 六个工作区示例通过。日志 `/tmp/miresidian-docfix-docs-examples.log`。根公开 API 新增说明后已重新构建；未对可执行代码改动进行隐瞒或减少必要验证。
- 通过本次构建的 CLI 0.3.3、Pack 0.13.3，在 `/tmp/miresidian-docfix-online-d8zf78oj` 实测 `new ... --miresidian-version 0.25.2` 与已有目录的 `init ... --miresidian-version 0.25.2`，均成功。两项目均为精确 =0.25.2、真实 registry 来源及 checksum；共享临时 target 编译通过，CLI check 的项目和 TestFlow 静态检查通过，init 保留原 retained.txt。首次离线尝试因未缓存 0.25.2 下载失败，保留残留目录未冒充成功；按工具规则在新目录联网正常下载后完成。日志 `/tmp/miresidian-docfix-template-smoke.log`。这些是临时验收项目，未引入框架本地覆盖或修改现有独立节点。
- 最终 23 份受影响 Markdown 的围栏及 251 处本地路径/章节锚点全部通过，索引与本记录均纳入检查。最终 30 个 Rust 文件非注释源码再次核对一致，格式、差异及版本门禁通过。
- 复用同会话此前 codec/format 及默认参数/format 宏组合的 34 项通过证据；对应代码与实际依赖未改，未重复整套运行时、Clippy/MSRV/Windows/硬件检查。本次没有 Node skill 修改、工具 v8 模板功能、Windows 原生加载、真实硬件、推送或 registry 发布。

## 推送与发布阶段（2026-11-07，用户追加授权）

用户明确要求“推送和发布”，覆盖当前 master、本次六个不可移动版本标签及六个升版 crate 的 crates.io 正式发布，发布后核对实际包、来源与精确在线文档。此前“不推送、不发布”“未发布”记录原实现阶段的事实，本节取代其作为后续操作依据；不授权升级独立 Node、发布无关组件、增加 v8 模板功能或执行硬件操作。

发行源码固定为 71388012c0a3397075a6de081fab5226b760fca5，六个标签均已指向该提交，工作区干净；远端 master 预检为 1ead60220c35204de09fdde170291a34af111de5，尚无本次六个标签。现有验证可复用，发行阶段不重复整套无关测试，也不为记录“已发布”改写历史标签或反复提升产品版本。

### 执行设计与步骤

1. 先归档本阶段方案，纯方案提交，版本影响为无：仅增加已授权的发布步骤与验收，不修改发行源码、交付指导、版本或锁文件。后续结果也只补入本方案。
2. 在 `/tmp/miresidian-publish-0.26.1` 创建固定 7138801 的干净 detached worktree，逐一核对根及五个组件的 require-tag，检查六个正式 Cargo 包文件清单。发布用该发行快照，不要求后续纯方案 HEAD 对齐原标签。
3. 显式原子推送 master 与本次六个标签；不使用 force，不推送所有无关标签。实读远端分支及标签解引用，确认六标签仍为发行提交。
4. 查询六个 crates.io 精确版本。未发布时先 ABI 0.15.1、macro 0.23.1、instances 0.16.6，再根 miresidian 0.26.1；工具按 Pack 0.13.3 后 CLI 0.3.3。正式命令为 `cargo publish -p <crate> --locked`，根加 `--all-features`。保留 Cargo 正式打包和 registry 依赖编译，不使用 allow-dirty/no-verify，不重复等价 dry-run。若版本已存在，先核对来源与 checksum，匹配才跳过上传；发布失败先核实上传状态再决定重试，不读取或输出凭据。
5. 从官方 API 确认六版本未撤回，下载实际 .crate 对比官方 SHA-256，核对 `.cargo_vcs_info.json` 为发行提交且非 dirty；逐包核对 README/CHANGELOG、规范化元数据及必要源码。根精确依赖 ABI/macro/instances 新版本，其他依赖不变；CLI 包内套件及 Pack 要求分别为 0.3.3/0.13.3，TestFlow 保持 0.12.0；Pack/CLI 功能及模板限制保持。
6. 核对六个 GitHub 精确标签源码入口、六组件精确 docs.rs README 及库组件 API，根可选 format API、关键中文宏/任务文档及 instances 广播说明。CLI/Pack 是二进制工具，验证其精确 README、源码与包内入口/清单，不要求不存在的 library API；这是核对实际 crate 目标后的验收细化，不增加源码或版本变化。文档初次排队/404 时记录真实状态并有限复查，不把上传成功当作生成成功。发行快照中的候选措辞描述原实现阶段，正式状态以本阶段验收及精确 registry/API 为准；已发布包不覆盖。
7. 将真实结果、checksum、在线资料和剩余限制归档本方案，纯方案提交并推送 master。最终报告六版本/标签、发行提交、registry 与 docs.rs 状态。当前任务不额外创建 GitHub Release 或工具压缩包；用户授权的公开发行以 crates.io 和源码/标签为交付。

### 验收与边界

六个精确发布包与 7138801 来源一致，正式 Cargo 验证通过；远端六标签不可移动，纯方案结果提交只推进 master。保留此前 43 项组件/工具测试、1 项 doctest、严格 Rustdoc、六示例和 registry new/init 证据；Windows 加载、真实硬件和独立 Node 升级不在本阶段。任何真实发行缺陷须先修订方案，按实际影响另行升版，不能覆盖原发行或移动标签。

### 发布执行结果

- 发行提交 71388012c0a3397075a6de081fab5226b760fca5；纯发布方案提交 da5e6ea。master 与本次六标签已显式原子推送，远端实读初始 master 为 da5e6ea，各标签解引用均为 7138801。后续验收归档只推进 master，不改写发行标签。
- 干净 `/tmp/miresidian-publish-0.26.1` 上六组件 require-tag 均通过；Cargo 包清单与实际包文件逐一比对：ABI 8、macro 21、instances 11、根 200、Pack 31、CLI 19 文件，README/CHANGELOG/必要源码、头文件、模板及套件清单完整。
- 六个精确版本均已在 crates.io 正式发行且未撤回。ABI → macro → instances → 根 → Pack → CLI 顺序发布；根以 --all-features 使用 registry 新 ABI/macro/instances 依赖编译验证成功。所有组件均保留正式 Packaging/Verifying/Compiling，没有 allow-dirty/no-verify 或等价 dry-run。日志 `/tmp/miresidian-0261-publish-*.log`。
- Pack 首次上传出现网络低速超时；随后 API 首次读取仍为 404，但重试被 crates.io 索引的“版本已存在”检查阻止。再次读取 API 已为 200、未撤回，说明原上传已完成而状态存在传播延迟；未覆盖版本或继续重复上传。真实发行包 checksum、来源和全部文件核对通过。CLI 此后正常完成上传及可用性等待。失败与恢复均保留在 `/tmp/miresidian-0261-publish-results.json`。
- 六包下载自 static.crates.io，SHA-256 与官方 API 完全一致；`.cargo_vcs_info.json` 均为发行提交且非 dirty，path_in_vcs 对应各组件目录。README、带日期 CHANGELOG、Cargo.toml.orig 与发行快照逐字节一致；根内部 registry 精确依赖为 ABI =0.15.1、macro =0.23.1、instances =0.16.6，其余依赖不变且无 path/git。CLI 内嵌套件/Pack 版本为 0.3.3/0.13.3，TestFlow 仍为 0.12.0。实际归档及核对结果位于 `/tmp/miresidian-0261-published` 与 `/tmp/miresidian-0261-package-verification.json`。

| crate | 版本 | 实际发行包 SHA-256 |
| --- | --- | --- |
| miresidian_abi | 0.15.1 | 4112ae630a5b127659c7337e375a6f854a4291781e9044ff9cb401e74d5e863d |
| miresidian_macro | 0.23.1 | aebcb193516e8737471672d32b333af08e4393b36375ecc71db69876dbbc7e23 |
| miresidian_instances | 0.16.6 | ff28e309f133add43bd773a956fa4576c0dc08e6b9b7a8572b09c1c70bb6021c |
| miresidian | 0.26.1 | 887bc825a8b002d885217725dc5fa9e3b8802ce906593bb4aa10f0ecf19ad307 |
| miresidian_pack | 0.13.3 | 2a7d47659632b7418fb8926b9e225bfc0c4de910a3771bbd7eba2e5bb7572853 |
| miresidian_cli | 0.3.3 | 0e26960552aa065eb9f5197698c6ac0158f334333c041ccdbb12fafec9ebe441 |

- 精确在线资料最终 25/25 核对通过：六组件 docs.rs README、四库 API、根 format 可选 API、TaskService/NodeRuntime 中文 Rustdoc、ABI 常量 8、node 宏 API、宏与 instances 中文源码、Pack 模板及 CLI 清单源码，以及六个 GitHub 精确标签 README（与发行快照逐字节一致）。CLI 首次 README/source 为 404，有限复查后均 200，保留初次与最终记录；Rustdoc 断词按 `<wbr>` 的真实显示处理，不把断词误判成 API 缺失。结果 `/tmp/miresidian-0261-online-first.json`、`/tmp/miresidian-0261-online-verification.json`。
- 精确入口：[根 API](https://docs.rs/miresidian/0.26.1/miresidian/)、[format](https://docs.rs/miresidian/0.26.1/miresidian/format/index.html)、[任务文档](https://docs.rs/miresidian/0.26.1/miresidian/struct.TaskService.html)、[ABI](https://docs.rs/miresidian_abi/0.15.1/miresidian_abi/)、[宏](https://docs.rs/miresidian_macro/0.23.1/miresidian_macro/)、[instances](https://docs.rs/miresidian_instances/0.16.6/miresidian_instances/)、[Pack README](https://docs.rs/crate/miresidian_pack/0.13.3)、[CLI README](https://docs.rs/crate/miresidian_cli/0.3.3)。
- 发布后发行 worktree 保持干净，六组件 require-tag 再次全部通过；既有不可移动标签保留。源码候选措辞记录原实现阶段，正式发行事实由本节补充，不覆盖已发布包或为修改该措辞重复升版。
- 本阶段版本影响：无；只补充授权、发布状态及真实验收，不修改产品源码、指导契约、组件版本或锁文件。复用前述 43 项组件/工具测试、1 项根 doctest、严格 Rustdoc、六个示例及 registry new/init 实测；没有独立 Node 升级、Windows 原生加载、真实硬件、GitHub Release 或无关组件发布。
