# 统一 JSON 请求 C ABI 方案

日期：2026-11-06。状态：实施与验证完成；本地提交/标签结果以 Git 记录与最终报告为准。

## 目标与授权

用户确认新增入口命名为 `miresidian_invoke_request`，统一 function/config/input，platform 使用可选数值；省略 platform 时参考 miresidian_invoke，由既有初始化 Config 选择平台。用户“可以，就这样安排吧”授权实施、必要验证及按仓库规则本地提交/标签。无远端推送、crates.io 发布、独立节点升级或实际设备操作。

工作区已有 `plans/2026-11-06-ate-platform-operation-improvements.md` 未提交修改，与本任务无关，保持原样且不纳入提交。现有 hooksPath 为 .githooks，继续使用。

## 现状与版本评估

根框架 0.25.2、macro 0.22.0、abi crate 0.14.0、C ABI v7。旧入口在 ffi_runtime 将三个文本参数转换为 Invocation；宏生成两个调用导出和 ABI 版本查询，运行时注册表支持数值平台与 Config 文本选择两条路径。

新增公开 Rust FFI API、宏导出和 C ABI 契约分别将根升至 0.26.0、macro 升至 0.23.0、abi crate 升至 0.15.0，C ABI 升至 8。仅同步根的实际内部精确依赖和 workspace Cargo.lock。其他 crate、工具、MSRV、租约/资源协议和独立节点依赖不变。独立节点/烟测锁文件不依赖本地工作区新版本，无需更改，不使用本地覆盖。新版本为源码候选，未发布前文档不指导用户安装候选。

## 契约与实现

新增 `const char *miresidian_invoke_request(const char *request_json)`。旧入口保持签名、语义及导出。新入口由 node 宏生成并调用根 ffi_runtime::invoke_request，共用同一 OnceLock 运行时、异常隔离、响应构造和线程局部响应缓存。

request_json 必须是非 NULL、以 NUL 结尾的 UTF-8 C 字符串，内容为单个 JSON 对象：

- function：必填非空字符串，按原操作名精确分发，不 trim。
- config/input：可省略，默认各为 []；提供时必须为字符串数组，按声明顺序排列，不接受 null 或双重序列化的数组文本。
- platform：可省略；提供时必须是 JSON 整数字面量且位于 u32 范围，拒绝字符串、布尔、null、小数、负数及溢出。1 为 TB 内置平台，2..65535 为当前保留编号，第三方从 65536 开始，须已注册。0/保留值报告 INVALID_PLATFORM_ID，合法未注册值报告 UNKNOWN_PLATFORM_ADAPTER。格式/类型错误报告 INVALID_ARGUMENT。
- 拒绝未知字段与重复字段。使用 serde Deserialize 结构体及 deny_unknown_fields；platform 使用字段级反序列化区分缺省和显式 null。请求字段顺序无约束。

省略 platform：构造 Invocation 后复用 runtime.invoke；初始化保留平台适配 Config 与原默认入口相同，不默认补 1。提供 platform：复用现有显式平台解析/注册表选择和 runtime.invoke_with；初始化不发布/不接受平台适配选择器。抽取最小的已解析请求分发辅助函数供旧显式入口与新入口共用，不建立新调度器，不改变资源/实例/超时/取消语义。请求结构与解析保留在 interop 下私有请求模块，避免继续扩大单个 FFI 文件。

响应仍为 status/code/message/output，错误输出 []；请求校验及执行 panic 不得穿过 ABI。输入内存归调用方，仅调用期间可读；返回文本归 DLL、不得修改/释放，当前线程任一三个调用入口的下一次调用、线程退出或 DLL 卸载会使此前响应指针失效，调用方应立即复制。

## 影响文件与步骤

1. 本方案先落盘；随后提升根/macro/abi 清单、ABI 常量及元数据，Cargo 生成锁文件。
2. `src/interop/ffi_runtime.rs`、私有请求解析模块、`macro/src/node/ffi.rs`：新增入口、共用分发与 Rustdoc；`abi/api/miresidian.h`、`abi/src/lib.rs`：签名、完整契约、版本和头文件一致性断言。
3. `tests/ffi.rs` 及独立请求入口集成测试、`tests/c_abi_smoke.c`：最小有意义回归；宏生成节点直接调用新导出证明生成链，工作区 DLL 示例验证实际符号。使用工作区示例而非创建依赖未发布版本的独立节点。
4. 根中英文 README、abi/macro README、docs/c-abi.md、宏驱动开发、架构、编码规范中的当前入口说明、docs/versioning.md 及新版本发行/迁移说明同步；更新三个组件各自带日期 CHANGELOG，历史版本归档不改写。TestFlow 仍可调用保留旧入口。只读核对发现 Pack 0.13.2 的 new/init 明确只接受 ABI v7，既有节点模板说明也固定 v7；本次不扩展为工具模板升级或发布，在迁移说明中明确其不能创建目标 ABI v8 的节点。已有节点在候选发布后手动升级时按本任务契约更新交付说明。
5. 验证、记录证据；只暂存本任务文件并按版本门禁提交；创建/核对三个组件不可移动标签。

