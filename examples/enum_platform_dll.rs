//! ABI v10 与平台配套枚举转换的框架内部加载夹具，不访问外部资源。
use miresidian::{
    Invocation, InvocationOutput, MiresidianResult, ParameterDescriptor, Platform, PlatformAdapter,
    RuntimeInvocation, RuntimeResponse,
};

#[derive(miresidian::NodeEnum)]
enum State {
    Idle,
    Running,
}

#[derive(Default)]
struct Alternate;
impl PlatformAdapter for Alternate {
    fn platform(&self) -> Platform {
        Platform::Custom(65_538)
    }
    fn identifier(&self) -> &'static str {
        "alternate_enum"
    }
    fn format_parameter(&self, parameter: &ParameterDescriptor) -> MiresidianResult<String> {
        if !parameter.has_auto_options() {
            return Ok(parameter.definition.to_owned());
        }
        Ok(serde_json::json!({"name":parameter.name,"choices":parameter.enum_options.iter().map(|o| o.value).collect::<Vec<_>>(),"inputPrefix":"pick:"}).to_string())
    }
    fn normalize_parameter(
        &self,
        parameter: &ParameterDescriptor,
        value: &str,
    ) -> MiresidianResult<String> {
        miresidian::normalize_enum_text(
            value.strip_prefix("pick:").unwrap_or(value),
            parameter.enum_options,
            parameter.name,
        )
    }
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation> {
        Ok(RuntimeInvocation::Single(request))
    }
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => Ok(output),
            _ => unreachable!(),
        }
    }
}

#[miresidian::node("枚举 DLL 验证", platforms(Alternate))]
struct Workflow;
impl Workflow {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("状态")] _state: State) -> MiresidianResult<Self> {
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("Echo")]
    fn echo(&mut self, #[miresidian::input("状态")] state: State) -> MiresidianResult<State> {
        Ok(state)
    }
}
