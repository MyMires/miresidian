#![cfg(feature = "format")]
//! 经真实节点调度验证格式选择、输出槽位和批量编解码。
use miresidian::{Invocation, MiresidianResult, NodeRuntime};

#[miresidian::node("格式转换")]
struct FormatNode {
    calls: usize,
}
impl FormatNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("转换")]
    #[miresidian::output("数据", format(formats = [hex, utf8], selector = "输出格式"))]
    #[miresidian::output("次数")]
    fn convert(
        &mut self,
        #[miresidian::input("数据", format(formats = [utf8, hex], selector = "输入格式"))]
        data: Vec<u8>,
        #[miresidian::input("后缀:!")] suffix: String,
    ) -> MiresidianResult<(Vec<u8>, usize)> {
        self.calls += 1;
        let mut data = data;
        data.extend(suffix.as_bytes());
        Ok((data, self.calls))
    }
    #[miresidian::operation("异步")]
    #[miresidian::output("数据", format(formats = [hex]))]
    async fn echo_async(
        &mut self,
        #[miresidian::input("数据", format(formats = [hex]))] data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("批量")]
    #[miresidian::output("数据", format(formats = [hex, utf8], selector = "输出格式"))]
    #[miresidian::output("次数")]
    fn batch(
        &mut self,
        #[miresidian::input("数据", batch, format(formats = [hex, utf8], selector = "输入格式"))]
        data: Vec<u8>,
        #[miresidian::input("倍率", batch)] scale: f64,
    ) -> MiresidianResult<(Vec<u8>, usize)> {
        assert!(scale > 0.0);
        self.calls += 1;
        Ok((data, self.calls))
    }
    #[cfg(feature = "batch")]
    #[miresidian::operation("顺序失败")]
    #[miresidian::output("数据", format(formats = [utf8]))]
    fn fail(&mut self, #[miresidian::input("序号", batch)] n: u16) -> MiresidianResult<Vec<u8>> {
        self.calls += 1;
        Ok(if n == 2 { vec![0xff] } else { vec![b'A'] })
    }
    #[miresidian::output("数据", format(formats = [hex]))]
    #[miresidian::operation("单元组")]
    fn single_tuple(&mut self) -> MiresidianResult<(Vec<u8>,)> {
        Ok((vec![0x41],))
    }

    #[miresidian::operation("次数")]
    fn count(&mut self) -> MiresidianResult<usize> {
        Ok(self.calls)
    }
}
fn runtime() -> NodeRuntime<FormatNode> {
    let runtime = NodeRuntime::new();
    runtime
        .invoke(&Invocation::new("InitDevice", vec![], vec![]))
        .unwrap();
    runtime
}
fn invoke(
    runtime: &NodeRuntime<FormatNode>,
    name: &str,
    input: &[&str],
) -> miresidian::MiresidianResult<miresidian::InvocationOutput> {
    runtime.invoke(&Invocation::new(
        name,
        vec![],
        input.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    ))
}
#[test]
fn descriptors_defaults_and_independent_selectors() {
    let runtime = runtime();
    let functions = invoke(&runtime, "GetFunctions", &[]).unwrap();
    assert!(functions.as_slice().iter().any(|s| s.contains("转换(数据,输入格式:UTF8/HEX,后缀:!,输出格式:HEX/UTF8)(数据,次数)")), "{functions:?}");
    assert_eq!(
        invoke(&runtime, "转换", &["31 32", "HEX", "!", "HEX"])
            .unwrap()
            .as_slice(),
        ["31 32 21", "1"]
    );
    assert_eq!(
        invoke(&runtime, "转换", &["AB", "", "", ""])
            .unwrap()
            .as_slice(),
        ["41 42 21", "2"]
    );
    assert_eq!(
        invoke(&runtime, "转换", &["AB", "", ""])
            .unwrap()
            .as_slice(),
        ["41 42 21", "3"]
    );
    assert!(invoke(&runtime, "转换", &["AB", "UTF8", "!", "Base64"]).is_err());
    assert!(invoke(&runtime, "转换", &["ZZ", "HEX", "!", "HEX"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["3"]);
}
#[test]
fn fixed_format_async_has_no_selector() {
    let runtime = runtime();
    assert_eq!(invoke(&runtime, "单元组", &[]).unwrap().as_slice(), ["41"]);
    assert_eq!(
        invoke(&runtime, "异步", &["31 32"]).unwrap().as_slice(),
        ["31 32"]
    );
}
#[cfg(feature = "batch")]
#[test]
fn batch_bytes_floats_empty_records_and_escaping() {
    let runtime = runtime();
    assert_eq!(
        invoke(
            &runtime,
            "批量",
            &["31 32|44 45", "HEX", "3.14|3.15", "HEX"]
        )
        .unwrap()
        .as_slice(),
        ["31 32|44 45", "1|2"]
    );
    assert_eq!(
        invoke(&runtime, "批量", &["|41", "HEX", "1", "HEX"])
            .unwrap()
            .as_slice(),
        ["|41", "3|4"]
    );
    assert_eq!(
        invoke(&runtime, "批量", &[r"A\|B|C\\D", "UTF8", "1", "UTF8"])
            .unwrap()
            .as_slice(),
        [r"A\|B|C\\D", "5|6"]
    );
    assert!(invoke(&runtime, "批量", &["41|42", "HEX", "1-5", "HEX"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["6"]);
}
#[cfg(feature = "batch")]
#[test]
fn output_conversion_error_stops_subsequent_records() {
    let runtime = runtime();
    assert!(invoke(&runtime, "顺序失败", &["1-3"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["2"]);
}
