# 实例能力统一使用 instance（2026-11-12）

## 目标、授权与范围

用户已明确要求统一改名，并补充要求 crate 也改名；据此直接实施，无需再次确认。统一 Cargo feature、节点宏、实例 crate、组件目录和对应策略字段为单数 instance。不保留旧 feature/节点宏别名。既有发行、标签、历史 CHANGELOG 和历史方案的版本事实保持原样；历史文档中指向移动源码的链接可修正。独立 serial、tcp_client 与 smoke-node 仍依赖它们已发布的旧框架，不在本任务中升级或改写旧用法。此次不发布、不推送。

## 现状与版本影响

- 根框架 0.32.0 → 0.33.0：feature instances → instance，NodeDescriptor.instances → instance，宏所需隐藏入口同改名；公开 Rust 契约不兼容，提升次版本。
- miresidian_macro 0.28.0 → 0.29.0：节点级 instances/instances(...) → instance/instance(...)；参数级 instance(key/ignore) 保持，按上下文区分。提升次版本。
- miresidian_instances 0.16.8 → miresidian_instance 0.17.0：新 package/import 名和 features/instance 目录，保留版本演进与 CHANGELOG；提升次版本，不重置历史版本。
- miresidian_pack 0.13.4 → 0.13.5：发布包内框架库清单改为新名，继续校验旧包名的精确依赖、来源和覆盖；兼容修复，补丁升版。
- miresidian_cli / 套件 0.3.5 → 0.3.6：嵌入清单最低 Pack 版本改为 0.13.5，路由不变；补丁升版。
- miresidian_batch 0.3.1 → 0.3.2：README 的能力组合指导引用了 instances，必须随新名称修正；仅指导修订，补丁升版。调研确认此项后，在实施前补入方案。
- 其他 crate、TestFlow、C ABI v10、租约/资源代理协议、MSRV 不变。内部精确依赖及根 Cargo.lock 与上述实际版本同步；旧独立节点锁文件不变。
- 版本依据是本检出 Cargo.toml；新版本均为未发布源码候选，不将新包或候选写成已可安装版本。无需查询外部 API；未执行发行验收。

## 实现设计与对外契约

1. 移动 features/instances → features/instance、macro/src/node/instances.rs → instance.rs、src/runtime/instances.rs → instance.rs；根 members/default-members、path、Rust import、Pack 清单同步。现行测试文件/分组名称随能力名同步，诊断快照由实际编译验证。
2. 宏 NodeArguments、生成器和 NodeDescriptor 的策略字段统一 instance；宏仍调用相同 InstancePolicy / InstanceMode / ExecutionPolicy，隐藏门禁改为 __require_instance_runtime。旧 instances 节点声明报不支持配置，不静默接受；增加旧写法拒绝回归。保留长短写、默认单对象、multiple 显式关联、参数 instance/lease 身份规则。
3. 运行时 feature cfg 和诊断改为 instance；实例容器、配置身份、锁、Future 调度、取消、失败清理、引用计数及资源所有权均不改变；本次仅名称/版本/配套校验变化，无生命周期新设计。
4. docs/instances.md 保持既有主题 URL，内容更新为新 API；更新现行 README（含英语）、Rustdoc、架构、节点手册、调用/资源指导、CI 和 DLL 验证命令。历史发行页面保留旧 API 示例，修正失效源码链接即可。新迁移 docs/releases/0.33.0.md 提供名称映射、版本、发布前不能安装及独立节点迁移边界。
5. tools/check_versions.py 使用当前包名核对工作区与 Pack 清单，同时保留旧包名的子节点来源检查；历史快照以其自己的组件清单核对，避免新名破坏旧提交检查。Pack is_framework 同样识别旧名。用真实 Git 夹具与 Pack 多组件依赖测试覆盖新旧包名，禁止 path/git/patch 与无来源锁文件。

## 影响文件与实施步骤

1. 本方案落盘后启用已确认 .githooks 配置，执行上述目录/代码/清单/测试/CI 更名。
2. 更新六个组件版本与各自 README、带 2026-11-12 日期的 CHANGELOG；根双语修订与候选示例一致；更新 CLI toolset、根 Cargo.lock 及 Pack/版本检查器历史兼容检查。
3. 更新现行手册与迁移说明；扫描旧名称，逐项区分刻意保留的历史记录、旧节点以及旧名拒绝/来源校验用例。
4. 验证后回填本方案证据，执行版本门禁 --base HEAD、暂存门禁、提交；创建并核对根/宏/实例/Pack/CLI 不可移动版本标签。最终报告版本、提交、标签与验证边界。

## 验收与验证范围