## 验收与停止条件

- 请求入口与旧默认/显式入口的查询、初始化、业务、关闭输出一致；省略 platform 可从 Config 选择自定义适配器，显式数值也能选择自定义适配器；旧入口与新入口共享实例状态。
- 缺省数组、字段乱序、中文/转义可用；NULL/非法 UTF-8/非法 JSON/非对象/缺失或空 function/错误字段类型/非字符串元素/显式 null/重复和未知字段/平台数值边界均返回统一错误响应；校验失败不得执行业务；panic 捕获可复用现有执行边界并加请求入口回归。
- 新旧三个入口均复制响应后使用，不解引用已失效指针；通过交错调用验证有效响应和共享运行时，不以重复分配地址判断生命周期。
- ABI crate 测试、相关 FFI/响应/平台和宏生成回归通过；默认/全特性根库和相关集成检查，fmt、受影响组件 Clippy、Rustdoc，C/C++ 头文件编译和工作区示例 DLL 动态加载及调用新旧符号。按实际耗时只扩大到直接相关问题；Windows 未实测不得宣称通过。
- `python3 tools/check_versions.py --base HEAD`、暂存门禁及提交后根/组件 require-tag 通过；标签分别 miresidian-v0.26.0、miresidian_macro-v0.23.0、miresidian_abi-v0.15.0，不移动已有标签。
- 仅本地源码交付，不发布候选、不改外部节点，不宣称 Windows 或硬件验收。

## 实施与证据

2026-11-06 首轮实现：请求解析、共用分发、宏导出、ABI v8 常量/元数据/头文件和版本同步已完成。默认特性下 ffi（8）、ffi_request（5）、response_contract（5）、c_header（1）共 19 项通过，证明 Config 自定义平台选择、显式数值、严格请求校验、panic 与三个入口共享状态。

实施补充决策：Pack new/init 的 v7 限制属于现有工具边界，本次没有创建节点或发布框架，不修改其版本/模板。迁移资料明确这一限制，不再笼统声称所有工具流程均可无需调整使用 v8。此项不改变新 ABI 的功能、版本或验收目标；工具后续适配须另立版本方案。

最终验收证据：

- `cargo check --offline -p miresidian --lib` 通过；Cargo 仅更新 workspace 锁文件中的根/macro/abi 三个版本。独立烟测及相邻节点锁文件未修改。
- 默认与全特性执行 `cargo test --offline --locked -p miresidian -p miresidian_abi -p miresidian_macro [--all-features] --lib --test ffi --test ffi_request --test response_contract --test macro_compile_fail --test method_registry --test panic_recovery --test config_roles_explicit --test concurrent_execution --test parameter_defaults`，分别 32 和 54 项通过；两种配置的 trybuild 分别通过 24 和 31 个编译夹具（含失败诊断与成功编译）。证据为 tests/ffi_request.rs 与既有测试，执行日志 /tmp/miresidian-request-{default,all-features}-tests.log。
- `cargo fmt --all -- --check`，上述三个组件的默认/全特性 `cargo clippy --offline --locked ... --all-targets -- -D warnings` 均通过。
- `RUSTDOCFLAGS='-D warnings' cargo doc --offline --locked -p miresidian -p miresidian_abi -p miresidian_macro --all-features --no-deps` 通过；根 all-feature doctest 1 项通过。候选本地文档位于 target/doc，未把它当作 docs.rs 已上线。
- `tests/c_header.rs` 的 C11 编译通过；同一 tests/c_abi_smoke.c 使用 c++17、x86_64-w64-mingw32-gcc、i686-w64-mingw32-gcc 的 -Wall -Werror 编译均通过，证明头文件类型可用于对应宿主编译，不代表 Windows DLL 已加载。
- `cargo build --offline --locked --example execution_dll --features instances` 通过；Python ctypes 加载 target/debug/examples/libexecution_dll.so，核对版本 8 和四个导出，查询新旧响应等价，省略/显式 platform 分别完成 InitDevice → 新旧 Record 交替计数 1/2/3 → CloseDevice，并覆盖 NULL/非法 UTF-8/平台/类型错误及全局清理。脚本 /tmp/miresidian-request-dll-smoke.py 输出 PASS。
- 代码/指导文档按请求字段、平台注册、共享状态、异常和响应生命周期逐项核对。已增加 -0、浮点/指数、溢出、显式 null、重复字段及未知字段回归；不通过读取失效指针验证生命周期。
- 工作区 `python3 tools/check_versions.py --base HEAD` 已通过；暂存门禁、提交与三个不可移动标签核对按方案执行，实际提交号/标签由最终报告给出，不在提交前虚构结果。

