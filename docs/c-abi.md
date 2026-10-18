# C ABI

Miresidian 节点 DLL 通过 C ABI 导出三个操作入口。三个入口均接收同一组输入：

```text
function + config[] + input[]
```

`function` 是平台传入的操作名称；框架在内部将其作为 operation 调度到 `InitDevice`、
`CloseDevice` 或节点声明的方法。

## 导出函数

| 导出名 | 输出方式 | 适用场景 |
| --- | --- | --- |
| `miresidian_invoke` | 一个合并数组 | 平台只能接收一个输出数组。 |
| `miresidian_invoke_with_error` | `output` 与 `error` 两个数组 | 平台支持独立错误通道。 |
| `miresidian_invoke_without_error` | 仅 `output` 数组 | 平台只关心业务输出和 C ABI 返回状态。 |
| `miresidian_free_text_array` | 释放数组 | 释放任意入口返回的数组。 |

`miresidian_abi_version()` 当前返回 `2`。

## 输出规则

### `miresidian_invoke`

`output` 固定为：

```text
[错误码, 错误信息, ..业务输出]
```

例如读取成功：

```text
["0", "", "01 03 00 00"]
```

例如未初始化：

```text
["2", "节点尚未初始化"]
```

### `miresidian_invoke_with_error`

成功时：

```text
output = [..业务输出]
error  = []
```

失败时：

```text
output = []
error  = [错误码, 错误信息]
```

### `miresidian_invoke_without_error`

成功时 `output` 仅包含业务输出；失败时为空数组。错误详情不写入数组。

## 返回状态与内存

三个入口均返回 `int32_t` 状态码：`0` 成功、`1` 参数错误、`2` 节点操作失败、`3` 框架捕获
到内部 panic。返回的每个 `MiresidianTextArray` 都必须以同一 DLL 的
`miresidian_free_text_array` 释放；零初始化或已释放的数组可重复传入释放函数。

错误不再通过线程本地 `last_error` 传递。
