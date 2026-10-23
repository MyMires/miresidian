# miresidian_testflow_windows

`miresidian_testflow_windows` 为节点项目的 `.testflow` 文件生成 Windows BAT 测试入口。

## 用法

在节点项目根目录运行：

```text
cargo miresidian-testflow-windows tools/serial_smoke.testflow
```

Cargo 必须能找到 `cargo-miresidian-testflow-windows` 二进制；在框架源码联调时，可由节点项目的 Cargo alias 指向此工具。

## 输出与边界

- 仅接受 `.testflow` 文件，且当前节点包必须声明 `cdylib`。
- 工具从 `cdylib` 名称推导 DLL 文件名，在流程文件旁生成同名 `.bat`。
- BAT 设置流程名、DLL 名和相对的 `../dist` 路径，再调用同目录的 `invoke_miresidian_dll.ps1`。
- 工具不解析 TestFlow 语法、不构建 DLL、不执行 PowerShell，也不访问测试硬件。

因此流程文件、PowerShell 调用器、测试数据与硬件环境始终归节点项目维护；框架只提供可复用的 Windows 入口生成能力。
