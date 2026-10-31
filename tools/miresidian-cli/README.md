# Miresidian CLI

独立版本 **0.2.0**。统一入口只启动原工具进程，不合并原工具项目，也不依赖它们的 Rust 库。
两个后端各自为 **0.12.0**，继续独立安装、使用及发布。框架运行时版本与 CLI、套件版本无关。

## 完整指令与实际调用

所有命令也可将 miresidian 替换为 mires。参数按原始进程参数传递，支持含空格名称及路径。

| 统一指令 | 实际独立工具调用 |
| --- | --- |
| cargo miresidian new demo-node | cargo miresidian-pack --new demo-node |
| cargo miresidian new demo-node --name 演示设备 | cargo miresidian-pack --new demo-node --name 演示设备 |
| cargo miresidian new demo-node --miresidian-version 0.14.0 | cargo miresidian-pack --new demo-node --miresidian-version 0.14.0 |
| cargo miresidian new demo-node --mires-version 0.14.0 | cargo miresidian-pack --new demo-node --mires-version 0.14.0 |
| cargo miresidian check [--node 名称] | cargo miresidian-pack --check [--node 名称] |
| cargo miresidian build [--node 名称] [--arch x64/x86/all] | cargo miresidian-pack --build，原样传递选项 |
| cargo miresidian pack [--node 名称] [--arch x64/x86/all] | cargo miresidian-pack --pack，原样传递选项 |
| cargo miresidian testflow --os windows tools/smoke.testflow | cargo miresidian-testflow-windows tools/smoke.testflow |

统一 check 在原项目检查成功后，若当前目录有 tools/smoke.testflow，还会调用独立 TestFlow 工具的
--check 检查该文件。工作区选择节点不会改变当前目录，其他流程可直接使用原工具 --check。
--os 必填，仅用于选择后端，不传给已经限定 Windows 的工具。

build/pack 默认全部节点、all 架构、release。build 只保留 Cargo 构建产物；pack 包含构建，
并整理 DLL、头文件、说明、CHANGELOG 与 manifest 到 dist。new 默认查询最新未撤回正式版，
两个版本参数等价，不能重复；生成后框架依赖精确锁定，后续构建不自动升级。

无参数及 --help/-h 显示总帮助；--version/-V 显示 CLI 版本。
new/check/build/pack --help 转发独立工具的帮助；testflow --help 在入口显示系统选择语法。
同目录后端优先于 PATH。版本探测需要至少清单中的后端版本；缺失或过旧时提示安装，
不自动下载。后端标准流、工作目录、普通退出码透传；异常退出返回 1。

## Cargo 全局安装、升级与版本切换

三个 crate 保持独立；所有用户安装都由 Cargo 完成，不依赖 Python 或额外安装脚本。
首次安装入口，然后安装全部后端：

    cargo install miresidian_cli
    cargo miresidian install --full

也可以一条原生命令安装三个包：

    cargo install miresidian_cli miresidian_pack miresidian_testflow_windows

CLI 安装本身只安装 cargo-miresidian 和 cargo-mires；--full 是 CLI 子命令参数，
不能写成 cargo install miresidian_cli --full。new 直接使用已安装 Pack，不进行临时安装。

| 管理指令（也支持 cargo mires） | 实际 Cargo 调用 |
| --- | --- |
| install --full | cargo install miresidian_pack miresidian_testflow_windows |
| install --full --pack-version 0.12.0 --testflow-version 0.12.0 | cargo install miresidian_pack@0.12.0 miresidian_testflow_windows@0.12.0 |
| install pack --version 0.12.0 | cargo install miresidian_pack@0.12.0 |
| update --full | cargo install miresidian_pack miresidian_testflow_windows |
| update pack | cargo install miresidian_pack |
| use pack 0.10.0 | cargo install miresidian_pack@0.10.0 |
| use testflow 0.12.0 | cargo install miresidian_testflow_windows@0.12.0 |
| list | cargo install --list |

--full 管理清单登记的全部后端；各工具支持别名和完整 crate 名。未指定版本时由 Cargo
选择最新正式版；显式指定时支持升级、降级及预发布版本，版本须为完整语义版本号。
切换到缺少新命令的旧工具时会说明兼容性要求，不静默升级。

常用 Cargo 参数直接透传（例如 --locked、--offline、--root、--force），其他参数放在 -- 后。
批量指定版本用各工具自己的 --<别名>-version；单个工具用 --version。
批量失败可能已有部分工具完成安装，保持 Cargo 原生行为，可用 list 检查，不自动回滚或重试。

默认使用 Cargo 的用户级安装根目录，通常为 CARGO_HOME 或用户 .cargo，程序位于其 bin 下。
--root、CARGO_INSTALL_ROOT 和 Cargo install.root 配置遵循 Cargo 优先级；不建立项目内工具目录。
同一安装根目录每个工具使用一个安装版本，新版本替换旧版本，不默认保留并存版本。
项目源码、节点 Cargo.toml 和 Cargo.lock 不因管理命令而改变。

CLI 本体使用原生 Cargo 命令更新或切换，避免运行中的 CLI 替换自身：

    cargo install miresidian_cli
    cargo install miresidian_cli --version 0.2.0

公开发行前可在框架根目录从源码安装（同样由 Cargo 管理安装记录）：

    cargo install --path tools/miresidian-pack
    cargo install --path tools/miresidian-testflow-windows
    cargo install --path tools/miresidian-cli

## 开发者发行构建

Python 仅用于仓库内部构建和测试，不是用户安装前提。可选的预编译压缩包仍包含四个
独立程序和 manifest.json，不附加用户安装器：

    python3 tools/miresidian-cli/distribution/bundle.py --host linux-x64
    python3 tools/miresidian-cli/distribution/bundle.py --host windows-x64

仅在对应 x64 宿主构建；输出名含套件版本 0.2.0，附 SHA-256。
Cargo 安装是正式用户安装方式，归档文件用于分发和开发验证，不改 Cargo 安装记录。

## 后续工具接入

toolset.toml 同时登记独立工具版本、二进制和命令路由；新增工具时增加工具与路由条目，
并增加对应的参数转换和进程测试。普通路由可以原样转发参数；系统选择型路由使用 os。
发行脚本读取同一清单构建、收集新工具；登记别名后，--full 安装/升级与版本选项自动包含该后端。
不引入动态插件协议。

## 验证与发布

    cargo test -p miresidian_cli --locked
    python3 -m unittest discover -s tools/tests -v
    python3 tools/miresidian-cli/distribution/verify.py
    python3 tools/miresidian-cli/distribution/release.py

release.py 默认只执行 dry-run，显式 --publish 才上传。发布前检查干净工作区、组件标签及版本清单，已发布的同版本跳过；批次发布不改变各组件独立版本。

工具套件 workflow 在 Linux/Windows 通过原生 Cargo 安装并验证命令与记录，默认仅上传构建产物。
手动将 publish 设置为 true 时，先按清单发布未发布的后端、再发布 CLI，然后创建对应标签的草稿 Release，附加两个宿主包。
Cargo 发布后即可原生安装；草稿公开控制 GitHub 归档下载。标签不可移动，版本来自各 Cargo.toml 与套件清单。
