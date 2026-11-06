# 推送与发布跨 DLL 节点租约

日期：2026-11-06。初始方案在修改和发行前落盘；当前状态：版本修订、源码推送、正式发布和精确版本文档核对完成。

## 目标、现状与授权

用户明确授权“推送和发布”，范围承接跨 DLL 节点租约实现和后续先方案后实施规范更新。推送 origin/master 及本次版本标签，发布本次实际变更且尚未在 crates.io 发布的组件。不得强推、移动标签、发布无关工具/crate，或改造和交付未提供源码的恒惠 DLL。

当前 HEAD 为 4671135，origin/master 为 9380a5b；框架 0.25.0、lease 0.17.0 的不可移动标签均指向 d538b80。crates.io 已核实框架最新 0.24.1、lease 最新 0.16.0；其他精确内部依赖沿用已发布框架依赖，Cargo 正式发布时还将从 registry 实际验证。

## 发布前缺口与具体修复

lease 的 SharedResource/ResourceHandler/local_ipc 公开能力受 local-ipc feature 控制，但 Cargo.toml 没有 package.metadata.docs.rs 配置。docs.rs 官方说明 all-features 默认 false（https://docs.rs/about/metadata），当前根包已有 all-features=true，lease 包缺少配置。仅上传 lease 0.17.0 会遗漏共享 API 的默认在线文档。

先修复此发布文档缺口：lease 新增 docs.rs all-features=true 及真实 documentation 地址，补丁升至 0.17.1；根框架因精确依赖同步补丁升至 0.25.1。保留旧版本和旧标签，不修改既有归档条目，不补发未发布中间候选。宏仍为 0.22.0，所有 Rust 运行时代码、宏标签、ABI v7、配置租约 v2、资源代理 v2、MSRV 1.89 和其他组件版本不变。

当前 README、共享资源技术说明、版本规范和新发布说明同步这两个补丁版本，并将依赖示例写为须先核实 registry 可用；不在发布前声称已上传。Node skill 保持独立，纯规范变更已同步的副本不随组件升版修改。

补充核查：lease 英文 CHANGELOG 当前缺少 0.17.0 条目且没有 Unreleased 标题；R3 在新增 0.17.1 时一并补齐该未发布候选的真实修订摘要，不改写 0.16.0 及更早已归档记录。

## 修改与发布步骤

| 步骤 | 文件/目标 | 实现及验收 |
| --- | --- | --- |
| R1 | 本文件 | 在配置和发行操作前落盘需求、修复依据、版本影响、顺序与验收；重大偏离先修订方案 |
| R2 | features/lease/Cargo.toml、根 Cargo.toml、Cargo.lock | lease 0.17.1 开启 docs.rs 全特性文档，根 0.25.1 同步精确依赖；由 Cargo 生成真实锁记录，不改源码和其他版本 |
| R3 | 根与 lease 的双语 README/CHANGELOG、docs/shared-resources.md、docs/versioning.md、docs/releases/0.25.1.md | 修复指导内容、归档带日期补丁修订和精确资料入口，保留旧版归档；root/lease README 与版本同提交 |
| R4 | 文档与版本检查 | 复用 d538b80 的默认/全特性测试、Clippy、MSRV、Windows 编译与双动态库证据；本次只改文档配置/依赖版本，严格生成 root/lease 全特性 Rustdoc，核实 SharedResource、ResourceHandler 和 close_with/transact_with_timeout 页面；运行版本及差异门禁 |
| R5 | 本地提交/标签 | 提交上述补丁及计划，钩子生成 miresidian-v0.25.1，按组件生成 miresidian_lease-v0.17.1，require-tag 核对；旧 0.25.0/0.17.0 标签保持原提交 |
| R6 | GitHub origin | 原子推送 master 和本次 4 个版本标签（框架 0.25.0/0.25.1、lease 0.17.0/0.17.1），不推送无关标签、不强推；远端分支与标签 SHA 逐项核实 |
| R7 | crates.io | 先 cargo publish -p miresidian_lease --locked --all-features，确认 registry 可解析 0.17.1，再 cargo publish -p miresidian --locked --all-features；保留 Cargo 正式包验证，不绕过 verify、不用 dirty、不重复等价 dry-run，不泄露凭据 |
| R8 | registry、发布包与 docs.rs | 核对两个精确版本未撤回、包内 README/文档配置/.cargo_vcs_info 源码修订和 GitHub 源码入口；访问精确版本根 API、共享 API 及两个新增方法；若 docs.rs 尚在排队或构建失败，如实记录并提供源码入口，不把上传成功等同文档成功 |

## 发布契约和停止条件

租约包不依赖设备连接类型，本次不改运行时实现。自动 Lease 仍按同契约同完整初始化参数共享整个节点对象及状态；Direct 保持本地独占；独立业务对象共用连接使用通信层 SharedResource。升级前停止资源代理 v1 参与者，旧 DLL 只有升级到真实发布依赖并重编后获得新行为。