验证边界：未执行完整无关工具/组件测试或重复资源代理验证；未发布/推送，未升级独立节点，未验证候选 registry/docs.rs、Windows DLL 加载或硬件。Pack v7 模板限制明确保留，未宣称其 new/init 可创建 v8 节点。原有其他方案未提交修改不纳入本次提交。

## 推送与发布阶段（2026-11-06，用户追加授权）

用户明确要求“推送和发布”，覆盖当前分支与本次三个版本标签推送、仅这三个 crate 的正式 crates.io 发布、发布后资料及 provenance 核对。此前“仅本地交付/不发布”描述上一阶段边界，本节取代其作为后续执行依据；不授权独立节点升级、工具发布或真实设备操作。

只读预检：工作区干净，当前 master 为 eb5b01d（前序任务纯方案归档），发行提交和三个不可移动标签均为 f356435f89ff6fd9c16d7273af13c5c933c927e0。远端 master 为 3d7c2da，尚无本次三个标签；2026-11-06 crates.io API 查询 miresidian_abi 0.15.0、miresidian_macro 0.23.0、miresidian 0.26.0 均返回 404。已有实际验证证据可复用，不重复运行整套框架/无关工具测试。

执行顺序与实现：

1. 先保存本阶段方案，版本影响：无，仅补充授权、发布步骤与验收记录，不改变发行源码或指导契约；保持现有产品版本/标签，按纯方案提交规则提交本文件。
2. 创建 `/tmp/miresidian-publish-0.26.0` 干净 detached worktree，固定 f356435；在该版本快照执行根/宏/ABI 的 require-tag 门禁，不要求纯方案 HEAD 对齐旧标签。`cargo package --list` 检查三个组件实际包文件，根包含请求模块、头文件来源和技术文档，子包包含各自 README/CHANGELOG/源码。
3. 将当前 master 与本次三个标签显式原子推送到 origin，禁止 force 或推送所有无关标签；核对远端分支及标签解引用修订。历史标签不改写。
4. 在固定发行提交的干净 worktree 中先正式 `cargo publish -p miresidian_abi --locked`，再 `cargo publish -p miresidian_macro --locked`，最后 `cargo publish -p miresidian --locked --all-features`。保留 Cargo 正式包编译验证，不使用 allow-dirty、no-verify，也不重复等价 dry-run；依赖 registry 可解析后再发布根，失败先核对上传状态后重试，凭据不读取/不输出。
5. 分别核对 crates.io 精确版本未撤回、官方 checksum、下载包 README/CHANGELOG/规范化依赖/`.cargo_vcs_info.json` 的修订与非 dirty 状态、GitHub 精确标签源码入口，以及 docs.rs 对应版本 README/API 和可选 format 能力。宏额外核对新导出生成源码，ABI 核对头文件与常量 8，根核对请求解析及 Rustdoc 说明。候选 README 描述原实现阶段，实际发布事实由本阶段结果补充，不为更改发布措辞反复补丁升版。
6. docs.rs 初次排队/404 时记录真实状态并作有限复查，不将上传成功当作在线文档生成成功；结果与遗留限制记入本方案，纯方案归档提交并推送。最终报告三个版本/标签、发行提交、registry 与文档实际可用性。

验收与边界：仅发布 ABI 0.15.0、macro 0.23.0、根 0.26.0；registry 精确包与 Git f356435 一致且保留正式验证；远端标签不可移动。Pack new/init 的 v7 模板限制仍保留，Windows 加载/硬件无新增验证。当前发布阶段不修改技术代码、依赖或产品版本；若发现真实发行缺陷，先修订方案、按实际影响升版而不覆盖旧发行。

