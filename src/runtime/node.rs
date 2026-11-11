//! 业务节点实现的扩展点。

use crate::{
    InvocationOutput, MiresidianResult, NodeDescriptor, PlatformRegistry, lease::LeaseRequest,
};

/// 由 `#[miresidian::node]` 和方法级 `#[miresidian::operation]` 为业务节点生成的运行时适配器。
///
/// 节点作者不应手写此 trait。宏会从 `operation`、`config`、`input` 与 `output`
/// 标注生成实现、强类型转换和描述元数据。
pub trait Node: Send + Sized + 'static {
    /// 返回此节点类型的只读静态元数据；宏生成实现会缓存已汇总的方法描述。
    fn descriptor() -> &'static NodeDescriptor;

    /// 返回编译进当前节点 DLL 的平台适配器。默认仅提供 TB LV Test Platform。
    /// `#[miresidian::node(..., platforms(...))]` 会覆盖此默认值。
    fn platform_registry() -> PlatformRegistry {
        PlatformRegistry::default()
    }

    /// 返回构建当前节点 DLL 的项目版本。
    ///
    /// 宏生成的节点实现会返回其 crate 的 `CARGO_PKG_VERSION`。手写实现若未覆盖
    /// 本方法，则显式表明其版本未声明。
    fn project_version() -> &'static str {
        "未声明"
    }

    /// 校验汇总后的平台契约，拒绝重复名称与框架保留操作名称。
    ///
    /// 运行时在处理请求前执行；方法排序不依赖链接器的注册顺序。
    fn validate_definition() -> MiresidianResult<()> {
        validate_descriptor(Self::descriptor())
    }

    /// 构造已初始化节点并返回原有平台业务输出。
    ///
    /// Lease 模式仅由唯一资源所有者执行一次；其他 DLL/Handle 引用复用该对象与输出，
    /// 完整规范化初始化 Config、Init Input 文本和命名接口必须一致。Direct 模式本地构造。
    ///
    /// 构造失败时没有可由运行时关闭的实例；初始化函数应通过资源所有权、Drop 或显式回滚
    /// 回收已取得的资源。返回实例后，运行时负责后续失败与正常关闭时的清理。
    ///
    /// `config` 与 `input` 保留外部调用的两个独立参数通道。通常初始化参数来自
    /// `config`；驱动方法将参数标注为 `#[miresidian::input]` 时，该参数改从
    /// `input` 读取。两个切片的下标分别按各自通道中的参数顺序计算。
    fn init_device(
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<(Self, InvocationOutput)>;

    /// 在单实例替换释放旧对象前校验初始化参数；宏生成强类型解析，不执行构造函数。
    fn validate_init(_config: &[String], _input: &[String]) -> MiresidianResult<()> {
        Ok(())
    }

    /// 已提取名称/选项后的单个业务 Config 值，供实例身份与租约统一编码。
    /// 宏按原参数类型生成纯归一化回调；手写契约默认保留文本。
    fn normalize_config_value(_index: usize, value: &str) -> MiresidianResult<String> {
        Ok(value.to_owned())
    }

    /// Config 关联多实例的键索引。None 表示全部初始化业务 Config。
    /// 宏根据 instance(key/ignore) 生成索引；组合顺序始终按 Config 声明，框架附加字段不参与。
    fn instance_config_indices() -> Option<&'static [usize]> {
        None
    }

    /// 返回由结构体级 `lease(...)` 声明生成的请求；在构造节点前执行。普通节点返回 `None`。
    fn lease_request(_config: &[String]) -> MiresidianResult<Option<LeaseRequest>> {
        Ok(None)
    }

    /// 释放节点持有的全部资源。
    ///
    /// 自动 Lease 模式只在最后一个引用退出时调用，使用最后显式关闭的 Input；
    /// 异常断连使用空 Input 尽力清理。非最后 CloseDevice 只释放代理引用。
    ///
    /// 框架在业务 panic 后仍会尽力调用本方法，再析构对象。实现应支持业务调用后
    /// 部分失败状态下的清理；清理错误不会阻止框架移除原通道并释放租约。
    ///
    /// 默认情况下关闭参数来自 `input`。节点作者可通过参数上的 `config` 或
    /// `input` 标注指定实际通道，因此运行时必须原样传入两组参数。
    fn close_device(&mut self, config: &[String], input: &[String]) -> MiresidianResult<()>;

    /// 在成功初始化后运行节点自定义操作。
    ///
    /// 未标注的操作参数默认来自 `input`。参数上的 `#[miresidian::config]` 可使
    /// 该单个参数从 `config` 读取，二者可在同一操作中混用。
    fn invoke(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput>;

    /// 判断指定平台操作是否具有宏生成的原生 Future 桥；同步操作或未知名称返回 false。
    fn is_async_operation(_operation: &str) -> bool {
        false
    }

    /// 执行原生异步业务，借用节点及两组参数直到返回输出或错误。
    ///
    /// 运行时在跨 await 期间保留目标实例的独占执行权；同步操作由阻塞池执行，
    /// 不因默认 Future 包装就变为可并行的异步业务。
    fn invoke_async<'a>(
        &'a mut self,
        operation: &'a str,
        config: &'a [String],
        input: &'a [String],
    ) -> NodeFuture<'a> {
        Box::pin(async move { self.invoke(operation, config, input) })
    }

    /// 无需构造节点即可探测端口；临时探测资源由关联函数自行回收。
    ///
    /// 未标注参数默认来自 `input`，也可以通过方法参数标注改从 `config` 读取。
    ///
    /// 节点宏方法是无 `self` 的关联函数，返回 `MiresidianResult<Option<String>>`。
    /// `Ok(Some(address))` 产生平台输出 `["1", address]`，`Ok(None)` 产生 `["0", "未找到匹配设备"]`；
    /// NotSupported 说明作为正常未找到的原因，空白说明补为“当前未实现端口发现能力”。
    /// 未声明方法默认返回 NotSupported，平台固定输出 `["0", "当前未实现端口发现能力"]`。
    /// 其他错误仍返回错误响应；临时连接和后台任务由探测实现释放。
    fn get_port_no(_config: &[String], _input: &[String]) -> MiresidianResult<Option<String>> {
        Err(crate::MiresidianError::NotSupported(
            "当前未实现端口发现能力".into(),
        ))
    }
}

