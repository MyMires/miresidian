//! 通用业务类型通过公开接口完成输入、集合、输出和配置规范化。
use miresidian::{
    Invocation, InvocationOutput, MiresidianError, MiresidianResult, Node, NodeCollectionElement,
    NodeInput, NodeOutput, NodeRuntime, NodeScalar, NodeValue,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkflowState {
    Queued,
    Done,
}

impl WorkflowState {
    fn name(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Done => "Done",
        }
    }
}

impl NodeInput for WorkflowState {
    fn parse_text(value: &str, field: &str) -> MiresidianResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "queued" => Ok(Self::Queued),
            "done" => Ok(Self::Done),
            _ => Err(MiresidianError::InvalidArgument {
                field: field.to_owned(),
                message: "流程状态不受支持".to_owned(),
            }),
        }
    }

    fn normalize_text(value: &str, field: &str) -> MiresidianResult<String> {
        Ok(Self::parse_text(value, field)?.name().to_owned())
    }
}

impl NodeOutput for WorkflowState {
    fn into_output(self) -> MiresidianResult<InvocationOutput> {
        self.name().to_owned().into_output()
    }
}

impl NodeCollectionElement for WorkflowState {
    fn parse_collection(value: &str, field: &str) -> MiresidianResult<Vec<Self>> {
        Vec::<String>::parse_text(value, field)?
            .iter()
            .map(|item| Self::parse_text(item, field))
            .collect()
    }

    fn format_collection(values: &[Self]) -> MiresidianResult<String> {
        String::format_collection(
            &values
                .iter()
                .map(|value| value.name().to_owned())
                .collect::<Vec<_>>(),
        )
    }
}

#[miresidian::node("流程节点")]
struct Workflow {
    state: WorkflowState,
}

impl Workflow {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("初始状态:Queued-排队/Done-完成")] state: WorkflowState,
    ) -> MiresidianResult<Self> {
        Ok(Self { state })
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("当前状态")]
    fn state(&mut self) -> MiresidianResult<WorkflowState> {
        Ok(self.state.clone())
    }

    #[miresidian::operation("回传状态")]
    fn echo(
        &mut self,
        #[miresidian::input("状态:Queued-排队/Done-完成")] states: Vec<WorkflowState>,
    ) -> MiresidianResult<Vec<WorkflowState>> {
        Ok(states)
    }
}

#[test]
fn business_types_work_through_generated_node_and_codecs() {
    let value: miresidian_codec::NodeValue = NodeValue::Scalar(NodeScalar::Text("流程".into()));
    assert_eq!(value, NodeValue::Scalar(NodeScalar::Text("流程".into())));
    assert_eq!(Workflow::descriptor().name, "流程节点");
    assert_eq!(
        Workflow::normalize_config_value(0, "queued").unwrap(),
        "Queued"
    );

    let runtime = NodeRuntime::<Workflow>::new();
    runtime
        .invoke(&Invocation::new("InitDevice", vec!["排队".into()], vec![]))
        .unwrap();
    let output = runtime
        .invoke(&Invocation::new("当前状态", vec!["queued".into()], vec![]))
        .unwrap();
    assert_eq!(output.as_slice(), ["Queued"]);
    let output = runtime
        .invoke(&Invocation::new(
            "回传状态",
            vec!["初始状态:Queued".into()],
            vec!["排队|完成".into()],
        ))
        .unwrap();
    assert_eq!(output.as_slice(), ["Queued|Done"]);
    assert!(matches!(
        runtime.invoke(&Invocation::new(
            "回传状态",
            vec!["Queued".into()],
            vec!["未知".into()],
        )),
        Err(MiresidianError::InvalidArgument { .. })
    ));
}
