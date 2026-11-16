//! format 的编译期契约与缺失 feature 诊断。
#[test]
fn codec_contract_diagnostics() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui-encoding/*.rs");
    if !cfg!(feature = "encoding") {
        cases.compile_fail("tests/ui-encoding-feature/*.rs");
    } else {
        cases.pass("tests/ui-encoding-feature/*.rs");
    }
}