- 格式和 diff 检查；默认、instance 单 feature、全 features 的根测试（含宏 UI）、宏/实例 crate 单元及集成回归；新 feature 可编译、旧 Cargo feature 拒绝；默认单实例和参数级 instance 仍有效。
- Pack、CLI 测试和 Python 版本门禁回归证明新旧包名来源约束有效、清单与锁一致；无关 TestFlow 无须重复业务验证。
- 受影响组件/目标的严格 Clippy、严格 Rustdoc 与示例编译；本次不交付新 DLL，不要求硬件或 Windows 加载实测。优先使用本地已有依赖；若环境缺失或检查受限如实记录，不以未执行记通过。
- 扫描并校验受影响 Markdown 本地链接，确保 features/instances 旧源码路径无失效链接；历史版本含旧名称为预期。
- python3 tools/check_versions.py --base HEAD、提交后 --require-tag 与五个独立组件 --require-tag 均通过，工作区无遗漏。

## 执行证据

### 实施核对

- 已完成 feature、节点宏、package/import、组件目录、运行时/宏策略字段及隐藏门禁更名；旧 Cargo feature 与节点宏明确拒绝。
- 已同步六个受影响组件版本、内部精确依赖、根锁文件、当前示例、CI、README/Rustdoc/手册、迁移说明与带日期修订。
- 已扫描残留名称：仅历史版本、旧节点夹具、主题 URL docs/instances.md、迁移/旧名拒绝及旧包来源校验保留复数。历史方案不改写；无需修改 Node skill。
- 逐文件比对实例组件 dispatch/policy/store/lib 和集成测试，除名称替换外实现一致；独立节点与夹具的源码/清单/锁文件未改动。

### 验证证据（均通过）

所有 Cargo 命令使用 --offline --locked；测试未开启 TRYBUILD=overwrite，诊断快照在单独核对定位变化后已更新。

| 检查 | 命令与结果 |
| --- | --- |
| 默认组合 | cargo test -p miresidian -p miresidian_macro -p miresidian_instance -p miresidian_pack -p miresidian_cli --all-targets；102 passed、2 个原有模板集成用例 ignored，37 项 UI 用例通过 |
| 全特性组合 | 上述命令加 --all-features；215 passed、3 ignored，48 项 UI 用例通过 |
| instance 单特性 | cargo test -p miresidian --all-targets --features instance；127 passed，45 项 UI 用例通过 |
| 文档示例 | cargo test -p miresidian -p miresidian_macro -p miresidian_instance --doc --all-features；2 passed |
| 严格静态检查 | 对上述五组件分别执行默认及 --all-features 的 cargo clippy --all-targets -- -D warnings；均通过 |
| 严格文档 | RUSTDOCFLAGS=-D warnings cargo doc -p miresidian -p miresidian_macro -p miresidian_instance -p miresidian_pack -p miresidian_cli -p miresidian_batch --all-features --no-deps；通过 |
| MSRV | cargo +1.89.0 check 对上述五组件 --all-targets --all-features；通过 |
| 版本检查器 | python3 -m unittest discover -s tools/tests -p test_check_versions.py -v；30 passed，含新旧实例包名的 path/git/覆盖/锁来源拒绝 |
| 新旧 Cargo 入口 | metadata 只有 instance feature、miresidian_instance 包且 runtime 转发准确；cargo check -p miresidian --features instances 明确拒绝旧 feature |
| 源码包 | cargo package -p miresidian_instance --list --allow-dirty，确认清单包含 README、CHANGELOG、源码与集成测试；仅检查文件清单，未发布 |
| 指导链接 | 已检查 250 处本地链接目标、48 个 Markdown 锚点，均存在；方案最终回填后再做差异/门禁核对 |
| 历史兼容 | 新版检查器在提交前执行 --require-tag，成功核对实际旧 HEAD 的 miresidian-v0.32.0，旧包工作区清单与旧节点锁来源保持可验 |

完整命令/结果及耗时临时记录在 /tmp/instance-verification-results.json，各项日志在 /tmp/instance-verify-*.log，版本测试日志在 /tmp/instance-version-tests.log。首次测试只发现更名后的诊断定位/标注长度差异及新增诊断快照，核对更新后正常运行全部通过。

原有 Pack 的两项 registry 模板集成用例继续 ignored，本次未修改模板；全特性日志另有仅供多进程父测试启动的 lease_process_helper 标记 ignored，父进程回归已通过。不交付 DLL，未进行 Windows/硬件加载实测；未执行推送、crates.io 发布或在线 docs.rs 验收。

### 提交与标签

