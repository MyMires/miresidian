//! 平台输入输出与平台无关运行时命令之间的适配边界。
//!
//! 运行时只处理结构化调用和共享槽位；函数名、文本编码和平台约定由各平台适配器负责。

mod tb_lv_test_platform;

use std::{collections::BTreeMap, sync::Arc};

use crate::{Invocation, InvocationOutput, MiresidianResult};

pub use tb_lv_test_platform::{TB_LV_TEST_PLATFORM, TbLvTestPlatformAdapter};

/// 运行时可选择的平台适配器集合。
///
/// `InitDevice` 的保留 Config `平台适配` 从此集合选择适配器。所有已初始化通道必须
/// 使用同一个平台，最后一个通道关闭后可重新选择。
#[derive(Clone)]
pub struct PlatformRegistry {
    default_identifier: String,
    adapters: BTreeMap<String, Arc<dyn PlatformAdapter>>,
}

impl PlatformRegistry {
    /// 以默认适配器创建注册表。
    pub fn new<A: PlatformAdapter>(default_adapter: A) -> Self {
        let default_identifier = default_adapter.identifier().to_owned();
        let mut adapters: BTreeMap<String, Arc<dyn PlatformAdapter>> = BTreeMap::new();
        adapters.insert(default_identifier.clone(), Arc::new(default_adapter));
        Self {
            default_identifier,
            adapters,
        }
    }

    /// 注册或替换一个可由 `平台适配` 选择的适配器。
    pub fn register<A: PlatformAdapter>(&mut self, adapter: A) {
        self.adapters
            .insert(adapter.identifier().to_owned(), Arc::new(adapter));
    }

    /// 链式注册一个适配器。
    pub fn with_adapter<A: PlatformAdapter>(mut self, adapter: A) -> Self {
        self.register(adapter);
        self
    }

    /// 默认平台的稳定标识。
    pub fn default_identifier(&self) -> &str {
        &self.default_identifier
    }

    /// 返回指定平台的适配器。
    pub fn adapter(&self, identifier: &str) -> Option<Arc<dyn PlatformAdapter>> {
        self.adapters.get(identifier).cloned()
    }

    /// 平台稳定标识列表，供发布初始化元数据使用。
    pub fn identifiers(&self) -> impl Iterator<Item = &str> {
        self.adapters.keys().map(String::as_str)
    }

    /// `InitDevice` 中平台选择器的发布定义。
    pub fn selector_definition(&self) -> String {
        format!(
            "平台适配:{}",
            self.identifiers().collect::<Vec<_>>().join("/")
        )
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

/// 共享调用的结构化参数。每个槽位对应一个已初始化通道；`None` 表示跳过该通道。
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

/// 共享调用结果。槽位数始终与运行时内已初始化通道数一致。
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
    /// 平台稳定标识，例如 `tb_lv_test_platform`。
    fn identifier(&self) -> &'static str;

    /// C ABI 统一后的文本请求。适配器可在此解析平台自己的名称、输入编码和共享协议。
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation>;

    /// 将运行时结果编码为当前 C ABI 的文本输出。
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput>;
}