执行结果待补充，不将预检记为已推送或已发布。


### 发布执行结果

- 方案归档提交 815af8a；master 与三个本次标签已原子推送。远端实读 master 为 815af8a4d13b61b78d60835ca641c1d5e7ee4fc8；三个标签解引用均为发行提交 f356435f89ff6fd9c16d7273af13c5c933c927e0。后续纯结果归档只推进 master，不移动发行标签。
- `/tmp/miresidian-publish-0.26.0` 固定发行提交的干净检出中，三个组件 require-tag 均通过。cargo package --list 核对 ABI 8 文件、macro 21 文件、根 198 文件，必需 README/CHANGELOG/契约/源码均存在。
- 正式 cargo publish 依次完成 ABI 0.15.0、macro 0.23.0、根 0.26.0；全部经历 Packaging → Verifying/Compiling → Uploading → Published。根以 --all-features 从 registry 使用新 ABI/macro 编译验证成功，没有跳过 verify、使用 dirty 或执行重复 dry-run。日志为 /tmp/miresidian-request-publish-{abi,macro,root}.log。
- 从 crates.io API 确认三个精确版本均未撤回，下载 static.crates.io 实际发行包并比对官方 SHA-256；三包 `.cargo_vcs_info.json` 均指向 f356435f89ff6fd9c16d7273af13c5c933c927e0，非 dirty。ABI 包元数据/头文件/常量为 8，宏包包含新导出生成源码；根规范化依赖为 registry ABI =0.15.0、macro =0.23.0，无本地 path/git，docs.rs all-features 配置保留。各包 README、日期 CHANGELOG、根技术手册及请求解析源码均已逐项核对。

| crate | 版本 | registry 实际包 SHA-256 |
| --- | --- | --- |
| miresidian_abi | 0.15.0 | 5b7010fe5b0a5aca7a0e614a435032f5f8084195600e547fc32dd45f3c0ff981 |
| miresidian_macro | 0.23.0 | 85372e116f49d2a89c869c148fbac9fd9bffccafa457ba49b7746b9042bffd0c |
| miresidian | 0.26.0 | d29e5730904ec03e6c4357a8c447c600959564cc63902d01c3dc4b34d63a8c3f |

在线资料首次核对：ABI 0.15.0 的 README/API、macro 0.23.0 的 README/node API/新入口生成源码均 HTTP 200，含精确版本与预期入口；三个 GitHub 精确标签源码/手册入口 HTTP 200。根 0.26.0 的 docs.rs README、根 API、format 可选能力首次仍为 404，暂不记为生成成功，继续有限复查。初次状态保留于 /tmp/miresidian-request-docs-results.json。

发布不改变既有本地回归证据：默认 32 项、全特性 54 项相关测试、Clippy/Rustdoc/头文件/Linux 动态库烟测已通过；没有新增 Windows DLL/硬件验证或独立节点/工具发布。


最终在线资料复查：根 0.26.0 的 [README](https://docs.rs/crate/miresidian/0.26.0)、[API](https://docs.rs/miresidian/0.26.0/miresidian/) 与 [format 可选能力](https://docs.rs/miresidian/0.26.0/miresidian/format/index.html) 均 HTTP 200，精确版本为 0.26.0，README/根 API 含 miresidian_invoke_request。首次 404 属于刚发布后的生成延迟，未记为首次成功；最终结果保留 /tmp/miresidian-request-docs-final-results.json。

[ABI 0.15.0 常量 API](https://docs.rs/miresidian_abi/0.15.0/miresidian_abi/constant.MIRESIDIAN_ABI_VERSION.html)、[ABI README](https://docs.rs/crate/miresidian_abi/0.15.0)、[macro 0.23.0 node API](https://docs.rs/miresidian_macro/0.23.0/miresidian_macro/attr.node.html)、[macro README](https://docs.rs/crate/miresidian_macro/0.23.0) 与 [macro 新入口生成源码](https://docs.rs/crate/miresidian_macro/0.23.0/source/src/node/ffi.rs) 已实际访问并核对。三个版本 registry 可用、真实发行包内容/来源一致、不可移动标签已推送、精确在线资料可用，本次发布验收完成。纯方案结果提交的版本影响为无，保留发行提交和版本标签，不为对齐结果归档 HEAD 移动标签。
