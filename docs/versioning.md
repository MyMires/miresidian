# 版本管理

Miresidian 将框架、实例项目与 C ABI 分开编号，并以发布清单将三者固定到实际 DLL。

## 版本层次

| 层次 | 示例 | 规则 |
| --- | --- | --- |
| 框架版本 | `miresidian 0.2.0` | 管理运行时与过程宏，使用语义化版本。 |
| 实例项目版本 | `miresidian_serial 0.2.0` | 每个节点独立使用语义化版本。 |
| ABI 版本 | `5` | 仅在 C ABI 不兼容时递增。 |

## 唯一版本来源

- 框架版本定义在根 `Cargo.toml` 的 `[workspace.package]`。
- 实例版本定义在各实例项目的 `Cargo.toml` 的 `[package]`。
- 实例的 `package.metadata.miresidian` 必须声明其构建所用的框架版本与 ABI 版本。

```toml
# 根 Cargo.toml
[workspace.package]
version = "0.2.0"

# constellation/serial/Cargo.toml
[package]
version = "0.2.0"

[package.metadata.miresidian]
miresidian_version = "0.2.0"
miresidian_abi_version = 5
```

## 更新与发布

1. 将未发布变更先写入对应的 `CHANGELOG.md`。
2. 判断变更是 `MAJOR`、`MINOR` 还是 `PATCH`，并更新框架或实例的版本号。
3. 变更框架版本时，同步更新每个待发布实例的 `miresidian_version`；重新构建的实例必须提升自身版本。
4. 变更 C ABI 时递增 ABI 版本，并同步更新实例元数据、头文件、ABI 文档和兼容性测试。
5. 运行全部测试；进入对应节点项目后运行 `cargo miresidian-pack` 打包；打包器会拒绝框架或 ABI 版本不匹配的实例。
6. 将 `Unreleased` 条目移动到带日期的版本条目。
7. 提交后创建不可移动的 Git 标签：框架用 `miresidian-vX.Y.Z`，实例用 `miresidian_serial-vX.Y.Z`。

`MAJOR` 表示不兼容变更，`MINOR` 表示向后兼容的新能力，`PATCH` 表示向后兼容的修复。

## ABI 更新规则

- 新增 `miresidian_invoke` 可识别的常驻查询或节点操作，不改变 ABI 版本。
- 改变 C 导出函数、参数顺序、编码、返回文本生命周期或内存所有权时，必须递增 ABI 版本。
- ABI 版本与框架、实例语义版本互相独立，不能用其中一个代替另一个。

## 修订记录

- 根 `CHANGELOG.md` 记录框架、宏和 ABI 的修订。
- 每个实例项目维护独立 `CHANGELOG.md`，记录配置兼容性、迁移方法和测试范围。
- 已发布条目不可改写；说明错误时，以新版本或勘误条目补充。

## 打包校验

运行以下命令发布实例：

```text
cd constellation/serial
cargo miresidian-pack
```

打包器会构建 DLL、复制头文件、说明与实例的 `CHANGELOG.md`，并生成 `dist/<节点>/manifest.json`。
清单固定记录实例版本、框架版本、ABI 版本、Git 修订、目标平台及每个 DLL 的 SHA-256。
测试平台应在加载 DLL 后用版本常驻查询核对清单中的框架与实例版本。
