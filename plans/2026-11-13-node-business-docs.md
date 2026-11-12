# Node 业务文档注释与 DLL 构建自动生成 Markdown

日期：2026-11-13。状态：实现与行为验收完成；提交与标签以 Git 实际记录及交付回复为准（方案先于实现落盘）。
执行依据：用户要求未来 Node 业务代码尽量使用详细文档注释，DLL 构建时将业务方法文档并入 Markdown；明确选择同时更新 skill 与增强 Pack，并要求生成阶段无需 AI 额外处理。

## 目标与范围

业务方法的说明随源码维护，Pack build/pack 成功后自动更新项目 docs/<节点名称>.md，pack 将同一说明复制到 dist 根目录。生成不调用 AI、不要求 nightly/rustdoc JSON，不改变框架、宏、ABI 或节点依赖。用户仍需在源码中准确描述业务行为；工具不推测缺失说明。

## 现状与版本评估

框架 0.33.0；Pack 0.13.6 已用 syn 2 解析节点源码，但仅复制已有 Markdown；CLI/套件 0.3.7 将 Pack 固定为 0.13.6。Rust 官方说明文档注释等价于 doc 属性，内容为 Markdown（https://doc.rust-lang.org/reference/comments.html）；本次使用稳定 syn 解析字面文档，不依赖不稳定 rustdoc JSON。

Pack 新增构建输出能力，升 MINOR 至 0.14.0；CLI 只更新后端要求，升 PATCH 至 0.3.8。同步两组件 README、日期 CHANGELOG、套件清单和 Cargo 生成的工作区锁文件。框架、其余组件、C ABI v10、租约协议和独立 Node 版本不变。skill 独立维护，不设固定工具适配版本。

## 设计与实现

- Pack 新增 documentation.rs（若职责体量需要，按解析/合并/测试拆分）：按文件路径排序扫描项目 src 中 Rust 文件与内联模块，提取 inherent impl 上 operation（裸名或 miresidian 命名空间）方法的字面 doc 属性及签名。只生成标记的方法，不导出未标记辅助函数，不推算平台操作名；保留 operation、Config/Input/Output 等源码声明，源码位置及原有 cfg 条件供读者核对。
- 这是明确标记的源码方法参考；静态解析不展开宏、cfg/cfg_attr、include!、属性别名或动态 doc 表达式。条件编译声明随参考显示，不能声称每项都在当前 DLL 启用；实际操作和参数布局仍以该 DLL 查询为准。动态文档表达式和缺失注释明确标记，无需人工为生成器改写业务文档。
- 生成内容仅包含业务 API 注释和声明，不拼接方法实现；Markdown 标题适当降级，代码围栏原样保留。排序稳定，不写入构建时间，确保重复构建不造成文档噪声。
- 使用唯一 start/end HTML 注释标记自动区域。首次无标记时追加；已有合法区域时替换，前后手写文本完整保留。重复、缺失、逆序标记报错，不猜测覆盖范围。生成区域随源码删除/更名同步清理。
- 每个节点在构建前只读准备并校验生成内容，所选目标均编译成功后再更新项目 Markdown；更新时重新读取当前手写内容，避免长时间构建覆盖其间的文档编辑。使用同目录临时文件和 rename 写入，失败保留原文并清理临时文件。pack 更新后复制文档进入现有 dist 结构。--check 保持只读，裸 cargo build 不宣称自动处理文档。
- 模板为初始化、Echo、关闭方法提供中文 Rustdoc 示例，并说明源码是业务方法说明来源。节点文档规范集中在 docs/coding-standards.md，工具用法集中在 Pack README；skill 主文档保留简短要求与官方资料查询，review 只留检查项，完整同步已安装 5 文件。

## 影响文件与步骤