只有这两个最终补丁版本需上传。推送与发布凭据使用既有配置，不读取或输出凭据内容；遇到权限失败保留已完成结果并报告具体阻塞。远端有新提交时先核对差异，不强推；相同版本已发布则核对来源和内容后跳过，不覆盖。

任务完成需具备：本地门禁与版本标签通过、远端提交/标签对应、两个 registry 精确版本可用且有实际包验证证据、公开文档实际状态明确。现场 COM5、恒惠 DLL、Node 依赖升级和工具发行不在本次范围。

## 实施进度与证据

- R1：计划在修改文档配置前落盘，发布授权来自用户本次“推送和发布”。
- R2/R3：框架 0.25.1、lease 0.17.1 的清单、精确依赖、Cargo 生成锁文件、双语 README/CHANGELOG 和版本/共享资源说明已同步；补齐原本缺失的英文 0.17.0 摘要。旧条目和旧标签不改写。
- R4：严格全特性 Rustdoc 通过并实际检查根 API、local_ipc、SharedResource、ResourceHandler、close_with 和 transact_with_timeout 页面；格式、版本门禁、差异和本地文档目标校验通过。src、macro、features/lease/src 相对 4671135 无变更，复用已有默认 42 项/全特性 157 项、Clippy、MSRV、Windows 编译与双动态库证据。
- R5 至 R8：按后续提交记录、远端引用及实际发行结果逐项核对；未执行部分不得记作成功。发布最终结果在本计划补充，不在发布前声称 registry 或 docs.rs 可用。

## 最终发行核对

核对时间：2026-11-06 13:45:17 Asia/Shanghai

- R5：发行提交为 bc918e7632bbe2c37f7607b9e294850abcf4191e；根与 lease 的 require-tag 检查通过，新标签 miresidian-v0.25.1、miresidian_lease-v0.17.1 指向该提交。旧 miresidian-v0.25.0、miresidian_lease-v0.17.0 仍指向 d538b808e2ffb87abbe5d50a6461fd6a19528980。
- R6：已原子推送 master 和上述四个标签至 origin，远端分支与各标签解引用 SHA 已核对。实现提交 d538b80、规范提交 4671135 和发行提交 bc918e7 均已进入远端；最终核对记录以本计划的后续纯记录提交一并推送，不修改版本标签。
- R7：cargo publish -p miresidian_lease --locked --all-features 正式包验证及上传成功，registry 可用后再执行 cargo publish -p miresidian --locked --all-features；根包从 registry 下载 lease 0.17.1 后编译验证成功。仅上传这两个最终版本，没有重复发布其他组件或中间候选，没有绕过 verify 或使用 dirty。
- R8：从 crates.io 下载两个实际发行包，校验官方 checksum，并读取包内 README、规范化 Cargo.toml、.cargo_vcs_info.json；两包源码修订均为 bc918e7632bbe2c37f7607b9e294850abcf4191e，git.dirty 不为 true。根精确依赖 registry lease 0.17.1，lease 精确依赖 registry codec 0.16.0，均无本地 path；文档 all-features 元数据与根包技术手册已核实。GitHub 对应修订的清单公开可访问。

| 正式包 | crates.io 精确版本 | SHA-256 |
| --- | --- | --- |
| miresidian | [0.25.1](https://crates.io/crates/miresidian/0.25.1) | `5ec26be0835a3dbb1c6b471aa08cfdf2fb033bb613b566c0c7ac1cc0d9d399ee` |
| miresidian_lease | [0.17.1](https://crates.io/crates/miresidian_lease/0.17.1) | `ea671010afbec67e40336e9c7b053dad3c4b86aa4a77b67f8d5c725cc9624f02` |

在线文档已实际返回 HTTP 200 并核对 API 内容：

- [框架 0.25.1 API](https://docs.rs/miresidian/0.25.1/miresidian/) 与 [可选 format 文档](https://docs.rs/miresidian/0.25.1/miresidian/format/index.html)。根包刚上传时曾返回 404，待 docs.rs 生成后重新核对通过，未把初次检查记作成功。
- [SharedResource 0.17.1](https://docs.rs/miresidian_lease/0.17.1/miresidian_lease/shared/struct.SharedResource.html)，页面含 close_with 和 transact_with_timeout 方法。
- [ResourceHandler 0.17.1](https://docs.rs/miresidian_lease/0.17.1/miresidian_lease/shared/trait.ResourceHandler.html)，页面含 transact 和 close；[LocalLease 0.17.1](https://docs.rs/miresidian_lease/0.17.1/miresidian_lease/local_ipc/struct.LocalLease.html) 含 acquire 和 validate。两份精确 crate 文档入口可访问。

运行时及宏源码与 d538b80 无差异，Node skill 双副本仍为完整 5 文件一致。本次没有升级独立 Node、替换旧 DLL、执行现场 COM5 通信或宣称 Windows 原生加载已通过。此前运行时验证与本次包编译、资料核对分别记录；未执行无关全量测试或工具发行。

本记录的版本影响：无；仅归档已经完成的推送、发布与验证结果，不改变已发布包内容、版本或不可移动标签。
