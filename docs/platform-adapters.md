# 平台适配器

Miresidian 的运行时只负责节点生命周期、通道定位和结构化共享调度。具体测试平台的函数名、请求载荷、共享调用语法、错误码和响应编码由平台适配器负责。

## 命名

平台稳定标识使用小写 snake_case，例如当前兼容平台为 `tb_lv_test_platform`；对应 Rust 类型使用 PascalCase：`TbLvTestPlatformAdapter`。后续平台遵循同一规则。

## 接口

平台实现 `miresidian::PlatformAdapter`，将统一 C ABI 已解析出的 `Invocation` 映射为 `RuntimeInvocation`，再把 `RuntimeResponse` 编码为 `InvocationOutput`。适配器不应在业务节点或 `NodeRuntime` 中引入平台特有的文本协议。完全不同的原生传输或请求类型，应由宿主先转换为 `Invocation`，或直接构造 `RuntimeInvocation` 调用 `NodeRuntime::invoke_command`。

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

`platforms(...)` 接受一个或多个实现 `PlatformAdapter + Default` 的 Rust 类型路径；框架始终保留 `TbLvTestPlatformAdapter`，并在 `miresidian_invoke` 首次调用时用节点生成的注册表创建运行时。因此，外部平台先调用原有 C ABI 的 `GetInitConfig`，再在 `InitDevice` 的 `Config[]` 中传入注册的 id 即可选中适配器，无需新增 C 导出函数。

`RuntimeInvocation::Shared` 使用 `SharedInvocation { slots }`。每个槽位对应一个已初始化通道；不足的槽位由运行时补为跳过，超过通道数的槽位会被拒绝。`SharedResponse` 的槽位数始终等于已初始化通道数。

## 当前兼容平台

`TbLvTestPlatformAdapter` 保留既有约定：`Input[0] = "共享"` 触发共享调用，后续项以英文逗号拆分为单通道参数，结果以逗号拼接回平台字符串。该逗号约定仅存在于适配器中；未来 JSON 或其他平台适配器可以直接保留嵌套参数和多输出边界。
