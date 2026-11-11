# miresidian_macro

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_macro` 是 Miresidian 的过程宏实现 crate，负责生成节点契约、调用分发、生命周期调度和 C ABI 入口。

节点项目应依赖并使用 `miresidian` 重导出的宏，而不需要直接依赖本 crate：

```rust
use miresidian::{config, input, node, operation, output};
```

宏标注和节点实现约束见[宏驱动开发](../docs/宏驱动开发.md)。

`node` 保留在结构体上，生成节点身份、注册绑定和 C ABI；每个 `operation` 独立生成方法契约，支持同模块多个固有 impl。初始化、关闭和可选探测同样使用 operation，节点不再需要 Default。

0.15.0 支持 operation 标注 async 方法，生成原生业务 Future 桥；构造、关闭和探测的 async 桥由生命周期线程等待。同步 DLL 入口共享内部并发运行时。参见[异步方法](../docs/宏驱动开发.md#异步方法)。


## 0.24.0 feature 宏简写

本源码候选未发布。节点从根框架 0.28.0 使用宏，不将未发布候选写入可直接安装示例。

```rust,ignore
#[miresidian::node("设备", instances, lease)]
#[miresidian::node("设备", instances(multiple, handle, selectable(sequential)), lease(selectable(required)))]
#[miresidian::input("数据", batch, format(hex))]
#[miresidian::output("结果", format(utf8, hex, base64, selector = "结果格式"))]
```

裸 instances/instances() 在新语法中默认 single + 固定 sequential，省略标记仍禁用多通道；multiple 必须明确 config/handle，single 不接受关联项。裸 lease 默认 required；disabled 固定直连，selectable(required) 才生成访问方式选择器。只支持既有默认租约，不新增默认 Direct。租约资源键仍在初始化 Config 上就地标记 lease(key)，不推导 instance 角色。

format 直接列出六种既有编码标识符；单格式固定，多格式首项默认且 selector 必填。batch 保持独立裸标记，异步和逐条格式列保持兼容。旧 NameValue 长写严格保留其原必填检查；格式长写允许原有具名项次序。短写禁止混用长写字段、重复策略/格式/feature、无默认 selectable 和多格式缺 selector，不新增 feature 等号别名。Cargo feature 门禁、槽位顺序与参数描述不变。完整对照见[0.28.0 迁移说明](../docs/releases/0.28.0.md)。

## 类型驱动的 batch 与 format 组合

普通业务 Input 的 batch 不再按 Rust 类型名称限制为数值；生成代码使用原参数类型的 DeviceInput、DeviceCollectionElement 与 Clone。整数保留单值/列表/范围/混合，字符串、布尔、字符和别名使用其类型解析。format 的单条编解码独立于 batch；组合宏对齐数据列和格式列，支持格式广播/逐条选择及输出格式列。固定单编码不生成选择输入，原有参数顺序与 metadata 布局保持。

宏只组合各功能 API；batch 与 format 两个实现 crate 不互相依赖。输入解析/选择/长度校验先于业务，保持顺序、首错停止、逐槽输出聚合和同步/async 桥接。

0.20.0 起，业务 Config/Input 描述仅原样发布，不生成运行时默认值；缺参报错，数值显式空白为零，具体选项显示值仍映射。format 选择器等框架生成的显式默认值继续生效。

0.21.0 按初始化 Config 的原 Rust 类型生成归一化回调，根运行时和 lease 参数共用；类型别名通过 trait 分派，不按类型名称猜测。

0.22.0 支持初始化 Config 的 instance(key/ignore)、lease(key/ignore) 嵌套角色；生成独立的实例键索引及租约字段选择，不新增独立参数属性。节点策略仍由 node 子配置声明。参见 [组合身份与迁移](../docs/config-identity.md)。

## 完整请求入口（0.23.0 起）

`node` 自动生成 C ABI 的 `miresidian_invoke_request(const char *request_json)`，同时保留原有 `miresidian_invoke`、`miresidian_invoke_with` 及 ABI 版本查询。完整请求含 function、config、input 和可选数值 platform；省略 platform 沿用初始化 Config 的平台选择，非零数值沿用显式平台规则。ABI 版本由所依赖框架的常量决定；框架 0.27.0 / ABI v9 起，数值 0 同样进入 Config 选择流程，宿主从 GetInitConfig/GetFunctions 呈现选项，再在 InitDevice 中传入用户选择。宏生成逻辑与标注无需改变。三个调用入口共用同一个运行时，节点作者无需增加标注或手写导出。请求解析和响应生命周期自框架 0.26.0 起由根运行时统一实现，完整字段与迁移见 [C ABI](../docs/c-abi.md#完整请求入口v8)。

0.23.0 已随框架 0.26.0 的后续授权阶段正式发布，见[发布验收](../plans/2026-11-06-unified-abi-request.md#发布执行结果)。本检出为尚未发布的 0.24.0，增加 feature 短写，并随根框架 0.28.0 提供；现有生成桥接、运行时和 ABI 契约保持。独立节点仍采用已发布框架的精确依赖，不用本地覆盖替代发行。

Pack 0.13.2 及本次 0.13.3 源码的 `new/init` 模板仍只接受 ABI v7；创建时显式选择已发布的 v7 框架，见[命令](../tools/miresidian-pack/README.md#最小模板)。既有节点手动升级到已发布 v8 框架时，须按实际头文件更新交付说明。TestFlow 保留原同步入口调用方式，不因此自动采用完整请求入口。

## English

Unpublished source 0.24.0 adds feature shorthand for framework 0.28.0: bare instances selects single/sequential multi-channel mode, bare lease means required, and format(hex) or format(utf8, hex, selector = "Format") preserves existing encoding behavior. All previous long forms remain supported; feature gates, defaults in old declarations, runtime ownership and ABI are unchanged. Generated exports obtain the ABI version from the framework. With framework 0.27.0 / ABI v9, platform zero follows Config selection: query the choices, present them in the host, and pass the chosen identifier in InitDevice Config. Nonzero values retain explicit selection. Only annotation parsing is extended; normalized policies and generated runtime bridges remain equivalent to the old forms.
