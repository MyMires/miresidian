# platform=0 进入运行时平台选择流程

日期：2026-11-10。状态：实施及本机验收完成；用户后续授权推送与正式发布，实际发行与文档核对见末尾记录。

## 需求与目标

用户希望 platform 输入 0 时让用户在运行时选择平台，并明确选择“沿用现有 Config 流程：0 返回可选平台，InitDevice 时通过‘平台适配’选择”。因此 0 表示选择流程入口，不是具体平台编号，也不直接弹出界面。宿主读取 GetInitConfig/GetFunctions 发布的选择项并呈现给用户；InitDevice 按元数据顺序传业务配置及平台选择。

## 设计与实现方式

- miresidian_invoke_with(0, ...) 与 miresidian_invoke 完全复用现有 Config 路径；完整 JSON 请求的 platform:0 与省略 platform 等价。查询仍按 function 返回相应内容，不凭 0 改变操作名或新增响应格式。
- C 头文件、ABI crate 和根导出提供具名常量 MIRESIDIAN_PLATFORM_RUNTIME_SELECT=0。它是入口路由值，不注册成适配器；Platform::from_abi(0) 仍返回 None，并补充 Rustdoc 说明区别。
- 显式非零编号保持现有校验、适配器注册、元数据和禁止重复选择器规则；2..65535 保留，合法未注册编号仍报 UNKNOWN_PLATFORM_ADAPTER。
- 0 路径复用运行时平台选择、默认值、Config 身份、单实例/多实例和生命周期约束。InitDevice 选择缺省时保持现有默认平台；后续调用跟随 active_platform。存活实例期间切换平台继续报错，全部关闭后可重新选择。平台选择项不是业务 Config，不进入业务参数或实例键。
- 不修改宏生成、Node、工具、租约或资源协议，不增加 UI、注册入口、交互阻塞或新响应字段。

## 版本评估与影响文件

公开调用契约及 ABI 常量扩展：根 miresidian 0.26.2 -> 0.27.0，miresidian_abi 0.15.1 -> 0.16.0；C ABI v8 -> v9。其他 crate、宏、工具独立版本保持；根内部精确 ABI 依赖和工作区 Cargo.lock 同步。独立节点及烟测夹具继续依赖原已发布 registry 版本，不因本次源码候选跟升或使用本地覆盖。其他锁文件仅核对其真实依赖，不改无关版本。

影响文件：src/interop/ffi_runtime.rs、src/lib.rs、src/platform.rs；abi/{Cargo.toml,src/lib.rs,api/miresidian.h,README.md,CHANGELOG.md}；根 Cargo.toml/Cargo.lock、双语 README/CHANGELOG；docs/c-abi.md、docs/宏驱动开发.md、docs/coding-standards.md、docs/releases/0.27.0.md；tests/ffi_request.rs、tests/response_contract.rs、tests/c_abi_smoke.c 及必要的 FFI 回归。框架与 ABI README 明确未发布源码边界，安装示例保留已核实的既有发行，不声称候选可安装或在线文档已生成。

## 实施步骤

1. 扩展 ABI 常量、版本与头文件；实现两个入口对 0 的默认路由，更新 Rustdoc。
2. 更新回归：0 查询含全部已注册选项；通过 Config 选择自定义平台并验证后续 0/省略入口共享状态；拒绝非法选择、存活实例切换；关闭后重选；显式非零规则、保留编号、JSON 类型和四字段响应保持。
3. 同步双语说明、带日期 CHANGELOG、迁移/调用示例及真实精确依赖与锁文件。
4. 运行有针对性的默认/全特性 FFI、完整请求、响应契约和运行时测试、ABI 一致性、格式、相关 Clippy/Rustdoc、C 头文件编译和版本门禁。若发现跨组件公共行为风险，再修订方案扩大验证。
5. 记录实际验收，git diff --check、python3 tools/check_versions.py --base HEAD，暂存并让钩子检查 --staged，完成中文 Conventional Commit；创建并核对根/ABI 不可移动版本标签。仅本地提交，不推送或发布。

## 验收标准与证据

