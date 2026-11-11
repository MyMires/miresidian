# Config 组合身份与租约声明

[文档中心](README.md) · [实例管理](instances.md) · [配置租约](../features/lease/README.md)

## 边界与语法

Config 描述平台输入；实例键定位业务对象；租约资源键标识底层资源，租约配置判断该资源的配置是否兼容。这三种职责分别生成元数据，不互相推导。只启用 instance 或只启用 lease 均可。

本版本只支持初始化参数上的嵌套写法：

```rust
#[miresidian::config("串口名称:COM1", instance(key), lease(key))]
port_name: String,
```

不新增独立的 instance/lease 参数属性。每个参数至多有一个 config 属性，每种角色至多声明一次；支持的角色只有 key、ignore。角色只能用于 init_device 的 Config 参数，不能用于 Input、关闭、探测或业务操作参数。instance 角色只适用于 multiple/config；lease 角色要求节点声明 lease(...)。错误或重复声明在编译期拒绝。

## 实例键

multiple/config 按以下规则选择业务 Config，按参数声明顺序组成结构化组合键：

| 标记情况 | 参与实例分发的参数 |
| --- | --- |
| 未标记 | 全部业务 Config |
| 只有 instance(ignore) | 除 ignore 外全部业务 Config |
| 任一参数标记 instance(key) | 仅标记 key 的参数 |

最终键不能为空。归一化使用原 Rust 类型，数值的等价表示不会创建不同实例。框架附加的 Handle、租约访问方式、平台和执行方式不参与业务组合键。初始化、同步/异步业务、多通道与关闭使用同一键规则。

初始化时任一选中键的原始文本为空，沿用空身份跳过初始化的约定；判空先于类型解析，其他参数不能补齐它。非初始化请求的键参数必须存在并通过类型解析，不将缺参解释为通配符。未选中参数仍按原有调用布局传入，但不参与路由类型校验；业务使用对象保存的 Config。

同键且全部规范化业务 Config 一致时复用对象；同键但其他业务 Config 不同则报配置冲突。ignore 仅排除实例分发，不能让重新初始化静默修改已有对象。默认全量键意味着改变普通 Config（如超时）会创建不同实例；需要稳定地址身份时显式选 key。

single 仍按完整配置复用或替换唯一对象；multiple/handle 仍以框架附加的 Handle 定位对象，这两种模式不接受 instance 角色。

## 租约资源键与配置

节点整体策略仍在结构体上，参数角色与实例键独立。0.28.0 起增加 `lease`（固定 required）、`lease(required)`、`lease(disabled)` 和 `lease(selectable(required))` 等价短写；旧 `lease(activation = ...)` 保留。裸标记不发布访问方式选择器，只有 selectable 形式才发布且默认租约；不改变参数角色要求，也不支持默认 Direct。新语法仍须显式启用 Cargo lease feature，独立节点采用前须核实该版本已发布。

参数级 `lease(key/ignore)` 写法保持：

```rust
#[miresidian::node("串口节点", lease(activation = selectable))]
struct SerialNode;

impl SerialNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("串口名称:COM1", lease(key))] port_name: String,
        #[miresidian::config("波特率:")] baud_rate: u32,
        #[miresidian::config("读取超时/ms:", lease(ignore))] timeout_ms: u64,
    ) -> miresidian::MiresidianResult<Self> {
        // 声明示例，不访问硬件。
        let _ = (port_name, baud_rate, timeout_ms);
        Ok(Self)
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
}
```

- lease(key)：组成资源身份，至少一个；不能把波特率等兼容性字段并入物理资源身份以绕过冲突。
- 未标记：参与租约配置比较，不需重复列出参数名。
- lease(ignore)：同时排除资源键和租约配置比较，仍正常解析并传入业务。

多个资源键按声明顺序和原类型归一化，采用带版本前缀及长度边界的编码，不直接拼接 ip:port。单个资源键保留原规范文本，保持旧单字段资源身份；前缀 `miresidian:tuple:v1:` 保留给组合编码，单字段不能占用。配置字节继续编码 Rust 参数名、顺序和规范值，可为空。跨节点共享同一资源时，组合键值及顺序、配置字段名及顺序、类型归一化规则必须一致；显示标签不决定资源身份。

相同资源键、不同配置必须在构造前拒绝。0.25.0 起 Lease 自动共享整个节点对象并串行执行完整调用；独立业务状态共享连接使用 SharedResource 通信层。超时只有在确实属于调用方独立设置时才能排除，若影响共享连接全局状态则应参与配置比较。

## TCP 与串口多设备

TCP 可分别在 ip、port 上标记 instance(key)、lease(key)，超时标记 lease(ignore)。实例键与资源键恰好使用同一组值，但没有隐式绑定；同 IP 不同端口形成不同对象与资源。

0.24.x 的纯配置租约可用 port_name 的 instance(key)、lease(key) 和 device_address 的 instance(key)、lease(ignore) 分离两种身份。0.25.0 自动共享的是整个节点对象，因此不同站号初始化不能直接附着；需要保留不同站号/超时的业务对象时，在通信提供者中统一资源键和配置编码并使用 SharedResource，业务节点只声明实例键。

## 兼容与迁移

0.23.x 的 multiple/config 仅按 Config[0] 关联；0.24.0 未标记时改为全部 Config。需要保留旧行为时，在初始化第一个 Config 上添加 instance(key)，其他参数不标记。依赖旧空首项跳过约定的节点同样显式标记原主键。升级后应重新查询 GetInitConfig 并验证路由、复用和关闭。

原 node(..., lease(activation = ..., resource = "name", configuration = ["..."])) 保留兼容；resource/configuration 必须成对声明，不能与参数 lease 角色混用。新声明只在 node 中保留 activation，不再列字段名。

新增 Rust/宏 API 和实例默认行为按 pre-1.0 次版本升级：框架 0.24.0、宏 0.22.0、lease 0.16.0；instance 0.16.5 仅修正随包职责说明。C ABI v7、配置租约 v2、资源代理 v1、MSRV 1.89 不变。组合资源身份编码是客户端约定，不更改 IPC 消息；旧单字段及配置编码保留。已有会话应关闭后升级；独立节点只在所需版本发布后升级 crates.io 精确依赖。

## 验证范围

覆盖默认全量、显式组合键、ignore 排除、类型归一化、空身份、同键配置冲突、同步/异步和多通道查找及关闭；覆盖组合资源无歧义、同资源异配置拒绝、配置为空、旧声明兼容及非法角色的编译期诊断。DLL 模拟和真实设备通信分别报告。

## 设计参考

组合主键与声明位置参考 [SeaORM 的多字段 primary_key](https://www.sea-ql.org/SeaORM/docs/1.1.x/generate-entity/entity-structure/#composite-key) 和 [Diesel 的 primary_key(a, b)](https://docs.rs/diesel/latest/diesel/prelude/derive.Identifiable.html)。局部排除的属性风格参考 [Serde field attributes](https://serde.rs/field-attrs.html)。本框架将字段标记适配到现有初始化参数契约；默认全量实例键、空身份跳过及同资源配置冲突规则属于 Miresidian 明确约定，不宣称为这些库的统一行为。

## 0.25.0 自动资源租约补充

参数角色与配置编码保持本页定义；自动共享节点、独立通信提供者和初始化兼容条件由[共享资源](shared-resources.md#现有-lease-标签自动共享节点资源)统一维护。
