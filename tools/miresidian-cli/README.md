# Miresidian CLI

独立版本 **0.1.0**。统一入口只启动原工具进程，不合并原工具项目，也不依赖它们的 Rust 库。
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

## 整套下载与安装

发行套件为 miresidian-tools-0.1.0-linux-x64.tar.gz 或 miresidian-tools-0.1.0-windows-x64.zip。
每个压缩包包含四个独立程序与 manifest.json，旁边有同名 .sha256 文件。
Windows 程序带 .exe。工具宿主系统与节点 DLL 的 x64/x86 架构分别管理。

安装器需要 Python 3.11+（仅安装步骤），运行工具仍需 Rust/Cargo、对应 target 和链接器。
首次公开发行前应从检出目录安装或使用本地生成套件；本次更新不上传 GitHub、不发布 crates.io。
公开发行后，从该版本 GitHub Release 下载 install.py 即可一次安装全部工具：

    python3 install.py --version 0.1.0

Windows 使用 py -3 install.py；省略 --version 时，从公开正式套件中选择最新版本，
不会把框架发布误认为工具套件。可指定 --install-dir。默认安装到用户 .miresidian/bin，
与 Cargo 独立安装记录分开；只提示 PATH，不修改 shell 或系统配置。

Shell/PowerShell 包装器与 install.py 一起使用：

    sh distribution/install.sh --version 0.1.0
    powershell -File distribution/install.ps1 -Version 0.1.0

本地构建与安装（在框架根目录）：

    python3 tools/miresidian-cli/distribution/bundle.py --host linux-x64
    python3 tools/miresidian-cli/distribution/install.py --archive dist-tools/miresidian-tools-0.1.0-linux-x64.tar.gz --install-dir /tmp/miresidian-tools/bin

Windows 的 --host 为 windows-x64，生成 zip。bundle 检查清单/Cargo.toml/实际程序版本一致，
读取 Cargo 实际 target_directory，执行 --release --locked 构建；--bin-dir 可使用已构建程序。
安装前核对压缩包摘要、文件摘要、宿主和清单，拒绝路径穿越、链接及未登记文件；
全部验证通过后才写安装目录。原工具仍可以分别 cargo install。

## 后续工具接入

toolset.toml 同时登记独立工具版本、二进制和命令路由；新增工具时增加工具与路由条目，
并增加对应的参数转换和进程测试。普通路由可以原样转发参数；系统选择型路由使用 os。
发行脚本读取同一清单自动构建、收集新工具，安装器按清单校验额外的 cargo-* 程序。
不引入动态插件协议。

## 验证与发布

    cargo test -p miresidian_cli --locked
    python3 -m unittest discover -s tools/tests -v

工具套件 workflow 在 Linux/Windows 构建、安装并验证命令和摘要，默认仅上传 Actions 构建产物。
手动将 publish 设置为 true 时才创建 CLI 版本标签对应的草稿 Release，附加两个宿主包及安装器。
检查并公开该草稿后，在线安装器才可下载。标签不可移动；版本来自 CLI 和套件清单。
