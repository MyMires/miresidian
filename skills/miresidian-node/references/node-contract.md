# 节点设计与实现约束

以项目精确依赖的已发布框架为准。不要把框架工作区尚未发布的版本或能力直接写入独立节点。

## 项目边界

一个独立节点 crate 声明 cdylib，并从 crates.io 精确依赖框架，由 Cargo 维护真实 Cargo.lock。一个节点 crate 声明一个具体节点；静态检查不展开宏、别名或条件编译，不用多份条件节点声明切换同一节点。

lib.rs 只承担入口、模块声明和必要导出。具体节点状态与标注方法保留在同一模块。确有独立职责时拆分传输、协议等实现；已有成熟业务抽象优先复用，不为简单节点建立通用驱动框架。

节点负责连接、协议、业务状态和方法声明；运行时负责生命周期调度、跨语言转换和 ABI 支撑；属性宏生成平台契约与导出。不能为实现业务把设备依赖放入框架，或把框架生成代码复制到节点。

## 从业务划分 Config、Input 和 Output

Config 描述初始化与对象关联需要的业务配置，Input 描述当前方法调用的输入，Output 表达业务结果。逐项明确 Rust 类型、来源、平台名称、单位、默认值、范围和失败条件。

有业务初始化 Config 时，第一个业务 Config 必须是设备对象主键，例如设备地址或串口端口。超时、波特率等不应替代对象身份。无业务配置的最小单实例节点允许空 Config，不为了满足形式而伪造设备主键。

初始化默认从 Config 读取参数，其他操作默认从 Input 读取；单个参数可显式标注覆盖来源：

```rust
#[miresidian::operation]
fn init_device(
    #[miresidian::config("设备地址")] address: String,
    #[miresidian::config("超时/ms:1000")] timeout_ms: u64,
) -> MiresidianResult<Self>
```

这只是签名示意，方法体必须按实际设备实现。业务方法中的 Input 也应采用实际含义和单位；输出标注放在方法上，数量与业务输出一致。不要在节点手工拼接平台响应 JSON。

## 节点与方法声明

使用结构体上的 node 宏声明节点；生命周期及业务方法使用 operation。初始化是无 self 关联函数，返回 MiresidianResult<Self> 或 MiresidianResult<(Self, T)>，后者只有 T 是业务输出。关闭及普通业务方法使用 &mut self。

不添加旧的 node_impl，不手写 DeviceNode 实现、操作字符串分发、GetFunctions、GetInitConfig 或 C ABI 导出。固定方法映射如下：

| 节点侧 | 对外行为 |
| --- | --- |
| init_device，标注 operation | InitDevice，构造实际业务对象 |
| close_device，标注 operation | CloseDevice，关闭对象资源 |
| 普通标注方法 | 显式操作名称，或从 snake_case 自动转换为 PascalCase |
| 可选的无 self get_port_no，标注 operation | GetPortNO，临时探测并自行回收资源 |

GetFunctions、GetInitConfig、GetFrameworkVersion、GetProjectVersion 及 CloseAllDevices 由框架提供，无需手写占位。常驻查询无需初始化；业务调用按正确配置执行初始化后再调用。

选择平台可表达的类型，如 String、数值、bool、Option 和基础类型 Vec；确认所选版本对具体类型/输出组合的支持。使用框架已有 HEX/Base64、文本与数组编解码，不重新设计平台字符串协议。

## 通信、状态与错误

先用设备资料确定请求格式、结束条件、帧长度、校验、编码和超时语义，再实现协议。输入验证发生在相关副作用前；响应不能只取“看起来可用”的片段而忽略设备错误或残帧。

将连接和业务状态放在节点持有的对象中；避免可变全局状态。初始化部分失败时由函数内部所有权、Drop 或显式回滚释放资源；返回成功后由节点生命周期管理。

完整请求—响应必须避免交错访问。根据真实资源共享方式处理同步，不把实例锁当成跨设备或跨进程的事务保护。自动重试必须考虑操作是否幂等，以及超时前设备可能已执行的副作用。

预期错误通过 MiresidianResult 返回，并包含有用的操作/设备上下文；使用所选框架版本实际提供的错误 API，不猜测构造函数。不吞掉错误后返回零值、空文本或成功状态。不依赖 panic 处理正常超时、无效配置和设备拒绝。

关闭释放连接和后台资源，失败时仍尽力清理其余资源。不要在关闭后让遗留任务继续使用句柄。注释说明资源归属和清理责任。

## 按需求启用实例与共享能力

依赖 features、节点声明和运行时行为必须配套，具体选择与可复制的配置见 [框架能力与 features](features.md)。不能只描述概念而不落实 Cargo.toml 和节点代码。

先判断是否需要同时保留多个设备对象。默认是单实例；只开启 instances feature 不改变默认行为，需要同时启用依赖 feature 和对应 instances(...) 声明。

multiple/config 以 Config[0] 关联对象：空主键跳过初始化，相同主键且配置相同复用，配置不一致报错。handle 关联由框架在业务配置后追加 Handle，不能把它混入业务参数索引。

多实例、多通道执行、租约是不同能力，不因存在多个设备就全部开启。租约不代表物理句柄共享或通信事务互斥。需要这些能力时查所选发布版本的实例/租约说明，确认配置布局和并发语义再实现；不要创造框架替代协议。

## ABI 与调用说明

ABI v7 由框架生成 miresidian_abi_version、miresidian_invoke 和 miresidian_invoke_with。以本次依赖提供的 miresidian.h 为正式签名来源。

Config/Input 是 UTF-8 JSON 字符串数组，例如 ["COM3","9600"]。响应含 status、code、message、output；业务结果在 output 字符串数组中，以 status/稳定 code 判断失败，不解析 message。返回文本由 DLL 当前线程持有，调用方在同线程下一次调用前复制，不释放返回指针。

docs/<节点显示名>.md 应说明实际支持的 DLL/位数、ABI 签名、业务配置、方法输入/输出、单位/默认值、错误、返回所有权/有效期、调用顺序和示例。文档应能指导真实调用；文件非空不等于说明完整。

若需进一步核对，优先读取与目标版本相符的框架 docs/宏驱动开发.md、docs/instances.md、docs/c-abi.md。源码仓库为 [MyMires/miresidian](https://github.com/MyMires/miresidian)，[发布版 API](https://docs.rs/miresidian) 应切换到项目精确版本；不默认 HEAD/latest 与项目一致。
