# C ABI

Miresidian 节点 DLL 使用固定的 C ABI。平台只需调用 DLL 导出的入口，不需要了解 Rust
类型、结构体或数组长度字段。

ABI 版本 `miresidian_abi_version()` 当前返回 `3`。

## 数据约定

- `Function` 是一个以 NUL 结尾的 UTF-8 字符串，例如 `GetFunctions`、`InitDevice` 或节点操作名。
- `Config`、`Input` 是以空指针结尾的 UTF-8 字符串指针数组；`NULL` 等同于空数组。
- `Output`、`Error` 是 DLL 返回的同类数组。数组最后一项必为 `NULL`，空数组仅包含该结束指针。
- 调用方不得释放返回数组或其中字符串。DLL 会保留每个输出通道最近一次结果；调用方必须在下一次调用写入同一通道前读取并复制文本。
- 所有入口使用 `__cdecl`；状态码为 `int32_t`：`0` 成功、`1` 参数错误、`2` 节点操作失败、`3` 内部错误。

`api/miresidian.h` 是 C/C++ 调用方的唯一声明来源。

## 导出入口

```c
uint32_t miresidian_abi_version(void);

int32_t miresidian_invoke(
    const char *Function,
    const char *const *Config,
    const char *const *Input,
    char ***Output);

int32_t miresidian_invoke_with_error(
    const char *Function,
    const char *const *Config,
    const char *const *Input,
    char ***Output,
    char ***Error);

int32_t miresidian_invoke_without_error(
    const char *Function,
    const char *const *Config,
    const char *const *Input,
    char ***Output);
```

三者处理同一个 `Function + Config[] + Input[]` 请求，区别仅在结果通道：

| 入口 | 成功结果 | 失败结果 |
| --- | --- | --- |
| `miresidian_invoke` | `Output = ["0", "", ..业务输出]` | `Output = [错误码, 错误信息]` |
| `miresidian_invoke_with_error` | `Output = [..业务输出]`，`Error = []` | `Output = []`，`Error = [错误码, 错误信息]` |
| `miresidian_invoke_without_error` | `Output = [..业务输出]` | `Output = []`，失败原因只由返回状态码表示 |

例如查询操作列表：

```c
char **output = NULL;
int32_t status = miresidian_invoke(
    "GetFunctions", NULL, NULL, &output);

/* status == 0；output[0] == "0"，output[1] == ""，业务输出从 output[2] 开始。 */
/* 读取所需文本后复制；不调用 free，也不向 DLL 传回 output。 */
```

## 错误边界

`Function` 为 `NULL`、非 UTF-8 文本、数组未以 `NULL` 结束，或输出地址为空时，入口返回
`MIRESIDIAN_STATUS_INVALID_ARGUMENT`。节点自身的初始化、配置、通信或业务失败返回
`MIRESIDIAN_STATUS_OPERATION_FAILED`。Rust panic 不会跨越 ABI 边界，而会被转换为
`MIRESIDIAN_STATUS_INTERNAL_ERROR`。

合并入口即使失败也会尽量把错误码与错误信息写入 `Output`；分离入口将它们写入 `Error`。
