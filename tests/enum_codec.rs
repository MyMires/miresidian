//! 派生 codec 本身只接收实际值；平台格式由适配器配套处理。
use miresidian::{
    EnumOption, MiresidianError, NodeCollectionElement, NodeInput, NodeOutput, ParameterDescriptor,
    PlatformAdapter, TbLvTestPlatformAdapter,
};

#[allow(non_camel_case_types)]
#[derive(miresidian::NodeEnum)]
enum Keyword {
    r#type,
    Other,
}

#[test]
fn raw_identifiers_and_wrappers_share_candidates_without_clone_or_copy() {
    assert_eq!(
        Keyword::parse_text("type", "值")
            .unwrap()
            .into_output()
            .unwrap()
            .as_slice(),
        ["type"]
    );
    assert_eq!(Keyword::normalize_text(" Other ", "值").unwrap(), "Other");
    assert_eq!(
        <Option<Keyword> as NodeInput>::OPTIONS,
        <Keyword as NodeInput>::OPTIONS
    );
    assert_eq!(
        <Vec<Keyword> as NodeInput>::OPTIONS,
        <Keyword as NodeInput>::OPTIONS
    );
    assert!(matches!(
        Keyword::parse_text("type/Other", "值"),
        Err(MiresidianError::InvalidArgument { .. })
    ));
    assert_eq!(
        Keyword::format_collection(&Keyword::parse_collection("type|Other", "值").unwrap())
            .unwrap(),
        "type|Other"
    );
}

fn parameter(options: &'static [EnumOption]) -> ParameterDescriptor {
    ParameterDescriptor {
        name: "状态",
        definition: "状态:",
        default_value: None,
        enum_options: options,
    }
}

#[test]
fn tb_validates_only_generated_candidates_and_preserves_explicit_descriptions() {
    let adapter = TbLvTestPlatformAdapter;
    let p = parameter(&[
        EnumOption {
            value: "Idle",
            display: "空闲",
        },
        EnumOption {
            value: "Run",
            display: "运行",
        },
    ]);
    assert_eq!(
        adapter.format_parameter(&p).unwrap(),
        "状态:Idle-空闲/Run-运行"
    );
    assert_eq!(
        adapter.normalize_parameter(&p, "状态:空闲|运行").unwrap(),
        "Idle|Run"
    );
    for options in [
        &[EnumOption {
            value: "A/B",
            display: "label",
        }][..],
        &[EnumOption {
            value: "A",
            display: " ",
        }][..],
        &[
            EnumOption {
                value: "A",
                display: "B",
            },
            EnumOption {
                value: "B",
                display: "C",
            },
        ][..],
    ] {
        assert!(adapter.format_parameter(&parameter(options)).is_err());
    }
    let explicit = ParameterDescriptor {
        definition: "状态:手写/候选",
        ..p
    };
    assert_eq!(
        adapter.format_parameter(&explicit).unwrap(),
        "状态:手写/候选"
    );
    let default = ParameterDescriptor {
        definition: "状态:Idle",
        ..p
    };
    assert_eq!(adapter.format_parameter(&default).unwrap(), "状态:Idle");
}
