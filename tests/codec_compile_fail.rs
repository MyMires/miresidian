//! codec 的编译期契约与缺失 feature 诊断。
#[test]
fn codec_contract_diagnostics() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui-codec/*.rs");
    if !cfg!(feature = "codec") {
        cases.compile_fail("tests/ui-codec-feature/*.rs");
    }
}
