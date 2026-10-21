# DLL 测试工具

```text
tools/
├── test_serial_dll.bat             # 串口节点的可双击测试入口
└── common/
    └── invoke_miresidian_dll.ps1   # 所有节点共享的 C ABI 调用器
```

Windows 不需要安装 Python。双击节点对应的 `test_<node>_dll.bat` 即可；BAT 会调用公共 PowerShell 脚本、实际加载 DLL，并在结束后暂停显示结果。

## 编辑节点测试

BAT 顶部只有三个业务输入，其他 DLL 定位和显示设置都是节点脚本内部配置：

| 变量 | 类型 | 示例 |
| --- | --- | --- |
| `FUNCTION` | 字符串 | `GetFunctions`、`InitDevice` |
| `CONFIG` | JSON 字符串数组 | `[]`、`["COM3","9600"]` |
| `INPUT` | JSON 字符串数组 | `[]`、`["测试,数据","中文"]` |

`CONFIG` 和 `INPUT` 不是逗号分隔文本：它们必须是合法的 JSON 字符串数组。公共调用器先反序列化并确认每项都是字符串，再将原始 JSON 用 UTF-8 编码传给 DLL；数值、布尔值、对象或非法 JSON 会在调用前报错。

JSON 字符串本身含双引号时用 JSON 转义，例如 `["He said \"OK\""]`。若 BAT 中的内容包含百分号 `%`，请写成 `%%`，避免被 `cmd.exe` 当作环境变量展开。

## 新增节点入口

复制 `test_serial_dll.bat` 并重命名为 `test_<node>_dll.bat`。在新 BAT 的“Fixed node configuration”区域替换 `MIRESIDIAN_DLL_NAME`；如果节点不在默认发布目录，也在该区域调整 `MIRESIDIAN_DLL_PATH`。日常测试仅编辑顶部的三个业务输入，不修改 `common/`。

## 输出和位数

BAT 固定启用 `-Pretty`：DLL 的 JSON 响应会反序列化为状态、消息与编号输出清单。PowerShell 进程位数必须与 DLL 一致：普通 Windows PowerShell 通常为 x64；x86 DLL 应使用 `%WINDIR%\SysWOW64\WindowsPowerShell\v1.0\powershell.exe`。

`common/invoke_miresidian_dll.ps1` 是公共实现，负责 C ABI 参数/返回值的 UTF-8 处理与返回指针的即时复制。不要针对某个节点修改它。
