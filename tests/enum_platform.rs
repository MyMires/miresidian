//! 同一个业务枚举通过调用平台生成候选和配套解析，参数上不重复候选列表。
use miresidian::{
    Invocation, InvocationOutput, MiresidianError, MiresidianResult, Node, NodeInput, NodeOutput,
    NodeRuntime, ParameterDescriptor, Platform, PlatformAdapter, RuntimeInvocation,
    RuntimeResponse,
};
use serde_json::{Value, json};

#[derive(miresidian::NodeEnum)]
#[cfg_attr(feature = "batch", derive(Clone))]
enum Mode {
    Idle = 4,
    Running = 9,
    #[cfg(feature = "batch")]
    Batched,
}
type ModeAlias = Mode;

#[cfg(feature = "format")]
struct FormatOnly(String);
#[cfg(feature = "format")]
impl miresidian::format::FormatInput for FormatOnly {
    fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self> {
        String::from_utf8(bytes)
            .map(Self)
            .map_err(|error| MiresidianError::InvalidArgument {
                field: field.to_owned(),
                message: error.to_string(),
            })
    }
}

#[derive(Default)]
struct JsonAdapter;
impl PlatformAdapter for JsonAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(65_538)
    }
    fn identifier(&self) -> &'static str {
        "enum_json"
    }
    fn format_parameter(&self, parameter: &ParameterDescriptor) -> MiresidianResult<String> {
        if !parameter.has_auto_options() {
            return Ok(parameter.definition.to_owned());
        }
        Ok(json!({"name":parameter.name,"choices":parameter.enum_options.iter()
            .map(|option| json!({"value":option.value,"label":option.display})).collect::<Vec<_>>()}).to_string())
    }
    fn normalize_parameter(
        &self,
        parameter: &ParameterDescriptor,
        value: &str,
    ) -> MiresidianResult<String> {
        let structured: Option<Value> = serde_json::from_str(value).ok();
        let value = structured
            .as_ref()
            .and_then(|value| value.get("selected"))
            .and_then(Value::as_str)
            .unwrap_or(value);
        miresidian::normalize_enum_text(value, parameter.enum_options, parameter.name)
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

#[derive(Default)]
struct Unsupported;
impl PlatformAdapter for Unsupported {
    fn platform(&self) -> Platform {
        Platform::Custom(65_539)
    }
    fn identifier(&self) -> &'static str {
        "enum_unsupported"
    }
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation> {
        JsonAdapter.decode(request)
    }
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        JsonAdapter.encode(response)
    }
}

struct DirectAdapter;
impl PlatformAdapter for DirectAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(65_540)
    }
    fn identifier(&self) -> &'static str {
        "direct_unregistered"
    }
    fn format_parameter(&self, parameter: &ParameterDescriptor) -> MiresidianResult<String> {
        Ok(format!(
            "direct:{}",
            JsonAdapter.format_parameter(parameter)?
        ))
    }
    fn normalize_parameter(
        &self,
        parameter: &ParameterDescriptor,
        value: &str,
    ) -> MiresidianResult<String> {
        JsonAdapter.normalize_parameter(parameter, value)
    }
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation> {
        JsonAdapter.decode(request)
    }
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        JsonAdapter.encode(response)
    }
}

