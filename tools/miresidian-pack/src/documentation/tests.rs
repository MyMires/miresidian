//! 针对源码到交付说明的内容、边界和覆盖保护回归。
use super::*;
use crate::test_support::TempDir;

fn source_project() -> (TempDir, NodeRelease) {
    let temp = TempDir::new();
    let node = temp.node();
    fs::create_dir(temp.0.join("src")).unwrap();
    (temp, node)
}

#[test]
fn 多文件与私有方法保留中文文档和声明但不暴露实现() {
    let (temp, node) = source_project();
    fs::write(
        temp.0.join("src/z.rs"),
        r#"
        impl Node {
            /// 发送数据，等待完整响应。
            ///
            /// # 参数
            /// `data`：字节数组。
            ///
            /// # 错误
            /// 超时返回错误；不自动重试。
            /// ```rust,ignore
            /// # let hidden = 1;
            /// let data = vec![1, 2];
            /// ```
            #[miresidian::operation("查询")]
            #[miresidian::output("响应")]
            fn query(&mut self, #[miresidian::input("数据")] data: Vec<u8>) -> Result<String, E> {
                secret_protocol_implementation()
            }
            /// 私有内部实现不应导出。
            fn helper() {}
        }
    "#,
    )
    .unwrap();
    fs::write(
        temp.0.join("src/a.rs"),
        r#"
        #[cfg(feature = "transport")]
        mod inner {
            #[cfg(windows)]
            impl Node {
                #[doc = "取得临时探测结果。"]
                #[cfg(target_pointer_width = "64")]
                #[operation]
                fn get_port_no() -> Option<String> { None }
            }
        }
    "#,
    )
    .unwrap();
    let text = prepare(&node).unwrap();
    assert!(text.find("inner::Node::get_port_no").unwrap() < text.find("Node::query").unwrap());
    for expected in [
        "发送数据，等待完整响应。",
        "#### 参数",
        "#### 错误",
        "# let hidden = 1;",
        "```rust,ignore",
        "取得临时探测结果。",
        "transport",
        "windows",
        "target_pointer_width",
        "src/z.rs",
        "响应",
        "数据",
        "Result < String , E >",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert!(!text.contains("helper"));
    assert!(!text.contains("secret_protocol_implementation"));
    assert!(!text.contains(&temp.0.to_string_lossy().to_string()));
}

#[test]
fn 缺少注释与动态表达式显式说明而不编造内容() {
    let (temp, node) = source_project();
    fs::write(
        temp.0.join("src/lib.rs"),
        r#"
        impl Node {
            #[operation] fn absent(&mut self) {}
            #[doc = include_str!("more.md")]
            #[operation] fn dynamic(&mut self) {}
            /** 块文档说明。 */
            #[operation] fn block(&mut self) {}
            #[other::operation] fn unrelated(&mut self) {}
        }
    "#,
    )
    .unwrap();
    let text = prepare(&node).unwrap();
    assert!(text.contains("此方法尚未提供文档注释"));
    assert!(text.contains("未展开的动态文档表达式"));
    assert!(text.contains("块文档说明"));
    assert!(!text.contains("unrelated"));
}

#[test]
fn 重复生成幂等且跟随源码更新删除并保留构建期间的手写内容() {
    let (temp, node) = source_project();
    let source = temp.0.join("src/lib.rs");
    fs::write(
        &source,
        "impl Node { /// 原有说明\n #[operation] fn original(&mut self) {} }",
    )
    .unwrap();
    let generated = prepare(&node).unwrap();
    assert_eq!(read_guide(&node).unwrap(), "设备调用说明");
    fs::write(guide_path(&node), "构建时补充的部署说明\r\n").unwrap();
    write(&node, &generated).unwrap();
    let initial = read_guide(&node).unwrap();
    write(&node, &generated).unwrap();
    assert_eq!(read_guide(&node).unwrap(), initial);
    let manual = format!("前言\n{initial}\n## 手写接线说明\n不要删除。\n");
    fs::write(guide_path(&node), &manual).unwrap();
    fs::write(
        &source,
        "impl Node { /// 新说明\n #[operation] fn replacement(&mut self) {} }",
    )
    .unwrap();
    write(&node, &prepare(&node).unwrap()).unwrap();
    let updated = read_guide(&node).unwrap();
    assert!(updated.starts_with("前言\n构建时补充的部署说明\r\n"));
    assert!(updated.ends_with("\n## 手写接线说明\n不要删除。\n"));
    assert!(updated.contains("新说明"));
    assert!(!updated.contains("原有说明"));
    assert!(!updated.contains("Node::original"));
    fs::write(&source, "impl Node { fn helper() {} }").unwrap();
    write(&node, &prepare(&node).unwrap()).unwrap();
    assert!(!read_guide(&node).unwrap().contains("Node::replacement"));
}

#[test]
fn 非法区域在准备与写入阶段都拒绝且保持原文() {
    let (temp, node) = source_project();
    fs::write(temp.0.join("src/lib.rs"), "").unwrap();
    for text in [
        START.to_owned(),
        END.to_owned(),
        format!("{END}\n{START}"),
        format!("{START}\n{START}\n{END}"),
        format!("{START}\n{END}\n{END}"),
    ] {
        fs::write(guide_path(&node), &text).unwrap();
        assert!(prepare(&node).is_err());
        assert!(write(&node, "新生成内容\n").is_err());
        assert_eq!(read_guide(&node).unwrap(), text);
    }
    fs::write(guide_path(&node), "手写说明").unwrap();
    fs::write(
        temp.0.join("src/lib.rs"),
        format!("impl Node {{ #[doc = {START:?}] #[operation] fn read(&mut self) {{}} }}"),
    )
    .unwrap();
    assert!(prepare(&node).is_err());
    assert_eq!(read_guide(&node).unwrap(), "手写说明");
}

#[test]
fn 解析失败不修改原文并报告源码位置() {
    let (temp, node) = source_project();
    fs::write(temp.0.join("src/lib.rs"), "impl {").unwrap();
    let error = prepare(&node).unwrap_err();
    assert!(error.contains("lib.rs"));
    assert_eq!(read_guide(&node).unwrap(), "设备调用说明");
}

#[test]
fn 文档代码围栏与标题层级互不干扰() {
    assert_eq!(
        nested_headings("# 标题\n~~~rust\n# 示例隐藏行\n~~~\n## 错误"),
        "#### 标题\n~~~rust\n# 示例隐藏行\n~~~\n##### 错误"
    );
    assert!(code_block("#[operation(\"```\")] fn read();").starts_with("````rust\n"));
}
