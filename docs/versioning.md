# 版本管理（P0 强制提交门禁）

## 优先级与完成标准

本规范优先于普通实现、整理和提交便利性要求；用户明确指令具有最高优先级。版本管理是每次相关更新的完成条件，必须在提交前完成，不能因“尚未发布”“先提交代码”或只写 Unreleased 而省略。

自动门禁允许不升版的范围限于测试、AGENTS 和版本/编码等纯规范文件；README、平台调用说明和生成的 BAT 按交付内容检查。

相关更新包括运行时、属性宏、编解码、ABI/返回格式、租约、默认值/错误语义、依赖、MSRV、构建/CI、开发发布工具及交付调用说明。纯测试、纯规范说明或拼写修改可以不升产品版本，但必须在提交说明中写明“版本影响：无”及理由；一旦改变可构建、可调用或可交付行为，必须升级。

每次相关提交的必需结果：版本提升、依赖/锁文件一致、带日期的修订归档、当前示例同步、验证通过、提交后创建并核对版本标签。最终汇报必须包含版本号、提交、标签和测试结果。Git 提交与本地标签完成后，是否推送和发布 crates.io 分别依用户授权执行。

Unreleased 只用于开发中暂记；提交前将本次条目归档到实际版本，不能用它代替版本号升级。已有提交和标签不可改写；遗漏通过新的版本提交修正。
Miresidian 将框架、实例项目与 C ABI 分开编号，并以发布清单将三者固定到实际 DLL。

## 版本层次

| 层次 | 示例 | 规则 |
| --- | --- | --- |
| 框架版本 | `miresidian 0.11.0` | 管理运行时与过程宏，使用语义化版本。 |
| 实例项目版本 | `miresidian_serial 0.7.0` | 每个节点独立使用语义化版本。 |
| 租约协议版本 | `2` | 独立维护 IPC 协议；协议不兼容变更须同步常量、服务端点、资源锁目录、迁移说明与回归，不能代替框架或 ABI 升级。 |
| ABI 版本 | `7` | 新增 C 导出入口或改变既有 C 导出契约时递增。 |
| crate 发布版本 | `miresidian_abi 0.11.0` | `miresidian`、`miresidian_abi`、`miresidian_codec`、`miresidian_macro` 与 `miresidian_lease` 五个框架 crate 采用同一发布列车；子 crate 的不兼容变更仍须独立按语义化版本判断。 |

工作区中的两个 Cargo 开发工具也继承 0.11.0 发布列车；独立烟测夹具为 0.4.0，随框架重建时同步提升自身版本。

## 唯一版本来源

- 框架版本定义在根 `Cargo.toml` 的 `[workspace.package]`。
- 实例版本定义在各实例项目的 `Cargo.toml` 的 `[package]`。
- 节点 `Cargo.toml` 的依赖图是其框架与 ABI 版本的唯一来源；`miresidian_pack` 从实际解析到的 `miresidian` 与 `miresidian_abi` 自动写入发布清单。

```toml
# 根 Cargo.toml
[workspace.package]
version = "0.11.0"

# ../miresidian_serial/Cargo.toml
[package]
version = "0.7.0"

[dependencies]
miresidian = { version = "=0.11.0", path = "../miresidian_rust", features = ["lease"] }

```

## 更新与发布

1. 修改前确定相关性和版本影响；运行时/工具改变必须升级对应版本。开发中可先写 Unreleased。
2. 判断变更是 `MAJOR`、`MINOR` 还是 `PATCH`，并更新框架或实例的版本号。
3. 变更框架版本时，更新待发布实例的 `miresidian` 依赖版本约束；重新构建的实例必须提升自身版本。
4. 变更 C ABI 时递增 ABI 版本，并同步更新 ABI crate 元数据、头文件、ABI 文档和兼容性测试。
5. 先执行版本门禁，再运行规定的格式、Clippy、默认/全特性测试、文档和构建检查；交付 DLL 前在目标平台执行打包与冒烟验证，记录实测范围。
6. 提交前将本次 Unreleased 条目归档到 `## [X.Y.Z] - YYYY-MM-DD`，同步当前版本示例并检查暂存区。
7. 提交后创建不可移动的 Git 标签：框架用 `miresidian-vX.Y.Z`，实例用 `miresidian_serial-vX.Y.Z`。

### crates.io 首发顺序

根 crate 依赖四个公开子 crate，因此首次发布必须按以下顺序进行：

