# C ABI v1

Miresidian 的原生 DLL 以 `extern "C"` 导出，调用约定为 C 默认调用约定。当前 DLL 内置“串口通信”节点，固定操作与节点操作均通过 `miresidian_invoke_v1` 调用。

对应 C 头文件为 [`api/miresidian.h`](../api/miresidian.h)。C#、LabVIEW 或其他语言的包装层必须严格遵守本文件的编码、所有权和线程约定。

## 导出函数

| 函数 | 用途 |
| --- | --- |
| `miresidian_abi_version` | 返回 ABI 版本，目前为 `1`。 |
| `miresidian_invoke_v1` | 调用固定操作或节点操作。 |
| `miresidian_free_output_v1` | 释放一次成功调用返回的输出。 |
| `miresidian_last_error_v1` | 获取当前线程最近一次 ABI 错误。 |

## 调用模型

```text
operation + config[] + input[] -> output[]
```

- `operation`、`config` 和 `input` 均使用以 NUL 结尾的 UTF-8 文本。
- `config_count` 或 `input_count` 为 `0` 时，可将对应数组指针传为 `NULL`。
- 数量非零时，对应数组及每一项字符串指针均不得为 `NULL`。
- 两个数组是相互独立的参数通道。具体参数从哪个通道取值由节点的宏标注决定：未标注的
  `InitDevice` 参数默认取 `config`，其他方法参数默认取 `input`；`#[miresidian::config]` 和
  `#[miresidian::input]` 可以逐参数覆盖默认来源。因此不得仅依据操作名推断另一个数组必定为空。
- 调用方必须把 `MiresidianOutputV1` 零初始化。
- 返回 `MIRESIDIAN_STATUS_OK` 后，调用方读取 `output.values[0..output.count]`，并且必须调用 `miresidian_free_output_v1`。
- 失败时不需要释放输出；读取 `miresidian_last_error_v1` 获取诊断文本。该文本由 DLL 管理，在当前线程的下一次 ABI 调用后可能失效，应立即复制。

## 状态码

| 值 | 常量 | 含义 |
| ---: | --- | --- |
| 0 | `MIRESIDIAN_STATUS_OK` | 调用成功。 |
| 1 | `MIRESIDIAN_STATUS_INVALID_ARGUMENT` | ABI 指针、UTF-8 文本或参数数组无效。 |
| 2 | `MIRESIDIAN_STATUS_OPERATION_FAILED` | 节点未初始化、操作不存在、串口访问或设备通信失败。 |
| 3 | `MIRESIDIAN_STATUS_INTERNAL_ERROR` | DLL 已捕获内部 panic；请读取错误文本并反馈。 |

## C 调用示例

```c
#include "miresidian.h"
#include <stdio.h>

int main(void) {
    MiresidianOutputV1 output = {0};
    int32_t status = miresidian_invoke_v1(
        "GetFunctions", NULL, 0, NULL, 0, &output);

    if (status != MIRESIDIAN_STATUS_OK) {
        fprintf(stderr, "%s\n", miresidian_last_error_v1());
        return 1;
    }

    for (size_t index = 0; index < output.count; ++index) {
        puts(output.values[index]);
    }
    miresidian_free_output_v1(&output);
    return 0;
}
```

## Windows x64 DLL 构建

在 Linux 开发容器中生成 Windows x64 DLL，需要安装 GNU 交叉链接器并添加 Rust 目标：

```bash
sudo apt-get install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo build --release --target x86_64-pc-windows-gnu
```

产物为：

```text
target/x86_64-pc-windows-gnu/release/miresidian.dll
```

构建完成后，应使用 `dumpbin /exports` 或 `llvm-nm` 验证上述四个导出符号。
