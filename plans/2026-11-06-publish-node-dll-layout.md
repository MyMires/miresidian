# 发布 Node DLL 主入口目录规范

## 目标、授权与现状

- 用户在规范提交 `64e3501` 推送后明确要求“还要发布啊”，授权本次更新的正式发布及对应版本提交/标签推送。
- 该提交已明确每个节点对应架构目录只展示一个主 Node DLL，辅助运行时 DLL 放入 `dependencies/`；完整 Node skill 已同步安装副本。
- 当前根框架为 0.25.1，lease 为 0.17.1、Pack 为 0.13.2。仅将本次框架随包指导内容正式发行，不发布未修改的子 crate、工具或独立 Node，也不改打包器和加载代码。
- 调研开始时工作区另有未跟踪的 `plans/2026-11-06-ate-platform-operation-improvements.md`，属于其他工作，本任务未暂存或修改。实施期间该文件由另一任务独立提交为 `787446c`，成为发行提交的既有父提交；不改写或删除该历史。正式发布在本次标签的干净临时 worktree 中执行，避免混入任何未提交内容。

## 设计与版本影响

- 根框架补丁升至 0.25.2，用正式发布包携带本次交付目录指导；内部依赖、子组件、工具、MSRV、C ABI v7、配置租约 v2 和资源代理 v2 保持。
- 同步根双语 README 的当前版本、条件式精确依赖示例、版本文档、相关节点开发手册和带日期双语 CHANGELOG；新增 0.25.2 发布说明。历史修订和旧标签不改写。
- 由 Cargo 更新根锁文件中的工作区包版本，不升级依赖；其他锁文件只有实际引用改变时才更新。skill 不因框架补丁发布再次修改。
- 说明主入口与辅助 DLL 的交付层次，明确实际整理产物需验证依赖加载，不宣称当前 Pack 自动收集第三方 DLL；独立 Node 无需为纯指导内容升级。

## 步骤与影响文件

1. 本方案在版本及文档修改前落盘；确认 crates.io 与远端没有既有 0.25.2，不覆盖已发布版本。
2. 修改根 `Cargo.toml`、Cargo 生成的 `Cargo.lock`、`README.md`、`README.en.md`、双语 `CHANGELOG`、`docs/versioning.md`、`docs/宏驱动开发.md` 和 `docs/releases/0.25.2.md`。
3. 验证版本门禁、差异与文档链接；复用运行时代码未变的既有验证，按需生成严格全特性 Rustdoc。打包文件清单在提交后的干净检出中检查，避免以 dirty 方式打包。正式 `cargo publish -p miresidian --locked --all-features` 保留包编译验证，不使用 dirty 或 no-verify，不重复等价 dry-run。
4. 仅暂存本次文件并提交，钩子生成不可移动标签 `miresidian-v0.25.2`，运行 `--require-tag`；原子推送 master 与新标签，不强推、不推其他标签。
5. 在 `/tmp` 的干净 detached worktree 中正式发布根包，使用现有凭据但不读取或输出凭据；失败时核对 registry 实际状态后决定是否重试。
6. 分别核对 crates.io 精确版本、实际发行包 checksum、README/交付规范/核心手册、规范化依赖和 `.cargo_vcs_info.json`、远端提交/标签及 docs.rs 精确 API/可选 format 文档。docs.rs 排队时如实记录，不将上传成功记为文档成功。
7. 在本方案归档实际证据并推送纯记录提交，保留版本标签指向实际发行提交。

## 验收与停止条件

- 完成条件：本地门禁和版本标签通过，远端引用一致，registry 0.25.2 未撤回、正式包编译上传成功且内容/来源正确，docs.rs 实际状态明确。
- 同版本已存在时先核对来源和内容，不覆盖；远端出现其他提交时先核对，不强推。凭据/权限失败则保留结果并报告具体阻塞。
- 不改其他未跟踪方案，不升级 Node 依赖，不发布无关组件，不执行现场硬件或无关全量测试；不把本次发行说成 Windows DLL 依赖加载验证。

## 实施与验证记录