- 0 与省略平台均可查询选择器并通过 InitDevice 选定已注册自定义平台；实际业务返回证明后续调用使用所选适配器，不回退至 TB。
- 0 路径元数据中包含平台选择器，显式 1/自定义编号元数据排除选择器；选择项不影响业务配置定位。
- 存活实例切换拒绝且原实例仍可用；关闭后重新选择成功；非法平台标识、保留数值和非整数输入保持相应稳定错误码。
- ABI 常量/头文件/元数据均为 9，生成导出查询返回 9；响应缓存和内存约定不变。
- 相关验证及版本门禁通过；报告框架/ABI 版本、提交和标签。验证仅覆盖本机模拟/进程内 ABI 和头文件，不声称 Windows DLL 加载或真实硬件验收；本次不修改独立节点、不发布 registry。

实施与验收证据见下节；未验证项保留明确边界。

## 实施前范围修订：宏随包指导（2026-11-10）

核对发现 macro/README.md 将“提供数值 platform”全部描述为显式选择，并将生成 ABI 固定描述为 v8；新 0 路由生效后该随包指导会失真。遵照指导性文档随组件版本更新规则，追加该 README 的实际语义修订及带日期 CHANGELOG，将宏仅因随包说明补丁升至 0.23.2，根精确依赖/锁文件同步；不改宏源码和生成逻辑。新增宏不可移动标签核对，验证复用框架生成导出集成测试及宏单元测试。原“不修改宏生成”仍成立，“宏版本不变”由本修订替代。其他组件和工具不受影响，不要求 skill 更新或独立 Node 跟升。


## 实施与验收记录（2026-11-10）

实现按方案完成：两个带平台入口在具体编号校验前处理 0，复用默认 Config 选择路径；C/Rust 公开具名常量，root 重导出；具体适配器解析与非零规则保留。宏源码无变化，仅更新 README/补丁版本。同步三个实际受影响组件版本及根精确依赖，Cargo 自动生成工作区锁文件，仅根、ABI、宏三项版本变化；独立 smoke-node 和相邻 Node 的依赖及锁文件未改。

验收证据：

- `cargo test -p miresidian -p miresidian_abi --test ffi --test ffi_request --test response_contract --test node_runtime --locked --offline`：默认配置 FFI 8、完整请求 6、响应契约 5，共 19 项通过；node_runtime 受 instances 条件编译，此配置 0 项，未冒充运行。该命令的显式 test 筛选不运行 ABI lib，另行验证见下。
- `cargo test -p miresidian --test ffi --test ffi_request --test response_contract --test node_runtime --all-features --locked --offline`：对应 8+6+6+16，共 36 项通过。包含元数据选项、Config 自定义选型、跨入口共享业务计数、原实例在切换拒绝后可用、关闭后 TB 重选、缺省选择、类型/错误响应及旧非零规则。
- 最终将真实宏导出共享状态用例的两个带平台调用改为 0 后，分别重跑默认/全特性 `--test ffi_request`，各 6 项通过。其余等价测试复用上述结果：宏只改随包说明，生成实现未变。
- `cargo test -p miresidian_abi -p miresidian_macro --lib --locked --offline`：ABI 一致性 2 项、宏单元 3 项全部通过；生成节点的 ABI 查询集成断言为 9。
- `cargo clippy -p miresidian --lib --test ffi --test ffi_request --test response_contract --test node_runtime --all-features --locked --offline -- -D warnings` 通过。
- `RUSTDOCFLAGS='-D warnings' cargo doc -p miresidian -p miresidian_abi -p miresidian_macro --all-features --no-deps --locked --offline` 通过，本机生成三个组件 API；不代表 docs.rs 在线文档已生成。
- `cc -std=c11 -Wall -Wextra -Werror -Iabi/api -fsyntax-only tests/c_abi_smoke.c` 和对应 C++11 语法检查通过；覆盖 ABI 9 和零值常量，不声称 DLL 链接或加载。
- 修改文档的本地链接/锚点检查：12 份文件、111 个引用通过；人工核对 0 的路由语义、非零排除选择器、版本/安装边界、Config 顺序及迁移说明。`cargo fmt --all -- --check`、`git diff --check`、`python3 tools/check_versions.py --base HEAD` 均通过。

范围边界：仅本机模拟和进程内 ABI，无 Windows 原生 DLL 加载或真实硬件验收；不新增框架/工具全量回归、不改变独立节点、不推送或发布。源码候选、不可移动标签、registry 发行与 docs.rs 可用性分别看待。本轮没有查询候选的 registry/docs.rs 在线状态，也未将候选标记为已发布。

