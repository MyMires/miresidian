//! 默认候选文本、显式输入与实例配置规范化共用同一解析规则。
use miresidian::{DeviceNode, Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[cfg_attr(feature = "instances", miresidian::node("默认候选回归", instances(mode = multiple, binding = config, execution = sequential)))]
#[cfg_attr(not(feature = "instances"), miresidian::node("默认候选回归"))]
struct DefaultTextNode {
    baud_rate: u32,
    parity: String,
}

impl DefaultTextNode {
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
    #[miresidian::output("波特率", "校验")]
    fn read_config(&mut self) -> MiresidianResult<(u32, String)> {
        Ok((self.baud_rate, self.parity.clone()))
    }

    #[miresidian::operation("解析输入")]
    #[miresidian::output("波特率")]
    fn parse_input(
        &mut self,
        #[miresidian::input("波特率:9600/19200")] baud_rate: u32,
    ) -> MiresidianResult<u32> {
        Ok(baud_rate)
    }
}

fn config() -> Vec<String> {
    vec!["device".to_owned(), "None".to_owned()]
}

#[test]
fn 声明元数据保留默认文本且不增加隐式缺省() {
    let descriptor = DefaultTextNode::descriptor();
    assert_eq!(descriptor.init_config[2].default_value, Some("9600/19200"));
    assert_eq!(descriptor.init_config[2].definition, "波特率:9600/19200");
    assert_eq!(descriptor.init_config[1].default_value, None);
    let runtime = NodeRuntime::<DefaultTextNode>::new();
    assert!(
        runtime
            .invoke(&Invocation::new(
                "InitDevice",
                vec!["device".to_owned()],
                vec![]
            ))
            .is_err()
    );
}

#[test]
fn 默认文本与实际默认值规范化到同一实例配置() {
    let runtime = NodeRuntime::<DefaultTextNode>::new();
    runtime
        .invoke(&Invocation::new("InitDevice", config(), vec![]))
        .unwrap();
    for baud_rate in ["", "9600/19200", "9600", "波特率:9600"] {
        let output = runtime
            .invoke(&Invocation::new(
                "InitDevice",
                vec![
                    "device".to_owned(),
                    "无校验".to_owned(),
                    baud_rate.to_owned(),
                ],
                vec![],
            ))
            .unwrap();
        assert_eq!(output.as_slice(), [miresidian::CHANNEL_REUSED_MESSAGE]);
    }
    let output = runtime
        .invoke(&Invocation::new("读取配置", config(), vec![]))
        .unwrap();
    assert_eq!(output.as_slice(), ["9600", "None"]);
}

#[test]
fn 显式配置与操作输入不会被默认候选覆盖() {
    let runtime = NodeRuntime::<DefaultTextNode>::new();
    let explicit_config = vec!["device".to_owned(), "奇校验".to_owned(), "19200".to_owned()];
    runtime
        .invoke(&Invocation::new(
            "InitDevice",
            explicit_config.clone(),
            vec![],
        ))
        .unwrap();
    let output = runtime
        .invoke(&Invocation::new(
            "读取配置",
            explicit_config.clone(),
            vec![],
        ))
        .unwrap();
    assert_eq!(output.as_slice(), ["19200", "Odd"]);
    for (input, expected) in [
        (vec![], "9600"),
        (vec!["".to_owned()], "9600"),
        (vec!["9600/19200".to_owned()], "9600"),
        (vec!["19200".to_owned()], "19200"),
        (vec!["38400".to_owned()], "38400"),
    ] {
        let output = runtime
            .invoke(&Invocation::new("解析输入", explicit_config.clone(), input))
            .unwrap();
        assert_eq!(output.as_slice(), [expected]);
    }
    for value in ["bad", "19200/38400"] {
        let error = runtime
            .invoke(&Invocation::new(
                "解析输入",
                explicit_config.clone(),
                vec![value.to_owned()],
            ))
            .unwrap_err();
        assert!(
            matches!(error, MiresidianError::InvalidArgument { field, .. } if field == "波特率")
        );
    }
}
