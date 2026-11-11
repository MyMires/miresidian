# miresidian_pack

0.13.4 的生成指导按项目实际依赖版本引用框架节点接口，不硬编码旧 trait 名称；不改变模板代码或可接受的 ABI 版本。

[文档中心](../../docs/README.md) · [架构总览](../../docs/architecture.md)

`miresidian_pack` 提供最小节点模板、项目结构检查和 Windows DLL 发布包整理。
本页对应工具源码版本 **0.13.4**，本次未发布；可在框架仓库执行
`cargo build -p miresidian_pack --locked`，再使用生成的 `target/debug/cargo-miresidian-pack`。
工具发布需单独授权，不随根框架发布。

## 命令

```text
cargo miresidian-pack --new demo-node --name 演示设备 --miresidian-version 0.25.2
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

**现有模板只支持 ABI v7。** 已发布的框架 0.26.0 为 ABI v8；0.13.2 和本次 0.13.4 源码均未提供 v8 模板。上述命令显式采用已核实发行的 0.25.2/v7。省略版本会查询最新正式版，不会自动筛选 ABI；若选到 v8，会写入模板及锁文件后报“模板创建尚未完成”。应在新的目标目录选择兼容版本重试；不要直接重跑 new 覆盖残留目录。v8 的既有项目升级须手动核对框架依赖、实际头文件和交付说明，再编译、检查及加载验收；不代表本工具已经提供 v8 模板支持。

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
可显式选择至少 0.14.0 的其他已发布版本；模板接口说明要求 ABI v7。
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

## 源码职责与验证

main.rs 负责入口，cli.rs 解析命令；template.rs 生成模板；validate/ 检查结构和依赖；
discover.rs 与 source.rs 读取 Cargo 图与节点属性；package.rs 编排交付，build.rs 调用 Cargo；
artifacts.rs 收集文件，manifest.rs 生成摘要及发布清单。各模块保持私有，不增加公开 Rust API。

```text
cargo test -p miresidian_pack --locked
cargo test -p miresidian_pack --test template --locked -- --ignored
```

第二条是显式 registry 集成测试，需要已发布框架及传递依赖可访问；缓存预热后可设置
CARGO_NET_OFFLINE=true。它验证生成项目的真实编译、检查不改锁文件、不覆盖既有目录，
并验证缺少 cdylib、范围依赖和缺失交付文档时失败。基础单元测试无需访问 registry。

## 0.12.0 的显式入口

    cargo miresidian-pack --new demo-node --miresidian-version 0.14.0
    cargo miresidian-pack --new demo-node --mires-version 0.14.0
    cargo miresidian-pack --check --node 演示设备
    cargo miresidian-pack --build --node 演示设备 --arch x64
    cargo miresidian-pack --pack --node 演示设备 --arch all
    cargo miresidian-pack --help

--node 缺省为全部节点，--arch 为 x64/x86/all，缺省 all。build 只编译，pack 包含构建并整理 dist。
构建先确认目标标准库可用，使用 --release --locked；链接器由项目配置，失败时明确提示核对；手动配置步骤见 [Windows GNU 构建环境](../../docs/coding-standards.md#windows-gnu-目标的手动配置)。
模板的 lib.rs 仅声明模块，node.rs 的注释说明配置、状态、资源清理及宏生成边界。
生成不代替编译、DLL 加载和硬件验证。--check 只检查项目；独立 TestFlow --check 可检查流程。
统一命令、整套下载安装和实际调用对照见 [CLI](../miresidian-cli/README.md)。

Pack 随包携带 framework-crates.txt，覆盖包括 batch、format 在内的全部框架库；声明、解析来源、覆盖配置和锁文件共用此清单。仓库版本门禁核对该清单与框架库集合一致。
