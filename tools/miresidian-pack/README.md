# miresidian_pack

本页对应源码候选 **0.14.0**，尚未发布。build/pack 新增业务方法文档自动生成；new/init 继续按实际框架 ABI 生成模板说明。模板支持 ABI v7–v10，调用说明按节点实际解析的 ABI 生成；未知 ABI 明确拒绝，不沿用旧签名。框架来源检查继续同时识别新旧实例包名。

[文档中心](../../docs/README.md) · [架构总览](../../docs/architecture.md)

`miresidian_pack` 提供最小节点模板、项目结构检查和 Windows DLL 发布包整理。
本页对应工具源码版本 **0.13.6**，本次未发布；可在框架仓库执行
`cargo build -p miresidian_pack --locked`，再使用生成的 `target/debug/cargo-miresidian-pack`。
工具发布需单独授权，不随根框架发布。

## 命令

```text
cargo miresidian-pack --new demo-node --name 演示设备 --miresidian-version 0.33.0
cargo miresidian-pack --check
cargo miresidian-pack --check 演示设备
cargo miresidian-pack
cargo miresidian-pack x64
cargo miresidian-pack x86 演示设备
cargo miresidian-pack --version
```

原来的 `[x86|x64] [节点名称]` 位置参数保持兼容。省略参数时构建所有发现节点的 x64 与 x86 版本。
新增入口使用长选项，`new`、`check` 等文字仍可作为原有节点名。
Cargo 必须能找到 `cargo-miresidian-pack` 二进制；也可以直接运行该二进制，省略 `miresidian-pack`。

