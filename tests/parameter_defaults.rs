//! 业务描述仅透传；数值空白为零，缺参报错，字符串不推导默认值。
use miresidian::{DeviceNode, Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[cfg_attr(feature = "instances", miresidian::node("参数描述回归", instances(mode = multiple, binding = config, execution = sequential)))]
#[cfg_attr(not(feature = "instances"), miresidian::node("参数描述回归"))]
struct DescriptionNode {
    baud_rate: u32,
    parity: String,
}
impl DescriptionNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("地址")] _address: String,
        #[miresidian::config("校验:None-无校验/Odd-奇校验")] parity: String,
        #[miresidian::config("波特率:9600/19200")] baud_rate: u32,
    ) -> MiresidianResult<Self> {
        Ok(Self { baud_rate, parity })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("读取配置")]
    fn read_config(&mut self) -> MiresidianResult<(u32, String)> {
        Ok((self.baud_rate, self.parity.clone()))
    }
    #[miresidian::operation("解析数值")]
    fn number(&mut self, #[miresidian::input("超时:1000")] value: u32) -> MiresidianResult<u32> {
        Ok(value)
    }
    #[miresidian::operation("异步数值")]
    async fn async_number(
        &mut self,
        #[miresidian::input("超时:1000")] value: u32,
    ) -> MiresidianResult<u32> {
        Ok(value)
    }
    #[miresidian::operation("文本")]
    fn text(&mut self, #[miresidian::input("文本:")] value: String) -> MiresidianResult<String> {
        Ok(value)
    }
}
fn request(op: &str, input: &[&str]) -> Invocation {
    Invocation::new(
        op,
        vec!["device".into(), "None".into(), "19200".into()],
        input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    )
}
#[test]
fn descriptions_are_unchanged_and_do_not_supply_runtime_defaults() {
    let descriptor = DescriptionNode::descriptor();
    assert_eq!(descriptor.init_config[2].definition, "波特率:9600/19200");
    assert!(
        descriptor
            .init_config
            .iter()
            .all(|p| p.default_value.is_none())
    );
    assert!(
        descriptor
            .operations
            .iter()
            .flat_map(|op| op.inputs)
            .all(|p| p.default_value.is_none())
    );
    let runtime = NodeRuntime::<DescriptionNode>::new();
    for config in [vec!["device".into()], vec!["device".into(), "None".into()]] {
        let result = runtime.invoke(&Invocation::new("InitDevice", config, vec![]));
        if cfg!(feature = "instances") {
            // Default full-config identity is incomplete: skip before parsing, without defaults.
            assert!(result.unwrap().as_slice().is_empty());
            assert!(!runtime.is_initialized());
        } else {
            assert!(matches!(
                result,
                Err(MiresidianError::InvalidArgument { .. })
            ));
        }
    }
    runtime.invoke(&request("InitDevice", &[])).unwrap();
    assert_eq!(
        runtime
            .invoke(&request("读取配置", &[]))
            .unwrap()
            .as_slice(),
        ["19200", "None"]
    );
    for value in ["19200", "波特率:19200"] {
        let output = runtime
            .invoke(&Invocation::new(
                "InitDevice",
                vec!["device".into(), "无校验".into(), value.into()],
                vec![],
            ))
            .unwrap();
        assert_eq!(output.as_slice(), [miresidian::CHANNEL_REUSED_MESSAGE]);
    }
}
#[test]
fn numeric_blanks_are_zero_and_invalid_or_missing_values_fail_sync_and_async() {
    let runtime = NodeRuntime::<DescriptionNode>::new();
    runtime.invoke(&request("InitDevice", &[])).unwrap();
    for op in ["解析数值", "异步数值"] {
        for (value, expected) in [
            ("", "0"),
            ("  ", "0"),
            ("超时:", "0"),
            ("1000", "1000"),
            ("0x10", "16"),
        ] {
            assert_eq!(
                runtime.invoke(&request(op, &[value])).unwrap().as_slice(),
                [expected]
            );
        }
        for values in [vec![], vec!["bad"], vec!["9600/19200"], vec!["4294967296"]] {
            assert_eq!(
                runtime.invoke(&request(op, &values)).unwrap_err().code(),
                "INVALID_ARGUMENT"
            );
        }
    }
}
#[test]
fn strings_and_paths_are_literal_inputs_without_defaults_or_separator_conversion() {
    let runtime = NodeRuntime::<DescriptionNode>::new();
    runtime.invoke(&request("InitDevice", &[])).unwrap();
    for value in [
        "",
        "  ",
        "logs/output.txt",
        "logs/my-file.txt",
        r"logs\my-file.txt",
        "logs//output.txt",
        "9600/19200",
    ] {
        assert_eq!(
            runtime
                .invoke(&request("文本", &[value]))
                .unwrap()
                .as_slice(),
            [value]
        );
    }
    assert_eq!(
        runtime.invoke(&request("文本", &[])).unwrap_err().code(),
        "INVALID_ARGUMENT"
    );
}
#[test]
fn both_ffi_entries_preserve_the_same_parameter_rules() {
    use std::{
        ffi::{CStr, CString},
        sync::OnceLock,
    };
    for explicit in [false, true] {
        let runtime = OnceLock::<NodeRuntime<DescriptionNode>>::new();
        let call = |op: &str, values: &[&str]| {
            let op = CString::new(op).unwrap();
            let config = CString::new(r#"["device","None","19200"]"#).unwrap();
            let input = CString::new(serde_json::to_string(values).unwrap()).unwrap();
            // SAFETY: C 字符串在调用期间有效；响应在下一次调用前复制和解析。
            unsafe {
                let ptr = if explicit {
                    miresidian::ffi_runtime::invoke_with(
                        &runtime,
                        1,
                        op.as_ptr(),
                        config.as_ptr(),
                        input.as_ptr(),
                    )
                } else {
                    miresidian::ffi_runtime::invoke(
                        &runtime,
                        op.as_ptr(),
                        config.as_ptr(),
                        input.as_ptr(),
                    )
                };
                serde_json::from_str::<serde_json::Value>(CStr::from_ptr(ptr).to_str().unwrap())
                    .unwrap()
            }
        };
        assert_eq!(call("InitDevice", &[])["code"], "OK");
        assert_eq!(call("解析数值", &[""])["output"], serde_json::json!(["0"]));
        assert_eq!(call("解析数值", &[])["code"], "INVALID_ARGUMENT");
        assert_eq!(
            call("解析数值", &["9600/19200"])["code"],
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            call("文本", &["logs/my-file.txt"])["output"],
            serde_json::json!(["logs/my-file.txt"])
        );
    }
}