提交要求：本方案与实现、文档、版本、锁文件同次暂存并执行钩子门禁；提交后用 `--require-tag` 核对 miresidian-v0.27.0，并为 ABI/宏执行 `--component ... --tag` 与 `--require-tag`。实际提交和标签以 Git 元数据及交付回复为准，已有版本标签不移动。

## 推送与正式发布阶段（2026-11-10，用户追加授权）

用户明确要求“推送和发布”，授权当前分支、本次三个版本标签的推送和对应 crate 的 crates.io 正式发布，及后续发行验收/方案结果归档。此前“仅本地交付/不发布”是实施阶段范围，本节作为本次发布阶段的执行依据。不升级独立 Node、不发布无关工具、不扩展实际设备操作。

### 目标、设计与版本评估

固定发行提交 de9f16f（feat: 支持零值平台进入运行时配置选择），不可移动标签 miresidian-v0.27.0、miresidian_abi-v0.16.0、miresidian_macro-v0.23.2 均指向该提交。按依赖顺序先 ABI 0.16.0、macro 0.23.2，再根 0.27.0；已发布精确依赖仅核对来源，不重复发布。不改变发行源码、已归档修订或已建标签；README 中的候选描述属于原实现阶段，实际发行状态由本节验收补充。仅追加本方案的授权和验收记录，版本影响：无；不改变产品、工具或可调用/可交付契约，无新产品版本。

### 步骤与验收

1. 核对干净状态、远端分支与三个标签；通过 crates.io 官方 API 查询三个目标版本和根全部内部精确依赖，拒绝覆盖已发布版本。复用前节已通过的测试、Clippy/Rustdoc 和头文件结果，不重复等价全量回归。
2. 本方案先落盘并以纯方案提交归档，保留发行标签；在 /tmp 创建固定 de9f16f 的干净 detached worktree。该发行快照核对根、ABI、宏 require-tag 门禁；cargo package --list 核对三个发行包的 README/CHANGELOG、源码及根技术指导/头文件来源。相对引用越出子包的资料通过精确 Git 标签源码入口核实。
3. 显式原子推送 master 和三个本次标签到 origin，禁用 force，不推送全部标签；实读远端分支和解引用标签，确保修订正确。
4. 在干净固定发行 worktree 执行 cargo publish -p miresidian_abi --locked、cargo publish -p miresidian_macro --locked、cargo publish -p miresidian --locked --all-features。保留 Cargo 打包编译验证，不使用 allow-dirty/no-verify，不额外重复 dry-run。若失败先查询 registry 上传状态，再根据明确错误修正或重试。
5. 发布后从官方 API 和 static.crates.io 核对精确版本未撤回、官方 SHA-256、真实发行包 README/CHANGELOG/规范化清单/.cargo_vcs_info.json 的修订及非 dirty。ABI 包核对编号 9 和零值常量；根核对新路由及全部 registry 精确依赖，宏核对转发源码未变。核对 GitHub 三个精确标签的源码/手册入口。
6. 逐一访问 docs.rs 精确版本 README/API；根需覆盖重导出零值常量、ffi_runtime::invoke_with 说明、format 可选能力及源码入口，ABI 需覆盖新常量，宏需覆盖 node API。排队或失败时如实记录，有限复查，不以上传成功代替文档可用。
7. 将实际命令、版本、提交/标签、checksum/来源及文档可用性记录到本方案，纯结果提交并推送 master；保留不可移动发行标签指向 de9f16f。最终核对干净状态、远端分支与全部发行标签，报告真实完成项和未验证边界。

预计正式上传/编译及在线资料生成需要数分钟，docs.rs 排队可能更久。当前只读预检不代表已推送或已发布，执行结果待补充。


### 发布执行结果（2026-11-10）