/// 发布平台查询或执行业务前校验实例策略及汇总操作名。
/// 重复或保留操作名、缺少 Config 实例键返回参数错误；缺少 instances feature 返回能力错误。
#[doc(hidden)]
pub fn validate_descriptor(descriptor: &NodeDescriptor) -> MiresidianResult<()> {
    descriptor.instances.validate()?;
    if descriptor.instances.enabled() && !cfg!(feature = "instances") {
        return Err(crate::MiresidianError::NotSupported(
            "框架未启用 instances feature".to_owned(),
        ));
    }
    if descriptor.instances.mode == crate::InstanceMode::Multiple
        && descriptor.instances.binding == crate::InstanceBinding::Config
        && descriptor.init_config.is_empty()
    {
        return Err(crate::MiresidianError::InvalidArgument {
            field: "Config".to_owned(),
            message: "Config 关联的多实例节点至少需要一个实例键参数".to_owned(),
        });
    }
    let mut names = std::collections::HashSet::new();
    for operation in descriptor.operations {
        if matches!(
            operation.name,
            crate::GET_FUNCTIONS_OPERATION
                | crate::GET_FRAMEWORK_VERSION_OPERATION
                | crate::GET_PROJECT_VERSION_OPERATION
                | crate::GET_INIT_CONFIG_OPERATION
                | crate::GET_PORT_NO_OPERATION
                | crate::INIT_DEVICE_OPERATION
                | crate::CLOSE_DEVICE_OPERATION
                | crate::CLOSE_ALL_DEVICES_OPERATION
        ) || !names.insert(operation.name)
        {
            return Err(crate::MiresidianError::InvalidArgument {
                field: "节点声明".to_owned(),
                message: format!("重复或保留的平台操作名称：{}", operation.name),
            });
        }
    }
    Ok(())
}

/// 宏生成的异步业务结果，允许在线程间调度；生命周期绑定本次节点和参数借用。
pub type NodeFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = MiresidianResult<InvocationOutput>> + Send + 'a>,
>;
