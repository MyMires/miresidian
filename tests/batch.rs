#![cfg(feature = "batch")]
//! 参数级 batch 将整数数组对齐为记录，但节点业务方法始终只接收一条标量记录。

use miresidian::{Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[cfg_attr(
    feature = "instances",
    miresidian::node(
        "顺序批量",
        instances(mode = multiple, binding = config, execution = sequential)
    )
)]
#[cfg_attr(not(feature = "instances"), miresidian::node("顺序批量"))]
struct BatchNode;

impl BatchNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("设备")] _device: String) -> MiresidianResult<Self> {
        Ok(Self)
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("读取")]
    #[miresidian::output("站号", "记录")]
    fn read(
        &mut self,
        #[miresidian::input("站号", batch)] station: i32,
        #[miresidian::input("通道", batch)] channel: i32,
        #[miresidian::input("倍率:2")] scale: i32,
    ) -> MiresidianResult<(i32, String)> {
        Ok((station, format!("{station}:{channel}:{}", channel * scale)))
    }
}

fn request(input: &[&str]) -> Invocation {
    Invocation::new(
        "读取",
        vec!["A".to_owned()],
        input
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn batch_aligns_multiple_inputs_broadcasts_single_values_and_reuses_plain_inputs() {
    let runtime = NodeRuntime::<BatchNode>::new();
    runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["A".to_owned()],
            Vec::new(),
        ))
        .expect("初始化");

    assert_eq!(
        runtime
            .invoke(&request(&["1", "1-3|5", "3"]))
            .expect("单站号广播"),
        miresidian::InvocationOutput::new(vec![
            "1|1|1|1".to_owned(),
            "1:1:3|1:2:6|1:3:9|1:5:15".to_owned(),
        ])
    );
    assert_eq!(
        runtime
            .invoke(&request(&["1|2", "1|3", ""]))
            .expect("按位置配对"),
        miresidian::InvocationOutput::new(vec!["1|2".to_owned(), "1:1:2|2:3:6".to_owned(),])
    );
}

#[test]
fn batch_rejects_unaligned_arrays_before_business_execution() {
    let runtime = NodeRuntime::<BatchNode>::new();
    runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["A".to_owned()],
            Vec::new(),
        ))
        .expect("初始化");

    assert!(matches!(
        runtime.invoke(&request(&["1|2", "1|3|5", "2"])),
        Err(MiresidianError::InvalidArgument { field, .. }) if field == "站号"
    ));
}
