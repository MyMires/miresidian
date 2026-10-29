//! 实例数量、对象关联与多通道执行策略；基础模型不依赖可选实现。

use miresidian_codec::{MiresidianError, MiresidianResult};

/// 平台各通道共享同一节点对象时的输入执行方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 在当前线程上按通道顺序依次执行。
    Sequential,
    /// 为各平台输入分发独立线程；同一对象仍互斥执行，不保证输入开始顺序。
    Parallel,
    /// 校验有效输入相同后，按平台通道顺序在同一对象上逐项执行。
    Broadcast,
}

impl ExecutionMode {
    /// 多通道执行方式的稳定平台实际值。
    pub fn definition(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential",
            Self::Parallel => "Parallel",
            Self::Broadcast => "Broadcast",
        }
    }

    /// 解析平台传入的实际值或显示值；空白和未知值安全回退为顺序执行。
    pub fn parse_platform_value(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "parallel" | "并行" => Self::Parallel,
            "broadcast" | "广播" => Self::Broadcast,
            "sequential" | "顺序" => Self::Sequential,
            _ => Self::Sequential,
        }
    }
}

/// 节点可同时保留的实例数量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceMode {
    /// 最多一个对象；不同配置初始化会先释放原对象。
    Single,
    /// 按关联键分别保留对象。
    Multiple,
}

/// 多实例对象的唯一关联键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceBinding {
    /// 使用规范化后的业务 `Config[0]`；同主键配置一致时复用对象。
    Config,
    /// 使用 Config 中的框架 Handle；不同 Handle 不按业务配置合并。
    Handle,
}

/// 多通道输入的执行策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPolicy {
    /// 默认节点只接受普通调用，不发布执行方式参数。
    Disabled,
    /// 固定方式，不发布执行方式参数。
    Fixed(ExecutionMode),
    /// 初始化时由平台选择；缺省或空白使用此默认方式。
    Selectable(ExecutionMode),
}

impl ExecutionPolicy {
    /// 是否向平台发布执行方式选择器。
    pub const fn is_selectable(self) -> bool {
        matches!(self, Self::Selectable(_))
    }

    pub fn resolve(self, value: Option<&str>) -> MiresidianResult<ExecutionMode> {
        match self {
            Self::Disabled => Ok(ExecutionMode::Sequential),
            Self::Fixed(mode) => Ok(mode),
            Self::Selectable(default) => match value.map(str::trim).filter(|s| !s.is_empty()) {
                None => Ok(default),
                // 保留 TB 既有未知值回退规则。
                Some(value) => Ok(ExecutionMode::parse_platform_value(Some(value))),
            },
        }
    }
}

/// 由节点宏生成的实例策略；单纯开启 Cargo feature 不改变节点默认行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstancePolicy {
    pub mode: InstanceMode,
    pub binding: InstanceBinding,
    pub execution: ExecutionPolicy,
}

impl InstancePolicy {
    /// 默认单实例、Config 配置校验、禁用多通道请求。
    pub const DEFAULT: Self = Self {
        mode: InstanceMode::Single,
        binding: InstanceBinding::Config,
        execution: ExecutionPolicy::Disabled,
    };

    pub const fn enabled(self) -> bool {
        !matches!(self.execution, ExecutionPolicy::Disabled)
    }

    pub const fn has_handle(self) -> bool {
        matches!(self.binding, InstanceBinding::Handle)
    }

    pub fn validate(self) -> MiresidianResult<()> {
        let invalid = if self.mode == InstanceMode::Single && self.has_handle() {
            Some("single 模式不接受 Handle 关联")
        } else if self.mode == InstanceMode::Multiple && !self.enabled() {
            Some("multiple 模式必须声明多通道执行策略")
        } else {
            None
        };
        match invalid {
            Some(message) => Err(MiresidianError::InvalidArgument {
                field: "instances".to_owned(),
                message: message.to_owned(),
            }),
            None => Ok(()),
        }
    }
}

impl Default for InstancePolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 描述发布与请求解析共用的框架配置布局。平台选择器仍允许兼容性省略。
pub struct ConfigLayout {
    pub business: usize,
    pub handle: Option<usize>,
    pub lease: Option<usize>,
    pub platform: usize,
    pub execution: bool,
}

impl ConfigLayout {
    pub fn new(business: usize, policy: InstancePolicy, lease_selector: bool) -> Self {
        let handle = policy.has_handle().then_some(business);
        let lease_index = business + usize::from(handle.is_some());
        let lease = lease_selector.then_some(lease_index);
        Self {
            business,
            handle,
            lease,
            platform: lease_index + usize::from(lease.is_some()),
            execution: policy.execution.is_selectable(),
        }
    }
}
