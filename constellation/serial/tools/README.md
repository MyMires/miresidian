# DLL 测试工具

```text
constellation/serial/tools/
├── invoke_miresidian_dll.ps1             # 串口节点的 DLL 调用器
├── test_serial_dll.bat                   # 双击测试入口
└── serial_smoke.flow                      # 与入口同目录的流程文件
```

Windows 不需要安装 Python。双击节点对应的 `test_<node>_dll.bat` 即可；BAT 会调用同目录的 PowerShell 脚本、实际加载 DLL，并在结束后暂停显示结果。

## 编辑流程测试

每个 DLL 项目目录都同时保存 BAT 入口和流程文件。BAT 顶部的 `FLOW_FILE` 指向可编辑的 UTF-8 流程文件。流程使用通用的 `方法名("字符串参数", ...)` 语法。除流程保留指令 `Config(...)` 与 `Delay("毫秒数")` 外，方法名必须由 DLL 的 `GetFunctions()` 返回：

```text
Config("COM3", "9600")
InitDevice()
Delay("500")
任意业务方法("输入一", "输入二")
CloseDevice()
```

调用器在同一个 PowerShell/DLL 进程中执行整个流程。`Config(...)` 是流程指令，不调用 DLL；它设置后续真实方法使用的 C ABI `config` 数组。`Delay("毫秒数")` 仅在流程步骤间等待指定毫秒数，不调用 DLL。真实 DLL 方法括号中的参数全部传入 C ABI `input` 数组，因此 `InitDevice()` 也可传入节点声明的 Input 参数。`Config(...)` 可以省略具有默认值的**尾部**配置参数，默认值取自 `GetInitConfig()`；枚举描述 `实际值-显示值` 会补入首个实际值。再次调用 `Config(...)` 可切换到另一个已初始化设备；`CloseDevice()` 关闭当前选择的设备。

流程参数必须是 JSON 字符串；共享执行方式可作为第九个 `Config` 参数传入 `Sequential` / `Parallel` / `Broadcast` 或对应中文显示值；双引号本身用 JSON 转义，例如 `任意业务方法("He said \"OK\"")`。空行、以 `#` 或 `//` 开头的行会被忽略。流程中任一步失败会停止，调用器会在退出时自动尝试关闭本流程成功初始化、但尚未关闭的每个 Config。业务方法的 Config/Input 参数来源由 DLL 自身校验。

对于需要直接定位 DLL 操作的场景，同目录脚本也支持单次调用：

```powershell
powershell -ExecutionPolicy Bypass -File .\invoke_miresidian_dll.ps1 .\miresidian_serial.dll GetFunctions '[]' '[]' -Pretty
```

## 新增节点入口

复制整个节点项目的 `tools/` 目录并改名为目标 DLL 项目名；BAT 与 `.flow` 始终放在该目录的同一级。随后在 BAT 中设置 `FLOW_FILE`，并在“Fixed node configuration”区域替换 `MIRESIDIAN_DLL_NAME`；如果节点不在默认发布目录，也调整 `MIRESIDIAN_DLL_PATH`。所有项目共用上一级 `common/` 调用器，不需要复制或修改。

## 输出和位数

BAT 固定启用 `-Pretty`：DLL 的 JSON 响应会反序列化为状态、消息与编号输出清单。PowerShell 进程位数必须与 DLL 一致：普通 Windows PowerShell 通常为 x64；x86 DLL 应使用 `%WINDIR%\SysWOW64\WindowsPowerShell\v1.0\powershell.exe`。

`invoke_miresidian_dll.ps1` 是该节点的调用实现，负责 C ABI 参数/返回值的 UTF-8 处理与返回指针的即时复制。新增节点时可按自身的测试与部署要求维护副本。