先查 [Pack 发行记录](https://crates.io/crates/miresidian_pack/versions)，普通安装由 Cargo 选择已发布版本：

```text
cargo install miresidian_pack --locked
```

如需固定版本，使用 `cargo install miresidian_pack --version X.Y.Z --locked`，将 X.Y.Z 替换为已确认发布的版本。安装后核对 --version 和 --help，沿 [同版本资料定位](../../docs/README.md#按实际版本查阅) 查该版本说明；不要从仓库版本号推断发布状态。

## 最小模板

`--new <目录> [--name <显示名称>] [--miresidian-version <已发布版本>|--mires-version <已发布版本>]` 创建独立单实例回显项目。

模板支持已核对的 **ABI v7–v10**。省略版本时仍由 Cargo 查询最新未撤回正式版并精确锁定；当前已发布框架 0.33.0 使用 ABI v10，可使用上述显式版本或省略版本。工具先生成通用项目和真实锁文件，再通过项目检查读取 ABI 元数据，最后生成对应调用说明；成功信息包含实际 ABI。

- ABI v7–v9：调用说明使用三参数 `miresidian_invoke(function, config_json, input_json)`，平台保留配置按实际 GetInitConfig 查询，不混用各版本的平台编号语义。
- ABI v10：调用说明使用单参数 `miresidian_invoke(request_json)`，明确请求对象、默认平台 0、输入/返回内存所有权和调用前版本检查，不再生成旧签名示例。
- 未知 ABI：返回“模板创建尚未完成”，调用说明保留 ABI 待解析/待完成提示，不生成猜测签名。解析或生成失败时，新建文件保留供诊断；new 不覆盖残留目录，应在新目标重试。可在修正环境后自行完成原项目，但必须按实际头文件修订说明并单独编译和调用验证。

已有 Node 的依赖和调用说明不会被自动改写；旧项目不需要因工具升级迁移框架。

`--init [目录]` 使用相同选项在已有目录初始化，省略目录时使用当前目录。模板目标文件或 Cargo.lock 已存在时拒绝写入；其他文件保留。

new 和 init 均以目标目录末段作为 Rust package 名，目录名须符合 Rust 包名规则；`--name` 只指定节点显示名称及调用说明文件名。省略时节点名称默认使用目录名，仍生成 `#[node("目录名")]`。例如目录 `serial-node` 指定 `--name "串口节点"` 后，package 名为 `serial-node`，节点名称为 `串口节点`，说明文件为 `docs/串口节点.md`。

init 的常见冲突文件包括 Cargo.toml、Cargo.lock、README.md、AGENTS.md、CHANGELOG.md、.gitignore、src/lib.rs、src/node.rs、tools/smoke.testflow 和 `docs/<节点名称>.md`；存在时先处理冲突再运行。已有 src、docs、tools 子目录必须是普通目录，不跟随符号链接。
new 要求目标目录不存在且父目录存在；init 要求目标目录已存在。包名取目录末段，须为合法 Rust 名称，可含连字符。
默认显示名等于包名；显示名同时作为交付目录名，必须符合 Windows 文件名规则。
new 拒绝已存在的目录，即使为空；init 则按前述模板文件冲突规则预检后写入。

生成内容：Cargo.toml、src/lib.rs、src/node.rs、tools/smoke.testflow、README.md、AGENTS.md、带 UTC 日期的 CHANGELOG.md、.gitignore、
以及 docs/<节点名称>.md。节点只使用宏标注，不手写 ABI 导出或字符串分发；不引入设备通信依赖。

AGENTS.md 填入当前项目名称、包名、初始化选定的框架版本及调用说明路径，包含项目依赖、职责、生命周期、针对性验证与交付要求，并指向完整 Node skill 和实际框架版本的官方资料。模板不固化实例键位置或复制逐版本的框架能力契约。它随新项目一次性生成，之后由项目维护者补充；不要求相邻框架目录、检查脚本或 Git 钩子，也不会修改已有项目。

模板默认通过 Cargo 查询 crates.io 最新未撤回正式版，排除预发布版，不按当前 Rust 版本静默回退。
--miresidian-version 与 --mires-version 为等价参数，--framework-version 兼容保留；重复指定报错。
离线无法确认最新版本时须显式指定。选定后写入精确依赖，后续构建不会自动升级。
可显式选择至少 0.14.0 且提供 ABI v7–v10 的其他已发布版本；实际 ABI 以该节点解析结果为准。
工具调用 Cargo 生成真实 Cargo.lock，并检查解析来源与项目结构，之后才报告创建成功。
不会通过 path/git/patch/replace/paths 绕过未发布依赖。解析失败时保留新建文件以便检查，命令失败，
不得将其视为完成的项目；修复 registry 访问或版本后运行 `cargo generate-lockfile`、结构检查与编译。

生成后运行 `cargo check --locked`；模板创建不代替编译及调用验证。
Windows target、链接器及交叉编译配置由项目维护者选择。
替换业务后同步修订调用说明、版本、带日期的 CHANGELOG，并补齐业务测试。
模板只包含默认单实例；多实例、租约与硬件协议按需要显式声明。

## 结构检查

在节点项目或其工作区内执行 `--check [节点名称]`。检查与正式打包共用同一套前置校验：

- 使用 `cargo metadata --locked` 读取实际依赖图；缺少或过期的锁文件应先由 Cargo 正常生成。
- 框架依赖声明使用 crates.io 精确版本；禁止框架 path/git、其他 registry 与 patch/replace 覆盖。
- 检查调用目录祖先与 CARGO_HOME 的 Cargo 配置，拒绝 paths 覆盖；核对锁文件来源及校验和。
- 按每个节点自己的依赖图确定唯一框架与 ABI 版本，不从整个工作区任取第一个版本。
- 静态扫描 src 下的 Rust 源码，节点 crate 须声明 cdylib，且仅有一个节点声明；显示名不可重复。
- 检查显示名适合作为 Windows 发布目录；确认解析到的 ABI 包含公共头文件与 ABI 版本元数据。
- 确认选定节点的 docs/<节点名称>.md 与 CHANGELOG.md 存在且非空。

检查不构建、不加载 DLL、不生成 dist，也不改写锁文件和源码；Cargo 可能访问 registry 或下载依赖到缓存。
静态扫描识别 `#[miresidian::node(...)]` 及导入后的 `#[node(...)]`，不展开宏、别名或条件编译，
不支持以多份条件节点声明供同一 crate 选择。它不强制业务模块目录布局。
文档检查只确认文件存在、非空；类型正确性、导出符号、文档内容准确性和硬件行为仍须另行验证。

## 构建与输出

节点项目是 Cargo 项目或工作区，框架和 ABI 版本从实际解析依赖自动取得。
打包前执行上述检查；按请求调用 Cargo release 构建，再从 Cargo 实际 target_directory 收集产物，
兼容自定义 target-dir。Windows target 与链接器仍复用项目的 Cargo 配置。

输出位于 `dist/<节点名称>/`。其中 windows-x64/、windows-x86/ 分别包含 DLL 和 miresidian.h；
根目录包含节点说明、CHANGELOG.md 与 manifest.json。
清单格式保持 format_version=1，记录节点、框架、ABI、Git 修订及各 DLL 的 SHA-256。
工具不会发布 Rust crate，不执行硬件测试，也不保证目录中不存在以前生成的其他位数文件；
本次清单 artifacts 只描述本次构建收集的产物。

## 业务方法文档自动生成

执行 `cargo miresidian-pack --build --arch x64` 或 `cargo miresidian-pack --pack --arch x64`，工具会在所选节点的全部目标编译成功后，自动更新 `docs/<节点名称>.md`；pack 随后把同一文档复制到 `dist/<节点名称>/<节点名称>.md`。统一 CLI 的 build/pack 也走这条流程。无需调用 AI、另跑文档整理命令或安装 nightly；直接 `cargo build` 不执行 Pack 的生成步骤。新工具安装前按上文核对 registry 的实际可用版本，不能将本源码候选当作已发布版本。

详细业务说明写在标记 `#[miresidian::operation]` 或导入后 `#[operation]` 的方法上，Rust 可见性为私有也会提取；初始化、关闭和探测方法同样适用。例如：

```rust,ignore
/// 发送请求并返回完整响应。
///
/// # 参数与返回值
/// `request` 来自 Input，按 UTF-8 编码；返回响应文本，不含协议结束符。
///
/// # 错误与副作用
/// 连接断开、超时或响应校验失败时返回错误；发送后不自动重试。
#[miresidian::operation]
fn query(&mut self, request: String) -> MiresidianResult<String> { /* 实际实现 */ }
```

注释的详细内容要求见[开发规范](../../docs/coding-standards.md#6-注释与文档)。工具读取 `src/` 下 Rust 文件和内联模块中的 inherent impl 方法，合并 `///`、`/** ... */` 与字面 `#[doc = "..."]` 的 Markdown，并附源码相对路径、方法签名和 operation/参数/输出等声明。方法实现和未标记的辅助函数不会写入。示例围栏保留，以 # 声明的注释标题降为方法标题的下级；文件排序稳定、内容无构建时间，重复生成不产生重复章节。

首次在已有说明末尾追加自动区域，之后只替换以下唯一标记对之间的内容；区域前后的手写部署、接线和 ABI 示例保持原样，不需要再手工复制业务正文：

```markdown
<!-- miresidian:business-docs:start -->
工具生成的方法参考
<!-- miresidian:business-docs:end -->
```

请在源码修改业务正文，不手工修改自动区域。方法删除或更名在下次成功构建后同步反映。标记缺失、重复或顺序错误会报错并保留原文；文档注释不能包含上述保留标记。源码解析失败或所选目标编译失败时不写入说明。成功写入通过同目录临时文件替换，写入失败保留原文并报告；写入前重新读取说明以保留构建期间的手工编辑。文档失败时整个命令返回失败，即使 Cargo 已成功产生 DLL，也不能把本次交付标为完成。

生成的是**源码方法参考**，不替代 DLL 的 GetFunctions/GetInitConfig 查询。扫描不展开宏、条件编译（cfg/cfg_attr）、include!、属性别名或 Rustdoc 自动链接；能读取到的文件/内联模块/impl 条件与方法自身声明会保留，不推断实际启用状态，也不推算平台操作名。`#[doc = include_str!(...)]` 等动态表达式标记为未展开；旧方法没有字面文档时明确提示缺少注释，不编造用途或阻止旧节点编译。实际启用能力、框架追加参数、ABI/内存契约和完整部署示例仍由当前 DLL 查询及自动区域外说明共同确定。源码注释的业务准确性在编码时维护，构建阶段不需要 AI 参与。

## 源码职责与验证

main.rs 负责入口，cli.rs 解析命令；template.rs 生成模板；validate/ 检查结构和依赖；
discover.rs 与 source.rs 读取 Cargo 图与节点属性；package.rs 编排交付，build.rs 调用 Cargo；documentation.rs 及 documentation/ 提取业务 Rustdoc、合并并写入受管 Markdown；
artifacts.rs 收集文件，manifest.rs 生成摘要及发布清单。各模块保持私有，不增加公开 Rust API。

```text
cargo test -p miresidian_pack --locked
cargo test -p miresidian_pack --test template --locked -- --ignored
```

第二条是显式 registry 集成测试，需要已发布框架及传递依赖可访问，以及 Python 3（可用 PYTHON 指定解释器）。
除保留 0.14.0 的结构/覆盖保护测试外，还覆盖 0.14.0/v7、0.26.1/v8、0.27.0/v9、0.33.0/v10，
以及不指定版本的 new/init：生成后检查实际调用说明，编译原生动态库，并先核对 ABI 再调用查询、初始化、Echo 和关闭。
构建缓存放在框架 target/pack-registry-tests，不使用本地框架依赖。
缓存预热后可设置 CARGO_NET_OFFLINE=true，此时仅跳过必须联网的默认 latest 用例；完整验收须联网执行。
既有回归继续验证检查不改锁、不覆盖文件，以及缺少 cdylib、范围依赖和缺失说明时失败。
基础单元测试无需访问 registry；宿主原生动态库验证不替代 Windows 双位数加载或硬件验证。

## 显式入口

    cargo miresidian-pack --new demo-node --miresidian-version 0.33.0
    cargo miresidian-pack --new demo-node --mires-version 0.33.0
    cargo miresidian-pack --check --node 演示设备
    cargo miresidian-pack --build --node 演示设备 --arch x64
    cargo miresidian-pack --pack --node 演示设备 --arch all
    cargo miresidian-pack --help

--node 缺省为全部节点，--arch 为 x64/x86/all，缺省 all。build 编译并更新业务 Markdown，pack 包含构建、文档更新并整理 dist。
构建先确认目标标准库可用，使用 --release --locked；链接器由项目配置，失败时明确提示核对；手动配置步骤见 [Windows GNU 构建环境](../../docs/coding-standards.md#windows-gnu-目标的手动配置)。
模板的 lib.rs 仅声明模块，node.rs 的注释说明配置、状态、资源清理及宏生成边界。
生成不代替编译、DLL 加载和硬件验证。--check 只检查项目；独立 TestFlow --check 可检查流程。
统一命令、整套下载安装和实际调用对照见 [CLI](../miresidian-cli/README.md)。

Pack 随包携带 framework-crates.txt，覆盖包括 batch、format 在内的全部框架库；声明、解析来源、覆盖配置和锁文件共用此清单。仓库版本门禁核对该清单与现行框架库集合一致；旧实例包名另行保留相同来源校验，旧节点无需因本次工具升级迁移。
