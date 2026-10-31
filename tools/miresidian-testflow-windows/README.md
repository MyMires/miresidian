# miresidian_testflow_windows

[文档中心](../../docs/README.md) · [架构总览](../../docs/architecture.md)

`miresidian_testflow_windows` 将节点项目的 `.testflow` 生成可直接在 Windows 上执行的自包含 BAT。

## 用法

在节点项目根目录运行：

```text
cargo miresidian-testflow-windows tools/serial_smoke.testflow
```

Cargo 必须能找到 `cargo-miresidian-testflow-windows` 二进制；在框架源码联调时，可由节点项目的 Cargo alias 指向此工具。

框架发布到 crates.io 后，安装一次即可在任意节点项目使用：

```text
cargo install miresidian_testflow_windows
```

## 输出与边界

- 仅接受 `.testflow` 文件，且当前节点包必须声明 `cdylib`。
- 工具从 `cdylib` 名称推导 DLL 文件名，在流程文件旁生成同名 `.bat`。
- BAT 以自身目录（`%~dp0`）定位流程文件和 `../dist`，并以内嵌的框架执行器解析流程、调用 DLL；因此不受 `cmd` 当前目录或 UNC 启动目录影响。
- BAT 运行时使用系统 PowerShell，但不会依赖或写入节点项目中的 `.ps1` 文件；生成命令本身不构建 DLL，也不访问测试硬件。
- BAT 默认在结束后暂停；自动化环境设置 `MIRESIDIAN_TESTFLOW_NO_PAUSE=1` 可保留退出码且跳过暂停。

因此流程文件、测试数据与硬件环境始终归节点项目维护；流程解析、DLL 调用和响应格式化由框架 TestFlow 工具统一维护。

DLL 返回 `status、code、message、output`；执行器显示稳定 `code`，错误诊断包含 status/code，业务结果仍读取 output。为兼容旧 DLL，code 缺失时沿用数值 status 判断，不解析 message 文本。

## 0.12.0 检查与帮助

    cargo miresidian-testflow-windows --check tools/smoke.testflow
    cargo miresidian-testflow-windows --help
    cargo miresidian-testflow-windows --version

--check 不加载 DLL、不生成 BAT，只检查语法和基本配置/初始化顺序；生成时自动执行相同检查。
Config(...) 显式选择配置，InitDevice() 后才调用业务；所有参数必须为 JSON 字符串。
Delay 接收 0..2147483647 的毫秒数字符串。方法是否存在、参数数量及默认值规范化仍由执行器加载 DLL 后验证。
不同 Config 的静态关联按原始字符串数组判断；涉及默认值等价替换时应保持相同 Config 写法。
统一入口为 cargo miresidian testflow --os windows <流程.testflow>，--os 由入口消费，原工具路径用法保持兼容。
整套下载与命令对照见 [CLI](../miresidian-cli/README.md)。