格式/差异、工作区与暂存门禁均已通过。交付提交包含本方案；提交后由 hooks 生成根标签，再用版本检查器逐个创建并核对独立组件标签，最终提交标识与实际标签核验结果在交付回复报告，避免在同一提交中写入自身哈希。不可移动标签清单：miresidian-v0.33.0、miresidian_macro-v0.29.0、miresidian_instance-v0.17.0、miresidian_batch-v0.3.2、miresidian_pack-v0.13.5、miresidian_cli-v0.3.6。


## 2026-11-12 推送与正式发布阶段

### 授权、版本与范围

用户明确要求“那就推送和发布吧”，授权本次 master 与六个组件标签的推送，以及六个当前版本的 crates.io 正式发布及发行验收。本节取代原实施阶段“不推送、不发布”的执行边界；原候选描述保留其当时事实。不升级独立 serial/tcp_client/smoke-node，不发布无关或被取代版本，不创建 GitHub Release、工具归档或部署。

- 发行源码固定为 5fb35784e14aca005868b39f6ba8858ba77de8b3；六个不可移动标签均已指向该提交。预检远端 master 为 8e63003fdc40cc160f332d65a1881a5f889d76c8，尚无本次六标签。
- 正式清单：miresidian_macro 0.29.0、miresidian_instance 0.17.0、miresidian_batch 0.3.2、miresidian 0.33.0、miresidian_pack 0.13.5、miresidian_cli 0.3.6。
- 复核 codec 0.18.0、ABI 0.17.1、format 0.4.1、lease 0.17.4 与 TestFlow 0.13.0 的实际 registry 可用性；已有可用版本不重复发布。官方 API 预检证据存 /tmp/instance-release-preflight.json；上传前确认目标状态，已存在版本必须先核对源码和 checksum，不能覆盖。
- 此阶段仅更新本方案，版本影响：无；发布原有已验证标签快照，不修改产品/工具代码、交付指导、版本或锁文件，不因归档发行事实提升无关补丁，也不移动已有标签。包内候选措辞记录原实施阶段，正式发行事实由本节和精确 registry/docs.rs 验收记录提供。

### 实现方式与执行步骤

1. 先提交本阶段方案，提交说明明确纯方案版本影响为无；fetch 后确认远端 master 是本地祖先，逐个核对待推标签同指向。以显式 refspec 原子推送 master 和六标签，不 force、不使用全局 --tags，核对远端解引用。
2. 在 /tmp/miresidian-publish-0.33.0 创建固定 5fb3578 的干净 detached worktree；逐组件执行 --require-tag，检查正式包文件清单及必要源码/README/CHANGELOG/迁移手册/模板/套件清单。复用本方案已有测试、静态检查、MSRV 与文档证据，发布阶段保留 Cargo 正式包编译验证；不重复等价 dry-run 或无关全量测试。
3. 按宏 → 实例 → batch → 根框架 → Pack → CLI 的顺序执行 cargo publish -p <crate> --locked --all-features；每个新依赖已能由 crates.io 解析后再发布依赖方。不使用 --allow-dirty 或 --no-verify，不绕过 registry。凭据仅由 Cargo/Git 既有配置使用，不输出任何令牌。网络失败先核对实际上传状态，确认不存在后才重试。
4. 检查此次 GitHub CI 结果；直接阻断发行的失败先查证并按原规则修复，若需源码变更则先修订方案、独立升版，不覆盖已发布包或移动旧标签。因本次未交付 DLL，不额外升级 Node 或执行硬件操作。
5. 从官方 API 核对六版本均未撤回，下载各 .crate 并比对 SHA-256/checksum；.cargo_vcs_info.json 必须为 5fb3578 且非 dirty；README/CHANGELOG/关键源码与发行快照一致。检查规范化清单第一方精确依赖无 path/git，根 feature 仅 instance → miresidian_instance/runtime，宏与策略字段已改名；CLI 内嵌最低 Pack 0.13.5。
6. 核对六个精确 GitHub 标签源码和 docs.rs README/源码入口；四个库核对精确 API，根额外核对 instance feature、NodeDescriptor.instance、NodeEnum 及 format 可选 API，新实例包核对 InstanceStore 和 runtime 调度入口。Pack/CLI 是二进制工具，按实际目标核对 README/源码/内嵌清单，不要求不存在的库 API。排队或失败按实际状态记录，有限复查，不用上传成功代替文档可访问。
7. 将真实结果、来源提交、checksum、在线资料和剩余边界归档本方案，纯方案提交并推送 master，六个发行标签继续固定 5fb3578。最终报告发布版本、提交/标签、registry、文档及 CI 状态。

### 发布验收

完成条件：六个包正式 Cargo 验证/上传成功，官方精确版本和下载源码吻合，远端提交/标签正确，线上资料状态已分别确认，实际证据归档且工作区干净。尚未执行的上传、在线文档或 CI 不记作通过。发布执行结果待填。
