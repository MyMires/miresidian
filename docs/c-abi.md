# C ABI

Miresidian 节点 DLL 只导出一个业务入口 miresidian_invoke。入口使用 UTF-8 JSON 文本，
不要求调用方分配输出缓冲区，不传递 Rust 结构体。

ABI 版本 miresidian_abi_version() 当前返回 5。

## 入口声明

~~~c
uint32_t miresidian_abi_version(void);

const char *miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json);
~~~

- function：UTF-8 C 字符串，例如 GetFunctions、InitDevice 或节点操作名。
- config_json、input_json：JSON 字符串数组，例如 ["COM3","9600"]；传 NULL 等同于 []。数组元素不能是数值、布尔值或对象。
- 返回值：DLL 内部保存的 UTF-8 JSON C 字符串指针。

DLL 在当前线程中持有返回文本。调用方无需分配输出缓冲区、无需传容量、也不得释放返回指针；
应在同一线程下一次调用前将文本复制到自己的内存。

例如，调用 GetFunctions：

~~~c
const char *response = miresidian_invoke(
    "GetFunctions", "[]", "[]");
~~~

## 响应体

返回文本始终是同一种 JSON 对象，字段顺序固定为 status、message、output。

成功时：

~~~json
{
  "status": 0,
  "message": "",
  "output": ["业务输出1", "业务输出2"]
}
~~~

失败时：

~~~json
{
  "status": 2,
  "message": "设备尚未初始化",
  "output": []
}
~~~

status 为状态码，message 为成功时的空字符串或失败诊断，output 始终为业务字符串数组。
JSON 转义、中文与换行由标准 JSON 编解码器处理，调用方不得自行按逗号拆分文本。

GetFunctions、GetInitConfig 和 GetPortNO 是常驻查询；它们不要求先调用 InitDevice。
只有节点定义的设备操作才会在未初始化时返回“设备尚未初始化”。

## 状态码

| 值 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 参数无效 |
| 2 | 操作失败 |
| 3 | 内部错误 |
