# 版本管理
Miresidian 将框架、实例项目与 C ABI 分开编号，并以发布清单将三者固定到实际 DLL。

## 版本层次

| 层次 | 示例 | 规则 |
| --- | --- | --- |
| 框架版本 | `miresidian 0.9.0` | 管理运行时与过程宏，使用语义化版本。 |
| 实例项目版本 | `<node package> 0.9.0` | 每个节点独立使用语义化版本。 |
| ABI 版本 | `6` | 新增 C 导出入口或改变既有 C 导出契约时递增。 |
| crate 发布版本 | `miresidian_abi 0.9.0` | 当前四个公开 crate 采用同一发布列车；子 crate 的不兼容变更仍须独立按语义化版本判断。 |

## 唯一版本来源

- 框架版本定义在根 `Cargo.toml` 的 `[workspace.package]`。
- 实例版本定义在各实例项目的 `Cargo.toml` 的 `[package]`。
- 节点 `Cargo.toml` 的依赖图是其框架与 ABI 版本的唯一来源；`miresidian_pack` 从实际解析到的 `miresidian` 与 `miresidian_abi` 自动写入发布清单。

```toml
# 根 Cargo.toml
[workspace.package]
version = "0.9.0"

# ../miresidian_serial/Cargo.toml
[package]
version = "0.9.0"

[dependencies]
miresidian = { version = "0.9.0" }

```

## 更新与发布

1. 将未发布变更先写入对应的 `CHANGELOG.md`。
2. 判断变更是 `MAJOR`、`MINOR` 还是 `PATCH`，并更新框架或实例的版本号。
3. 变更框架版本时，更新待发布实例的 `miresidian` 依赖版本约束；重新构建的实例必须提升自身版本。
4. 变更 C ABI 时递增 ABI 版本，并同步更新 ABI crate 元数据、头文件、ABI 文档和兼容性测试。
5. 运行全部测试；进入对应节点项目后运行 `cargo miresidian-pack` 打包；打包器从已解析依赖自动记录框架与 ABI 版本。
6. 将 `Unreleased` 条目移动到带日期的版本条目。
7. 提交后创建不可移动的 Git 标签：框架用 `miresidian-vX.Y.Z`，实例用 `miresidian_serial-vX.Y.Z`。

### crates.io 首发顺序

根 crate 依赖三个公开子 crate，因此首次发布必须按以下顺序进行：

1. 对 `miresidian_abi`、`miresidian_codec`、`miresidian_macro` 分别执行 `cargo package --allow-dirty` 与测试。
2. 依次执行三个子 crate 的 `cargo publish`，并等待 crates.io 索引可解析对应版本。
3. 在干净工作区对根 crate 执行 `cargo package`；确认它从 crates.io 解析三个子 crate 后，再执行 `cargo publish`。
4. `miresidian_pack` 与 `miresidian_testflow_windows` 是独立开发工具，均已具备发布元数据与包级 README；配置真实的公开仓库链接后，可在根 crate 之后分别执行 `cargo publish`。

发布前必须为每个公开 crate 配置真实的 `repository`；如有文档站或主页，再补充 `documentation`、`homepage`。这些 URL 不得以猜测值代替。

`MAJOR` 表示不兼容变更，`MINOR` 表示向后兼容的新能力，`PATCH` 表示向后兼容的修复。

## ABI 更新规则

- 在既有 `miresidian_invoke` 中新增可识别操作不改变 ABI 版本；新增 C 导出入口或改变既有导出契约时递增 ABI 版本。
- 改变 C 导出函数、参数顺序、编码、返回文本生命周期或内存所有权时，必须递增 ABI 版本。
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