- 方案归档提交 4a747a1；master 与本次三个版本标签已显式原子推送至 origin。远端实读 master 为 4a747a1a4bd14f970afbb8fddb0ad487f7b19511，三个版本标签解引用均为发行提交 de9f16f7edef28e4c223f87b89febbca513db068。后续结果归档仅推进 master，不移动发行标签。
- 固定该发行提交的干净 detached worktree `/tmp/miresidian-publish-0.27.0` 中，根/ABI/宏 require-tag 均通过；package --list 验证 ABI 8 文件、宏 21 文件、根 205 文件，必需 README/CHANGELOG/源码和根技术指导/迁移文档均在包内。
- crates.io 发布前官方 API 确认三个目标版本均不存在，根其他精确内部依赖 format 0.3.2、codec 0.16.0、batch 0.2.2、lease 0.17.1、instances 0.16.6 均已发布且未撤回，没有重复发布无关包。
- 正式 `cargo publish` 按 ABI 0.16.0 → macro 0.23.2 → 根 0.27.0 顺序全部完成 Packaging / Verifying / Uploading / Published；根以 --all-features 从真实 registry 依赖编译通过，没有 allow-dirty 或 no-verify。日志分别为 `/tmp/miresidian-platform-publish-{abi,macro,root}.log`。
- 发布后官方 API 再次确认三个精确版本可用且未撤回；下载 static.crates.io 的实际 .crate 并核对官方 SHA-256。三个包 `.cargo_vcs_info.json` 均为发行提交 de9f16f7edef28e4c223f87b89febbca513db068 且非 dirty；README/日期 CHANGELOG 与固定源码一致，规范化清单 repository 正确。
- ABI 包的元数据/头文件/常量均为 C ABI 9，含 MIRESIDIAN_PLATFORM_RUNTIME_SELECT=0；根包的内部依赖均为相应真实 registry 精确版本，无 path/git，保留 docs.rs all-features，路由源码及接入/迁移文档一致；宏转发源码与发行提交一致。GitHub 三个精确版本标签下的 README、ABI 头文件、宏入口源码及根接入/迁移手册均 HTTP 200 且字节一致。
- 真实发行验收脚本及结果：`/tmp/miresidian-platform-verify-release.py`、`/tmp/miresidian-platform-release-results.json`。源码检出和固定发行 worktree 在上传后仍干净。

官方 SHA-256：

| crate | version | SHA-256 |
| --- | --- | --- |
| miresidian_abi | 0.16.0 | `d9cb2c08e2c0380698b21dd8a44c68be640f283746ae12d27a4c99c43e47a9c2` |
| miresidian_macro | 0.23.2 | `c4535eb1bc858f8621de4a321a664f4c67bf0db00c85d3d8903685c0ee51e9af` |
| miresidian | 0.27.0 | `bcbafb43b3de54a31d9c8268930260091683aecfab73363a893f6698b7afb7ca` |

精确在线资料首次检查：三个组件 README/API、新零值常量、根 format 可选能力和源码入口共 12 页均 HTTP 404，尚未确认 docs.rs 收录/构建；精确版本 builds 页面同样 404。保留初次事实于 `/tmp/miresidian-platform-docs-initial-results.json`，不将 registry 上传成功当成 docs.rs 文档已生成。后续有限复查结果另记，不改写首次状态。

在线文档有限复查结果：两轮 12 页精确链接检查均未能确认生成；第一轮全部 404，第二轮 10 页 404、根 README/API 两页 TLS 握手超时。另对三个精确版本 README 加查询参数并请求 no-cache，全部仍为 404；构建页面亦为 404，不能据此断言构建已失败或确认正在排队。最后探测时官方 API 记录的发布时刻分别为 ABI 2026-11-10T13:24:00Z、宏 13:24:21Z、根 13:25:02Z，探测约为 13:29 UTC（北京时间 21:29）。保留 `/tmp/miresidian-platform-docs-results.json` 与首次结果；docs.rs README/API、根 format 与新常量、源码在线页面均明确为尚未确认可用，不将 registry 和 GitHub 成功替代文档可用性。

本次推送及 crates.io 正式发布完成；真实发行包、不可移动标签、GitHub 精确源码及 Cargo 包编译验证已通过。docs.rs 生成状态为待外部服务完成/可访问后再核对，属于明确保留的在线资料未完成项；未声明 Windows 加载或真实硬件新增验收。方案和验收记录的版本影响：无；仅记录已授权动作和实际结果，不改变产品源码、依赖、交付契约或已有版本。结果归档提交后仅推送 master，发行标签继续固定在 de9f16f。
