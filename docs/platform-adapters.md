# 平台适配器

[文档中心](README.md) · [架构总览](architecture.md)

Miresidian 的运行时只负责节点生命周期、通道定位和结构化共享调度。具体测试平台的函数名、请求载荷、共享调用语法、错误码和响应编码由平台适配器负责。

## 命名

平台稳定标识使用小写 snake_case，例如当前兼容平台为 `tb_lv_test_platform`；对应 Rust 类型使用 PascalCase：`TbLvTestPlatformAdapter`。后续平台遵循同一规则。

## 接口

平台实现 `miresidian::PlatformAdapter`，并通过 `platform() -> Platform` 声明稳定平台枚举值；内置平台使用具名变体，第三方平台使用 `Platform::Custom(>= 0x0001_0000)`。适配器再将统一 C ABI 已解析出的 `Invocation` 映射为 `RuntimeInvocation`，并把 `RuntimeResponse` 编码为 `InvocationOutput`。适配器不应在业务节点或 `NodeRuntime` 中引入平台特有的文本协议。完全不同的原生传输或请求类型，应由宿主先转换为 `Invocation`，或直接构造 `RuntimeInvocation` 调用 `NodeRuntime::invoke_command`。

```text
Invocation
  -> PlatformAdapter::decode
  -> RuntimeInvocation
  -> NodeRuntime::invoke_command
  -> RuntimeResponse
  -> PlatformAdapter::encode
  -> InvocationOutput
```

## 平台选择

有两条互斥调用路径：

- `NodeRuntime::invoke(...)` 使用运行时注册表。`GetInitConfig` 与 `GetFunctions` 会发布保留 Config `平台适配:<id...>`；`InitDevice` 可选择其中一个已注册 id，选择不参与通道键，也不传给业务节点。运行时持有通道期间只能使用同一平台。
- `NodeRuntime::invoke_with(adapter, ...)` 由宿主显式指定适配器。此路径的初始化元数据不包含 `平台适配`，并会拒绝在 Config 中重复传入平台 id；只保留业务 Config、可选租约方式与共享执行方式。

默认构造的 `NodeRuntime::new()` 仅注册 `tb_lv_test_platform`。Rust 宿主可使用 `NodeRuntime::with_platforms(PlatformRegistry::new(...).with_adapter(...))` 注册更多适配器。

对 DLL 外部调用，适配器必须在节点声明时编译进 DLL：

```rust
#[miresidian::node("示例节点", platforms(MyPlatformAdapter))]
#[derive(Default)]
struct ExampleNode;

#[derive(Default)]
struct MyPlatformAdapter;
```

`platforms(...)` 接受一个或多个实现 `PlatformAdapter + Default` 的 Rust 类型路径；框架始终保留 `TbLvTestPlatformAdapter`，并在任一 C ABI 入口首次调用时用节点生成的注册表创建运行时。外部平台可继续用 `miresidian_invoke` 的 `InitDevice` Config 选择文本 id；需要显式选型时改用 `miresidian_invoke_with(uint32_t platform, function, config_json, input_json)`，它等价于 Rust 的 `invoke_with`，且初始化 Config 不包含平台选择器。

`RuntimeInvocation::Shared` 使用 `SharedInvocation { slots }`。第 N 个输入槽位对应第 N 个已初始化通道；超过通道数的槽位会被拒绝。`SharedResponse` 与请求槽位保持相同顺序和数量，未提供输入的后续通道不会产生输出槽位。

## 当前兼容平台

`TbLvTestPlatformAdapter` 保留既有约定：`Input[0] = "共享"` 触发共享调用，后续项以英文逗号拆分为单通道参数，结果以逗号拼接回平台字符串。该逗号约定仅存在于适配器中；未来 JSON 或其他平台适配器可以直接保留嵌套参数和多输出边界。
