//! 平台输入输出与平台无关运行时命令之间的适配边界。
//!
//! 运行时只处理结构化调用和共享槽位；函数名、文本编码和平台约定由各平台适配器负责。

mod tb_lv_test_platform;

use std::{collections::BTreeMap, fmt, sync::Arc};

use crate::{Invocation, InvocationOutput, MiresidianResult};

pub use tb_lv_test_platform::{TB_LV_TEST_PLATFORM, TbLvTestPlatformAdapter};

/// 平台的稳定类型标识。
///
/// 内置平台使用框架保留的低位编号；第三方平台必须使用 `Custom` 且编号不小于
/// `Platform::CUSTOM_MIN`，避免未来内置平台与扩展平台冲突。该类型直接映射到 C ABI 的
/// `uint32_t` 参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    /// 当前 TB LV Test Platform 兼容适配器。
    TbLvTestPlatform,
    /// 已注册的第三方或私有平台。
    Custom(u32),
}

impl Platform {
    /// TB LV Test Platform 在 C ABI 中的稳定数值。
    pub const TB_LV_TEST_PLATFORM: u32 = 1;
    /// 第三方平台可分配编号的起始值。
    pub const CUSTOM_MIN: u32 = 0x0001_0000;

    /// 返回 C ABI 使用的稳定无符号整数。
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::TbLvTestPlatform => Self::TB_LV_TEST_PLATFORM,
            Self::Custom(value) => value,
        }
    }

    /// 由 C ABI 数值解析平台。保留编号和零值会被拒绝。
    pub const fn from_abi(value: u32) -> Option<Self> {
        match value {
            Self::TB_LV_TEST_PLATFORM => Some(Self::TbLvTestPlatform),
            Self::CUSTOM_MIN..=u32::MAX => Some(Self::Custom(value)),
            _ => None,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TbLvTestPlatform => formatter.write_str(TB_LV_TEST_PLATFORM),
            Self::Custom(value) => write!(formatter, "Custom({value})"),
        }
    }
}

/// 运行时可选择的平台适配器集合。
///
/// `InitDevice` 的保留 Config `平台适配` 从此集合选择适配器。所有已初始化通道必须
/// 使用同一个平台，最后一个通道关闭后可重新选择。
#[derive(Clone)]
pub struct PlatformRegistry {
    default_platform: Platform,
    adapters: BTreeMap<Platform, Arc<dyn PlatformAdapter>>,
}

impl PlatformRegistry {
    /// 以默认适配器创建注册表。
    pub fn new<A: PlatformAdapter>(default_adapter: A) -> Self {
        let default_platform = default_adapter.platform();
        let mut adapters: BTreeMap<Platform, Arc<dyn PlatformAdapter>> = BTreeMap::new();
        adapters.insert(default_platform, Arc::new(default_adapter));
        Self {
            default_platform,
            adapters,
        }
    }

    /// 注册或替换一个可由 `平台适配` 选择的适配器。平台数值与文本标识均保持唯一；
    /// 若替换了默认适配器的文本标识，默认平台数值也随之更新，确保默认调用始终可解析。
    pub fn register<A: PlatformAdapter>(&mut self, adapter: A) {
        let platform = adapter.platform();
        let replaces_default = self
            .adapters
            .get(&self.default_platform)
            .is_some_and(|default| default.identifier() == adapter.identifier());
        self.adapters.retain(|registered, existing| {
            *registered == platform || existing.identifier() != adapter.identifier()
        });
        self.adapters.insert(platform, Arc::new(adapter));
        if replaces_default {
            self.default_platform = platform;
        }
    }

    /// 链式注册一个适配器。
    pub fn with_adapter<A: PlatformAdapter>(mut self, adapter: A) -> Self {
        self.register(adapter);
        self
    }

    /// 默认平台的类型标识。
    pub const fn default_platform(&self) -> Platform {
        self.default_platform
    }

    /// 根据类型标识返回适配器。
    pub fn adapter(&self, platform: Platform) -> Option<Arc<dyn PlatformAdapter>> {
        self.adapters.get(&platform).cloned()
    }

    /// 根据旧 Config 文本标识返回适配器，供默认 C ABI 路径兼容使用。
    pub fn adapter_by_identifier(&self, identifier: &str) -> Option<Arc<dyn PlatformAdapter>> {
        self.adapters
            .values()
            .find(|adapter| adapter.identifier() == identifier)
            .cloned()
    }

    /// 根据旧 Config 文本标识返回平台类型。
    pub fn platform_by_identifier(&self, identifier: &str) -> Option<Platform> {
        self.adapters.iter().find_map(|(platform, adapter)| {
            (adapter.identifier() == identifier).then_some(*platform)
        })
    }

    /// 平台稳定文本标识列表，供兼容初始化元数据使用。
    pub fn identifiers(&self) -> Vec<&str> {
        let mut identifiers = self
            .adapters
            .values()
            .map(|adapter| adapter.identifier())
            .collect::<Vec<_>>();
        identifiers.sort_unstable();
        identifiers
    }

    /// `InitDevice` 中平台选择器的发布定义。
    pub fn selector_definition(&self) -> String {
        format!("平台适配:{}", self.identifiers().join("/"))
    }
}

impl Default for PlatformRegistry {
    fn default() -> Self {
        Self::new(TbLvTestPlatformAdapter)
    }
}

/// 平台无关的运行时调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeInvocation {
    /// 单通道或常驻查询调用。
    Single(Invocation),
    /// 多通道共享调用。
    Shared(SharedInvocation),
}

/// 共享调用的结构化参数。第 N 个槽位对应第 N 个已初始化通道；`None` 表示跳过该通道。
/// 返回槽位与此数组保持相同顺序和数量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedInvocation {
    pub operation: String,
    pub slots: Vec<Option<Vec<String>>>,
}

impl SharedInvocation {
    pub fn new(operation: impl Into<String>, slots: Vec<Option<Vec<String>>>) -> Self {
        Self {
            operation: operation.into(),
            slots,
        }
    }
}

/// 平台无关的运行时返回值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeResponse {
    Output(InvocationOutput),
    Shared(SharedResponse),
}

/// 共享调用结果。槽位与请求输入保持相同顺序和数量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedResponse {
    pub slots: Vec<Option<InvocationOutput>>,
}

impl SharedResponse {
    pub(crate) fn new(slots: Vec<Option<InvocationOutput>>) -> Self {
        Self { slots }
    }
}

/// 将具体平台的请求和响应映射为运行时命令。
///
/// 新平台应实现此 trait，而不应将其专有的函数命名、参数拼接或输出编码写入运行时。
pub trait PlatformAdapter: Send + Sync + 'static {
    /// 平台稳定类型标识；会映射为 C ABI 的 `uint32_t`。
    fn platform(&self) -> Platform;

    /// 供默认 Config 兼容路径与诊断使用的平台文本标识，例如 `tb_lv_test_platform`。
    fn identifier(&self) -> &'static str;

    /// C ABI 统一后的文本请求。适配器可在此解析平台自己的名称、输入编码和共享协议。
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation>;

    /// 将运行时结果编码为当前 C ABI 的文本输出。
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput>;
}
