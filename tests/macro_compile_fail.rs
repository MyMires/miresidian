//! 属性宏的编译失败回归测试。
//!
//! 这些用例锁定面向驱动作者的诊断信息，避免将来重构时退化为难以理解的 Rust 内部错误。

#[test]
fn 非法节点声明会在编译期失败() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    cases.pass("tests/ui-pass/*.rs");
    if cfg!(feature = "instances") {
        cases.compile_fail("tests/ui-instances-fail/*.rs");
        cases.pass("tests/ui-instances-pass/*.rs");
    } else {
        cases.compile_fail("tests/ui-instances-feature/*.rs");
    }
}
