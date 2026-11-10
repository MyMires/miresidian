# platform=0 进入运行时平台选择流程

日期：2026-11-10。状态：用户已确认采用现有 Config 选择流程；实施及本机验收完成，待本地提交与不可移动标签核对。

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