1. 对 `miresidian_abi`、`miresidian_codec`、`miresidian_macro`、`miresidian_lease` 分别执行 `cargo package --allow-dirty` 与测试。
2. 按依赖顺序执行四个子 crate 的 `cargo publish`：先发布 `miresidian_abi`、`miresidian_codec`、`miresidian_macro`，再发布依赖 `miresidian_codec` 的 `miresidian_lease`；等待 crates.io 索引可解析对应版本。
3. 在干净工作区对根 crate 执行 `cargo package`；确认它从 crates.io 解析四个子 crate 后，再执行 `cargo publish`。
4. `miresidian_pack` 与 `miresidian_testflow_windows` 是独立开发工具，均已具备发布元数据与包级 README；配置真实的公开仓库链接后，可在根 crate 之后分别执行 `cargo publish`。

发布前必须为每个公开 crate 配置真实的 `repository`；如有文档站或主页，再补充 `documentation`、`homepage`。这些 URL 不得以猜测值代替。

`MAJOR` 表示不兼容变更，`MINOR` 表示向后兼容的新能力，`PATCH` 表示向后兼容的修复。

## ABI 更新规则

- 在既有 `miresidian_invoke` 中新增可识别操作不改变 ABI 版本；新增 C 导出入口或改变既有导出契约时递增 ABI 版本。
- 改变 C 导出函数、参数顺序、编码、返回文本生命周期或内存所有权时，必须递增 ABI 版本。
- JSON 响应增加可忽略的新字段，且保留既有字段类型、语义和内存约定时属于兼容扩展；本次新增 `code` 保持 ABI v7。调用方应允许未知字段，新工具消费旧 DLL 时允许 code 缺失。
- 新增公开 Rust 错误类型、变体或调整公开错误分类时，按 pre-1.0 的次版本发布；开发中可暂记 Unreleased，但相关代码提交前必须完成次版本升级与修订归档。
- ABI 版本与框架、实例语义版本互相独立，不能用其中一个代替另一个。

## 修订记录

- 根 `CHANGELOG.md` 记录框架、宏和 ABI 的修订。
- 每个实例项目维护独立 `CHANGELOG.md`，记录配置兼容性、迁移方法和测试范围。
- 已发布条目不可改写；说明错误时，以新版本或勘误条目补充。

## 打包校验

运行以下命令发布实例：

```text
cd ../miresidian_serial
cargo miresidian-pack
```

打包器会构建 DLL、复制头文件、说明与实例的 `CHANGELOG.md`，并生成 `dist/<节点>/manifest.json`。
清单固定记录实例版本、框架版本、ABI 版本、Git 修订、目标平台及每个 DLL 的 SHA-256。
测试平台应在加载 DLL 后用版本常驻查询核对清单中的框架与实例版本。

## 自动门禁与本地安装

Python 3.11+ 的 `tools/check_versions.py` 是框架和相邻节点共用的只读版本检查器。安装每个检出目录的 Git 钩子：

```bash
git config core.hooksPath .githooks
```

框架提交前执行：

```bash
python3 tools/check_versions.py --base HEAD
```

串口节点提交前在节点根目录执行：

```bash
python3 ../miresidian_rust/tools/check_versions.py --repo . --base HEAD
```

pre-commit 使用 `--staged` 读取暂存区，防止工作区已升版本但提交中遗漏；检查源码/构建/依赖变化相对基线是否升版、发布列车与精确依赖、三个锁文件、修订归档、ABI 常量/元数据/头文件和已有标签冲突。C 导出签名变化会要求 ABI 同步递增。语义兼容性仍须按本文人工判断，自动检查不能替代接口评审。

commit-msg 拒绝未升版且未说明“版本影响：无；具体理由”的提交。post-commit 只读取 HEAD，忽略尚未提交的版本修改，仅在该提交提升版本时创建附注标签，不移动已有标签；随后必须用 `--require-tag` 确认标签指向 HEAD。纯文档提交不会移动已有版本标签。首次检出必须安装钩子；不得绕过门禁。已有自定义钩子须先整合。CI 使用完整 Git 历史和 PR 基线/推送前提交，通过 `--check-commits` 逐一检查规范生效后的每个提交，避免仅比较最终版本而漏掉中间未升版的相关提交。规范生效前的历史保留。

## 未发布框架的可复现开发

框架与节点相邻检出时，节点同时声明 `version = "=X.Y.Z"` 与 `path`，以实际源码生成 Cargo.lock 并执行 `--locked` 检查。框架尚未发布时，不允许保留旧锁文件或伪造 registry 校验和。Cargo 生成发布包时移除 path 并保留精确版本；正式发布节点前必须先发布对应框架。安装和发布后的 crate 仍通过 crates.io 依赖，不包含具体通信依赖。
