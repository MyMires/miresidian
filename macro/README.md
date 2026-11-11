# miresidian_macro

本源码 0.27.0 与根框架 0.31.0 配套；生成 Node 实现、NodeEnum 业务枚举 codec，以及 ABI v10 的唯一 JSON 调用入口。成员显示值支持长短标注，候选格式和输入表示归调用平台适配器，宏不固定 TB 语法。候选尚未发布。

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

本源码候选未发布。节点从根框架 0.31.0 使用宏；0.24.0 引入的 feature 简写继续有效，不把候选写成可直接安装依赖。

```rust,ignore
#[miresidian::node("设备", instances, lease)]
#[miresidian::node("设备", instances(multiple, handle, selectable(sequential)), lease(selectable(required)))]
#[miresidian::input("数据", batch, format(hex))]
#[miresidian::output("结果", format(utf8, hex, base64, selector = "结果格式"))]
```

裸 instances/instances() 在新语法中默认 single + 固定 sequential，省略标记仍禁用多通道；multiple 必须明确 config/handle，single 不接受关联项。裸 lease 默认 required；disabled 固定直连，selectable(required) 才生成访问方式选择器。只支持既有默认租约，不新增默认 Direct。租约资源键仍在初始化 Config 上就地标记 lease(key)，不推导 instance 角色。

format 直接列出六种既有编码标识符；单格式固定，多格式首项默认且 selector 必填。batch 保持独立裸标记，异步和逐条格式列保持兼容。旧 NameValue 长写严格保留其原必填检查；格式长写允许原有具名项次序。短写禁止混用长写字段、重复策略/格式/feature、无默认 selectable 和多格式缺 selector，不新增 feature 等号别名。Cargo feature 门禁、槽位顺序与参数描述不变。完整对照见[0.28.0 迁移说明](../docs/releases/0.28.0.md)。

## 类型驱动的 batch 与 format 组合

普通业务 Input 的 batch 不再按 Rust 类型名称限制为数值；生成代码使用原参数类型的 NodeInput、NodeCollectionElement 与 Clone。整数保留单值/列表/范围/混合，字符串、布尔、字符和别名使用其类型解析。format 的单条编解码独立于 batch；组合宏对齐数据列和格式列，支持格式广播/逐条选择及输出格式列。固定单编码不生成选择输入，原有参数顺序与 metadata 布局保持。

宏只组合各功能 API；batch 与 format 两个实现 crate 不互相依赖。输入解析/选择/长度校验先于业务，保持顺序、首错停止、逐槽输出聚合和同步/async 桥接。

0.20.0 起，业务 Config/Input 描述仅原样发布，不生成运行时默认值；缺参报错，数值显式空白为零，具体选项显示值仍映射。format 选择器等框架生成的显式默认值继续生效。

0.21.0 按初始化 Config 的原 Rust 类型生成归一化回调，根运行时和 lease 参数共用；类型别名通过 trait 分派，不按类型名称猜测。

0.22.0 支持初始化 Config 的 instance(key/ignore)、lease(key/ignore) 嵌套角色；生成独立的实例键索引及租约字段选择，不新增独立参数属性。节点策略仍由 node 子配置声明。参见 [组合身份与迁移](../docs/config-identity.md)。

## 完整请求入口（0.23.0 起）

`node` 仅生成 `miresidian_invoke(const char *request_json)` 和独立 ABI 版本查询。请求包括 function/config/input/platform；platform 非零显式选择，0/省略使用 DLL 默认平台，不添加平台 Config。旧分参数调用及旧完整请求名称已移除；先查询 ABI 10 再按新签名调用，见 [C ABI](../docs/c-abi.md)。

既有 0.23.0/v8 发布记录见[历史验收](../plans/2026-11-06-unified-abi-request.md#发布执行结果)；本检出为未发布 0.27.0/v10 配套源码，不能从历史记录推断本候选已可安装。

Pack 0.13.2 及本次 0.13.4 源码的 `new/init` 模板仍只接受 ABI v7；创建时显式选择已发布的 v7 框架，见[命令](../tools/miresidian-pack/README.md#最小模板)。既有节点手动升级到已发布 v8 框架时，须按实际头文件更新交付说明。TestFlow 保留原同步入口调用方式，不因此自动采用完整请求入口。

## English

Unpublished macro source 0.27.0 supports NodeEnum member display labels with either `#[node_enum(display = "label")]` or `#[node_enum("label")]`, integrated through framework 0.31.0. Omitted labels default to member names. Each member accepts one nonblank string; duplicate, mixed, unknown-key, wrong-type or enum-level annotations fail to compile. Direct codecs and outputs still use member names. Platforms render choices and normalize display inputs as a pair, applying their own syntax and ambiguity rules. Generated DLLs retain ABI v10's single-JSON invocation and separate version query.

## NodeEnum 派生（0.27.0 成员显示值）

```rust,ignore
#[derive(miresidian::NodeEnum)]
enum ReceiveMode {
    #[node_enum(display = "空闲超时")]
    IdleTimeout,
    #[node_enum("结束符")]
    Terminator,
    Length,
}
```

一次生成输入、输出、纯规范化、集合实现和结构化候选；参数仍用原 input/config 标注，不重复成员。无非空描述时平台自动生成候选，已有默认/候选文本原样保留；具体格式和输入转换由同一个调用平台适配器负责。仅支持非空、无字段、无泛型枚举，成员 cfg 与原定义一致，实际值使用成员名而不是数字 discriminant。详见[节点用法](../docs/宏驱动开发.md#枚举自动转换与候选)。

两种成员标注等价，可带单个末尾逗号；未标注的 Length 显示值仍为 Length。每个成员仅允许一个非空白字符串，重复标注、长短混用、未知键、非字符串、多余参数和枚举级标注编译报错。TB 自动候选为 `接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length`，输入显示值经平台转换，直接 codec 和输出仍使用实际成员名。TB 分隔符/首尾空白/歧义检查留在适配器，不限制其他平台的显示值格式。已有项目无需迁移；本次新标注需采用包含此能力的已发布框架后才能在独立 Node 中使用。
