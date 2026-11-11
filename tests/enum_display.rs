//! 枚举成员显示名的长短标注贯通实际平台，直接 codec 保持实际值契约。
use miresidian::{
    EnumOption, Invocation, InvocationOutput, MiresidianResult, Node, NodeInput, NodeOutput,
    NodeRuntime, ParameterDescriptor, Platform, PlatformAdapter, RuntimeInvocation,
    RuntimeResponse, TbLvTestPlatformAdapter,
};
use serde_json::{Value, json};

#[derive(Clone, miresidian::NodeEnum)]
enum Mode {
    #[node_enum(display = "空闲")]
    Idle = 4,
    #[node_enum("运行中")]
    Running = 9,
    Other,
    #[cfg(any())]
    #[node_enum("不可见")]
    Disabled,
}

#[derive(miresidian::NodeEnum)]
enum Conditional {
    #[cfg(feature = "batch")]
    #[node_enum(display = "批次")]
    Batched,
    #[cfg(not(feature = "batch"))]
    #[node_enum("普通")]
    Plain,
}

#[derive(miresidian::NodeEnum)]
enum ConditionalLabel {
    #[cfg_attr(feature = "batch", node_enum("批次"))]
    #[cfg_attr(not(feature = "batch"), node_enum(display = "普通"))]
    Item,
}