#[miresidian::node("枚举业务", platforms(JsonAdapter, Unsupported))]
struct Workflow {
    mode: Mode,
    seed: Option<Mode>,
}
impl Workflow {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("配置状态")] mode: ModeAlias,
        #[miresidian::input("初始值:")] seed: Option<Mode>,
    ) -> MiresidianResult<Self> {
        Ok(Self { mode, seed })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn get_port_no(
        #[miresidian::input("探测状态")] mode: Mode,
    ) -> MiresidianResult<Option<String>> {
        Ok(Some(mode.into_output()?.into_values().remove(0)))
    }
    #[miresidian::operation("当前值")]
    fn current(&mut self) -> MiresidianResult<(String, String)> {
        Ok((
            Mode::normalize_text(
                match self.mode {
                    Mode::Idle => "Idle",
                    _ => "Running",
                },
                "状态",
            )?,
            if self.seed.is_some() { "Some" } else { "None" }.to_owned(),
        ))
    }
    #[miresidian::operation("回传")]
    fn echo(&mut self, #[miresidian::input("状态")] mode: Mode) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[miresidian::operation("异步回传")]
    async fn echo_async(
        &mut self,
        #[miresidian::input("状态:")] mode: crate::Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[miresidian::operation("可选")]
    fn optional(
        &mut self,
        #[miresidian::input("状态")] mode: Option<Mode>,
    ) -> MiresidianResult<Option<Mode>> {
        Ok(mode)
    }
    #[miresidian::operation("列表")]
    fn list(
        &mut self,
        #[miresidian::input("状态")] modes: Vec<Mode>,
    ) -> MiresidianResult<Vec<Mode>> {
        Ok(modes)
    }
    #[miresidian::operation("显示值")]
    fn manual(
        &mut self,
        #[miresidian::input("状态:Idle-空闲/Running-运行中")] mode: Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[miresidian::operation("手写预填")]
    fn prefill(
        &mut self,
        #[miresidian::input("状态:Running")] mode: Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }

    #[cfg(feature = "format")]
    #[miresidian::operation("格式类型")]
    fn formatted(
        &mut self,
        #[miresidian::input("数据", format(utf8))] value: FormatOnly,
    ) -> MiresidianResult<String> {
        Ok(value.0)
    }
    #[cfg(feature = "batch")]
    #[miresidian::operation("批次")]
    fn batch(
        &mut self, #[miresidian::input("状态", batch)] mode: Mode
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
}

fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        input.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
    )
}

#[test]
fn auto_candidates_are_structured_and_rendered_per_actual_adapter() {
    let descriptor = Workflow::descriptor();
    assert_eq!(descriptor.init_config[0].definition, "配置状态");
    assert!(descriptor.init_config[0].default_value.is_none());
    assert_eq!(
        descriptor.init_config[0].enum_options,
        <Mode as NodeInput>::OPTIONS
    );
    let choices = <Mode as NodeInput>::OPTIONS
        .iter()
        .map(|o| o.value)
        .collect::<Vec<_>>()
        .join("/");
    assert_eq!(
        choices,
        if cfg!(feature = "batch") {
            "Idle/Running/Batched"
        } else {
            "Idle/Running"
        }
    );
    let runtime = NodeRuntime::<Workflow>::with_platforms(Workflow::platform_registry());
    let tb = runtime.invoke(&request("GetInitConfig", &[], &[])).unwrap();
    assert_eq!(tb.as_slice(), [format!("配置状态:{choices}")]);
    let custom = runtime
        .invoke_with_platform(Platform::Custom(65_538), request("GetInitConfig", &[], &[]))
        .unwrap();
    let parsed: Value = serde_json::from_str(&custom.as_slice()[0]).unwrap();
    assert_eq!(parsed["choices"][0], json!({"value":"Idle","label":"Idle"}));
    assert_eq!(
        runtime.invoke(&request("GetInitConfig", &[], &[])).unwrap(),
        tb
    );
    let direct = runtime
        .invoke_with(&DirectAdapter, request("GetInitConfig", &[], &[]))
        .unwrap();
    assert!(direct.as_slice()[0].starts_with("direct:{"));
    let functions = runtime.invoke(&request("GetFunctions", &[], &[])).unwrap();
    for exact in [
        format!("回传(状态:{choices})(返回值)"),
        format!("异步回传(状态:{choices})(返回值)"),
        "显示值(状态:Idle-空闲/Running-运行中)(返回值)".into(),
        "手写预填(状态:Running)(返回值)".into(),
    ] {
        assert!(functions.as_slice().contains(&exact), "{exact}");
    }
    assert!(
        functions
            .as_slice()
            .iter()
            .all(|s| !s.contains("平台适配:"))
    );
}

#[test]
fn codec_converts_alias_optional_collection_and_manual_display_without_defaults() {
    let runtime = NodeRuntime::<Workflow>::new();
    runtime
        .invoke(&request("InitDevice", &["配置状态:Running"], &["Idle"]))
        .unwrap();
    for operation in ["回传", "异步回传"] {
        assert_eq!(
            runtime
                .invoke(&request(operation, &["Running"], &["状态:Idle"]))
                .unwrap()
                .as_slice(),
            ["Idle"]
        );
        for input in [vec![], vec![""], vec!["Idle/Running"], vec!["bad"]] {
            assert!(matches!(
                runtime.invoke(&request(operation, &["Running"], &input)),
                Err(MiresidianError::InvalidArgument { .. })
            ));
        }
    }
    assert_eq!(
        runtime
            .invoke(&request("显示值", &["Running"], &["空闲"]))
            .unwrap()
            .as_slice(),
        ["Idle"]
    );
    assert!(
        runtime
            .invoke(&request("手写预填", &["Running"], &[]))
            .is_err()
    );
    assert_eq!(
        runtime
            .invoke(&request("可选", &["Running"], &[""]))
            .unwrap()
            .as_slice(),
        [""]
    );
    assert_eq!(
        runtime
            .invoke(&request("列表", &["Running"], &["Idle|Running"]))
            .unwrap()
            .as_slice(),
        ["Idle|Running"]
    );
    assert_eq!(
        runtime
            .invoke(&request("GetPortNO", &[], &["Running"]))
            .unwrap()
            .as_slice(),
        ["1", "Running"]
    );
    #[cfg(feature = "batch")]
    assert_eq!(
        runtime
            .invoke(&request("批次", &["Running"], &["Idle|Running"]))
            .unwrap()
            .as_slice(),
        ["Idle|Running"]
    );
    #[cfg(feature = "format")]
    assert_eq!(
        runtime
            .invoke(&request("格式类型", &["Running"], &["业务文本"]))
            .unwrap()
            .as_slice(),
        ["业务文本"]
    );
}

#[test]
fn unsupported_platform_must_opt_into_both_enum_halves() {
    let runtime = NodeRuntime::<Workflow>::new();
    assert!(matches!(
        runtime.invoke_with(&Unsupported, request("GetInitConfig", &[], &[])),
        Err(MiresidianError::NotSupported(_))
    ));
    assert!(matches!(
        runtime.invoke_with(&Unsupported, request("InitDevice", &["Running"], &["Idle"])),
        Err(MiresidianError::NotSupported(_))
    ));
    assert!(!runtime.is_initialized());
    assert!(
        runtime
            .invoke_with(&Unsupported, request("GetFrameworkVersion", &[], &[]))
            .is_ok()
    );
}

#[test]
fn single_json_export_pairs_platform_candidate_format_with_input_conversion() {
    use std::ffi::{CStr, CString};
    let call = |value: Value| -> Value {
        let value = CString::new(value.to_string()).unwrap();
        // SAFETY: 请求有效，返回值在下一次调用前复制。
        let pointer = unsafe { miresidian_invoke(value.as_ptr()) };
        serde_json::from_str(unsafe { CStr::from_ptr(pointer) }.to_str().unwrap()).unwrap()
    };
    assert_eq!(miresidian_abi_version(), 10);
    let metadata = call(json!({"platform":65538,"function":"GetInitConfig"}));
    assert_eq!(metadata["code"], "OK");
    assert!(metadata["output"][0].as_str().unwrap().starts_with('{'));
    assert_eq!(
        call(
            json!({"platform":65538,"function":"InitDevice","config":["{\"selected\":\"Running\"}"],"input":["{\"selected\":\"Idle\"}"]})
        )["code"],
        "OK"
    );
    assert_eq!(
        call(
            json!({"platform":65538,"function":"回传","config":["{\"selected\":\"Running\"}"],"input":["{\"selected\":\"Idle\"}"]})
        )["output"],
        json!(["Idle"])
    );
    assert_eq!(
        call(json!({"platform":0,"function":"GetInitConfig"}))["code"],
        "INVALID_ARGUMENT"
    );
    assert_eq!(
        call(json!({"platform":65538,"function":"CloseAllDevices"}))["code"],
        "OK"
    );
    assert!(
        call(json!({"function":"GetInitConfig"}))["output"][0]
            .as_str()
            .unwrap()
            .starts_with("配置状态:Idle/Running")
    );
}