- 已更新根清单、Cargo 自动生成锁文件、双语 README/CHANGELOG、当前版本规范、节点开发手册及 0.25.2 发布说明。按 TOML 结构比较确认清单与锁文件只有根包版本变化，src/ABI/codec/features/macro/tools 相对 64e3501 无差异。
- 版本门禁、差异检查、6 文件的 Markdown 围栏及 69 处本地链接/锚点检查通过；严格全特性 Rustdoc 成功，实际检查根 API 与 format 页面存在且显示 0.25.2。
- 提交前运行 `cargo package --list` 被 Cargo 的未提交更改检查拒绝，未使用 allow-dirty；因此文件清单检查明确安排到干净发布检出中，不将此尝试记作通过。
- 远端 master 已核实为 64e3501；crates.io 已有未撤回 0.25.1，0.25.2 返回 404，远端没有 0.25.2 标签。后续提交/标签、正式包验证、上传及 docs.rs 分别记录，未执行部分不记为成功。
- 发行提交为 `cf1d5ef`，标签 `miresidian-v0.25.2` 的门禁已通过并与 master 原子推送。干净 worktree 为 `/tmp/miresidian-publish-0.25.2`；Cargo 文件清单包含双语 README、目录规范、节点手册及完整 skill。
- 清单检查发现另一任务已提交的 ATE 方案也按现有默认打包规则进入文件清单，原先“完全不包含该文件”的断言失败。核对其唯一提交 `787446c` 仅新增方案文档，没有源码/工具或依赖变更。发布前修订本方案：保留该既有提交及方案文件，不为排除一个已提交文档改变打包配置或重写历史；继续要求工作树干净、仅发布根框架以及没有未提交内容。此调整不改变产品行为、版本范围或发布权限。

## 最终发行核对

核对日期：2026-11-06（Asia/Shanghai）。本次发布及验收已完成。

- 发行提交：`cf1d5ef7a2c5dfef89b794581623d9293801c71d`。`miresidian-v0.25.2` 标签对象为 `f0123e297c7aa618f1c23fdae4e342aea95ec4e0`，解引用指向该发行提交；本地 `--require-tag` 及远端 master/标签 SHA 核对通过。原版本标签不移动。
- 已从干净 detached worktree 正式执行 `cargo publish -p miresidian --locked --all-features`：打包 194 文件，完成 registry 依赖下的正式包编译验证，上传成功且等待 registry 可用成功。没有使用 allow-dirty、no-verify 或重复等价 dry-run，只发布根包 0.25.2。
- [crates.io 0.25.2](https://crates.io/crates/miresidian/0.25.2) 精确记录存在且未撤回；下载实际发行包并核对官方 checksum。SHA-256 为 `427f0aa73d934c6d28eec33a26e1880c38aabac3edc1d9d270f84be912572cd7`。
- 发行包 `.cargo_vcs_info.json` 指向上述发行提交，非 dirty；包内双语 README、主入口/依赖目录规范、节点开发手册及完整 5 文件 skill 与本次源码一致。规范化 Cargo.toml 保留 docs.rs 全特性配置、registry lease 精确依赖 0.17.1，无本地 path/git 依赖。
- [框架 0.25.2 API](https://docs.rs/miresidian/0.25.2/miresidian/) 与 [format 可选能力页面](https://docs.rs/miresidian/0.25.2/miresidian/format/index.html) 实际返回 HTTP 200，并核对页面显示 0.25.2；[GitHub 精确标签交付规范](https://github.com/MyMires/miresidian/blob/miresidian-v0.25.2/docs/coding-standards.md) 的原始内容返回 HTTP 200，包含 `dependencies/` 要求。
- 实际验收对应目标：已确认的简单目录规则进入 registry 正式包，入口说明、手册与 skill 可从同版本来源查阅。未修改运行时、Pack 收集行为、DLL 加载代码或独立 Node；没有新增 Windows 原生加载或硬件验证。

本节是发行后纯记录更新，版本影响：无；不改变正式包内容、版本或不可移动标签。记录提交单独推送，发行标签继续指向 `cf1d5ef`。