1. 本方案落盘并完成版本评估。
2. tools/miresidian-pack/{Cargo.toml,src/main.rs,src/documentation*,src/build.rs,src/package.rs,templates/*} 实现提取、受管区域合并和构建接入，新增针对性测试。
3. Pack README/CHANGELOG、CLI Cargo.toml/toolset.toml/README/CHANGELOG 与 Cargo.lock 同步实际版本及边界；根 CHANGELOG 记录组件能力与 skill 要求。
4. docs/coding-standards.md、skills/miresidian-node/{SKILL.md,references/review.md} 增加文档注释与自动交付要求，同步完整安装副本；不扩散重复规范。
5. 完成下列验证，记录证据后提交，创建并核对实际升版组件的不可移动标签；不推送、不发布。

## 验收矩阵

| 需求 | 验证与通过标准 | 状态 |
| --- | --- | --- |
| 自动提取业务文档 | 测试多文件/内联模块、私有 operation 方法、中文/Markdown、字面 doc、签名/标注、条件声明、非业务方法排除和缺注释提示 | 已通过，证据见下节 |
| 安全、稳定合并 | 首次追加、重复生成幂等、方法修改/删除、手写前后缀保留、非法标记不覆盖、写入失败保留原文 | 内容测试通过；文件替换经实现审查，未注入磁盘故障 |
| build/pack 真实入口 | 新增 registry 集成测试（仅 crates.io 已发布精确框架），实际 x64 构建及 pack，核对项目说明与 dist 文档；--check 不写入。环境不足则如实记录 | 已通过，证据见下节 |
| 工具与依赖 | Pack/CLI 相关测试、fmt、严格 Clippy/Rustdoc，Rust 1.89 可用时检查受影响组件；不运行无关全工作区 | 已通过，证据见下节 |
| skill 与提交门禁 | 格式、引用/锚点、UI 与完整副本一致；base/staged/提交后组件标签检查 | 内容与工作区门禁通过；暂存与标签在提交阶段核对，以 Git 实际记录及交付回复为准 |

## 实际结果与修订

实现与方案一致：documentation.rs 负责提取编排、受管区域和同目录替换，documentation/source.rs 负责 syn 静态解析，documentation/tests.rs 承担内容与合并保护回归；build/package 在全部所选目标成功后更新项目说明，pack 复制同一文档。模板、组件 README/CHANGELOG、Cargo 清单/锁文件、套件清单、开发规范与完整 skill 均已同步。

验收证据：

- cargo test -p miresidian_pack -p miresidian_cli --locked --offline：Pack 30 项单元、CLI 双入口 6 项单元及 4 项进程测试通过。首次 CLI 进程测试遇到 Text file busy，未修改测试/实现，重跑全部相关工具测试通过；记录为一次环境瞬态，不以首次失败作为通过。
- 显式运行新增 registry 回归“业务注释经_build_自动合并且_pack_交付同一文档”：1 项通过（17.25 秒）。临时节点精确依赖 crates.io 已发布框架 0.33.0，真实 GNU x64 DLL 构建及 pack 通过；--check 不改文档、注释变更同步、手写部署内容保留、交付副本相同、重复构建幂等、类型检查失败不写入新文档均有断言。
- cargo clippy -p miresidian_pack -p miresidian_cli --all-targets --locked --offline -- -D warnings 通过。
- RUSTDOCFLAGS=-D warnings cargo doc -p miresidian_pack -p miresidian_cli --no-deps --locked --offline 通过。
- cargo +1.89.0 check -p miresidian_pack -p miresidian_cli --locked --offline --target-dir target/msrv-business-docs 通过；cargo fmt --all --check 通过。
- skill 完整 5 文件逐字节一致，双副本 quick_validate、42 处内部链接/锚点、UI 元数据及 base 版本门禁通过。文件写入故障的原文保护由临时文件后 rename 的实现保证；未注入磁盘耗尽等系统故障，不将此部分表述为故障实测。

此次不交付真实设备 Node，不声称 Windows 原生加载或硬件验证；Linux 交叉编译仅证明相应构建与生成链。已有 4 项无关 registry/最新版集成仍按默认忽略，本次仅显式执行受影响的文档构建回归。不推送、不发布。

组件标签核对目标为 miresidian_pack-v0.14.0 与 miresidian_cli-v0.3.8，根标签 miresidian-v0.33.0 保持原指向。2026-11-13 查询两个工具候选精确版本的 crates.io API 均为 HTTP 404，不将本次候选写成可直接安装的已发布版本。