#[derive(Default)]
struct JsonAdapter;
impl PlatformAdapter for JsonAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(65_538)
    }
    fn identifier(&self) -> &'static str {
        "enum_display_json"
    }
    fn format_parameter(&self, parameter: &ParameterDescriptor) -> MiresidianResult<String> {
        if !parameter.has_auto_options() {
            return Ok(parameter.definition.to_owned());
        }
        Ok(
            json!({"name": parameter.name, "choices": parameter.enum_options.iter()
            .map(|option| json!({"value": option.value, "label": option.display}))
            .collect::<Vec<_>>()})
            .to_string(),
        )
    }
    fn normalize_parameter(
        &self,
        parameter: &ParameterDescriptor,
        value: &str,
    ) -> MiresidianResult<String> {
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

#[cfg_attr(
    feature = "instance",
    miresidian::node("成员显示名", platforms(JsonAdapter), instance)
)]
#[cfg_attr(
    not(feature = "instance"),
    miresidian::node("成员显示名", platforms(JsonAdapter))
)]
struct Workflow;
impl Workflow {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("配置状态")] _mode: Mode,
        #[miresidian::input("初始值")] seed: Option<Mode>,
    ) -> MiresidianResult<(Self, Option<Mode>)> {
        Ok((Self, seed))
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn get_port_no(#[miresidian::input("状态")] mode: Mode) -> MiresidianResult<Option<String>> {
        Ok(Some(mode.into_output()?.into_values().remove(0)))
    }
    #[miresidian::operation]
    fn echo(&mut self, #[miresidian::input("状态")] mode: Mode) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[miresidian::operation]
    async fn echo_async(
        &mut self,
        #[miresidian::input("状态")] mode: Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[miresidian::operation]
    fn optional(
        &mut self,
        #[miresidian::input("状态")] mode: Option<Mode>,
    ) -> MiresidianResult<Option<Mode>> {
        Ok(mode)
    }
    #[miresidian::operation]
    fn list(
        &mut self,
        #[miresidian::input("状态")] modes: Vec<Mode>,
    ) -> MiresidianResult<Vec<Mode>> {
        Ok(modes)
    }
    #[miresidian::operation]
    fn explicit(
        &mut self,
        #[miresidian::input("状态:Idle-待机/Running-工作")] mode: Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
    #[cfg(feature = "batch")]
    #[miresidian::operation]
    fn batched(
        &mut self,
        #[miresidian::input("状态", batch)] mode: Mode,
    ) -> MiresidianResult<Mode> {
        Ok(mode)
    }
}

fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        input
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn labels_are_metadata_and_codec_keeps_member_names() {
    assert_eq!(
        <Mode as NodeInput>::OPTIONS,
        [
            EnumOption {
                value: "Idle",
                display: "空闲"
            },
            EnumOption {
                value: "Running",
                display: "运行中"
            },
            EnumOption {
                value: "Other",
                display: "Other"
            },
        ]
    );
    assert_eq!(
        Mode::parse_text("Idle", "状态")
            .unwrap()
            .into_output()
            .unwrap()
            .as_slice(),
        ["Idle"]
    );
    assert!(Mode::parse_text("空闲", "状态").is_err());
    assert_eq!(
        Mode::normalize_text(" Running ", "状态").unwrap(),
        "Running"
    );
    assert_eq!(
        <Option<Mode> as NodeInput>::OPTIONS,
        <Mode as NodeInput>::OPTIONS
    );
    assert_eq!(
        <Vec<Mode> as NodeInput>::OPTIONS,
        <Mode as NodeInput>::OPTIONS
    );
    let conditional = <Conditional as NodeInput>::OPTIONS;
    assert_eq!(conditional.len(), 1);
    assert_eq!(
        conditional[0].display,
        if cfg!(feature = "batch") {
            "批次"
        } else {
            "普通"
        }
    );
    assert_eq!(
        <ConditionalLabel as NodeInput>::OPTIONS[0].display,
        conditional[0].display
    );
}

#[test]
fn both_platforms_render_labels_and_normalize_all_input_channels() {
    let runtime = NodeRuntime::<Workflow>::with_platforms(Workflow::platform_registry());
    let tb = runtime.invoke(&request("GetInitConfig", &[], &[])).unwrap();
    assert_eq!(tb.as_slice(), ["配置状态:Idle-空闲/Running-运行中/Other"]);
    let custom = runtime
        .invoke_with_platform(Platform::Custom(65_538), request("GetInitConfig", &[], &[]))
        .unwrap();
    let parsed: Value = serde_json::from_str(&custom.as_slice()[0]).unwrap();
    assert_eq!(
        parsed["choices"],
        json!([
            {"value":"Idle", "label":"空闲"},
            {"value":"Running", "label":"运行中"},
            {"value":"Other", "label":"Other"},
        ])
    );
    assert_eq!(
        runtime.invoke(&request("GetInitConfig", &[], &[])).unwrap(),
        tb
    );
    let functions = runtime.invoke(&request("GetFunctions", &[], &[])).unwrap();
    assert!(
        functions
            .as_slice()
            .iter()
            .any(|line| line == "Explicit(状态:Idle-待机/Running-工作)(返回值)")
    );

    for platform in [Platform::TbLvTestPlatform, Platform::Custom(65_538)] {
        let call = |operation, config: &[&str], input: &[&str]| {
            runtime.invoke_with_platform(platform, request(operation, config, input))
        };
        assert_eq!(
            call("GetPortNO", &[], &["运行中"]).unwrap().as_slice(),
            ["1", "Running"]
        );
        assert_eq!(
            call("InitDevice", &["空闲"], &["运行中"])
                .unwrap()
                .as_slice(),
            ["Running"]
        );
        for operation in ["Echo", "EchoAsync", "Optional"] {
            assert_eq!(
                call(operation, &["Idle"], &["运行中"]).unwrap().as_slice(),
                ["Running"]
            );
        }
        assert_eq!(
            call("List", &["空闲"], &["空闲|运行中|Other"])
                .unwrap()
                .as_slice(),
            ["Idle|Running|Other"]
        );
        assert_eq!(call("Optional", &["Idle"], &[""]).unwrap().as_slice(), [""]);
        assert!(call("Echo", &["Idle"], &["未知"]).is_err());
        if platform == Platform::TbLvTestPlatform {
            assert_eq!(
                call("Explicit", &["Idle"], &["待机"]).unwrap().as_slice(),
                ["Idle"]
            );
            #[cfg(feature = "batch")]
            assert_eq!(
                call("Batched", &["Idle"], &["空闲|运行中"])
                    .unwrap()
                    .as_slice(),
                ["Idle|Running"]
            );
            #[cfg(feature = "instance")]
            assert_eq!(
                call("Echo", &["Idle"], &["共享", "空闲", "", "运行中"])
                    .unwrap()
                    .as_slice(),
                ["Idle", "", "Running"]
            );
        }
        call("CloseAllDevices", &[], &[]).unwrap();
    }
}

#[test]
fn platform_specific_label_restrictions_stay_in_adapter() {
    #[derive(miresidian::NodeEnum)]
    enum Delimiter {
        #[node_enum("A/B")]
        Item,
    }
    #[derive(miresidian::NodeEnum)]
    enum Ambiguous {
        #[node_enum("Other")]
        Item,
        Other,
    }
    for options in [
        <Delimiter as NodeInput>::OPTIONS,
        <Ambiguous as NodeInput>::OPTIONS,
    ] {
        let parameter = ParameterDescriptor {
            name: "状态",
            definition: "状态",
            default_value: None,
            enum_options: options,
        };
        assert!(
            TbLvTestPlatformAdapter
                .format_parameter(&parameter)
                .is_err()
        );
        assert!(JsonAdapter.format_parameter(&parameter).is_ok());
    }
}
