# C ABI

Miresidian 节点 DLL 只导出一个业务入口 `miresidian_invoke`。入口使用 UTF-8 JSON 文本，
不传递 `char**`、`char***` 或 Rust 结构体。

ABI 版本 `miresidian_abi_version()` 当前返回 `4`。

## 导出入口

```c
uint32_t miresidian_abi_version(void);

int32_t miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json,
    char *output_json,
    uint32_t output_capacity);
```

- `function`：UTF-8 C 字符串，例如 `GetFunctions`、`InitDevice` 或节点操作名。
- `config_json`、`input_json`：JSON **字符串数组**，例如 `["COM3","9600"]`；传 `NULL` 等同于 `[]`。数组元素不能是数值、布尔值或对象。
- `output_json`：调用方预分配的 UTF-8 文本缓冲区。
- `output_capacity`：输出缓冲区字节数，包含结尾 NUL 字符。
- 返回值：唯一的错误码来源。

DLL 不分配、不保存、不释放调用方提供的任何内存。缓冲区不足时返回
`MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL`，且不会越界写入；此时 `output_json` 不包含可用响应，调用方应扩大缓冲区后重新调用。

例如，调用 `GetFunctions`：

```c
char output_json[65536] = {0};
int32_t status = miresidian_invoke(
    "GetFunctions", "[]", "[]", output_json, sizeof(output_json));
```

## 响应体

`output_json` 始终是同一种 JSON 对象，借鉴 HTTP 的“状态码在响应头、内容在响应体”规则。

成功时：

```json
{
  "output": ["业务输出1", "业务输出2"],
  "error": null
}
```

失败时：

```json
{
  "output": [],
  "error": "设备尚未初始化"
}
```

错误码不重复写入 JSON；调用方通过函数返回值判断错误类型，通过 `error` 读取诊断文本。
`output` 始终为字符串数组；`error` 只会是 `null` 或一个字符串。JSON 转义、中文与换行由标准 JSON 编解码器处理，调用方不得自行按逗号拆分文本。

## 状态码

| 值 | 名称 | 含义 |
| ---: | --- | --- |
| 0 | `MIRESIDIAN_STATUS_OK` | 调用成功。 |
| 1 | `MIRESIDIAN_STATUS_INVALID_ARGUMENT` | 空函数名、非 UTF-8 文本或 JSON 不是字符串数组。 |
| 2 | `MIRESIDIAN_STATUS_OPERATION_FAILED` | 节点生命周期、配置、通信或业务调用失败。 |
| 3 | `MIRESIDIAN_STATUS_INTERNAL_ERROR` | DLL 捕获到未预期内部错误。 |
| 4 | `MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL` | 调用方提供的输出缓冲区不足。 |
