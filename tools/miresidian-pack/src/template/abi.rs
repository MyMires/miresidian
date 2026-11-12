//! 按实际解析的 ABI 选择调用说明；不由框架版本推算函数签名。

pub(super) const PENDING: &str = "ABI 尚未解析，模板创建尚未完成；请先完成 Cargo 依赖解析和模板生成，再按所附 miresidian.h 调用。";

pub(super) fn contract(version: u32) -> Result<String, String> {
    let template = match version {
        7..=9 => include_str!("../../templates/abi-legacy.md.in"),
        10 => include_str!("../../templates/abi-v10.md.in"),
        _ => {
            return Err(format!(
                "模板创建尚未完成：不支持 ABI v{version}（支持 v7–v10）；未生成该 ABI 的调用签名，请使用支持它的工具版本"
            ));
        }
    };
    Ok(template.replace("{{abi_version}}", &version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 调用契约按实际_abi_选择且不猜测未知版本() {
        for version in 7..=10 {
            let text = contract(version).unwrap();
            assert!(text.contains(&format!("## C ABI v{version}")));
            assert!(!text.contains("{{abi_version}}"));
            if version == 10 {
                assert!(text.contains("miresidian_invoke(const char *request_json)"));
                assert!(text.contains("platform=0"));
                assert!(!text.contains("const char *function"));
            } else {
                assert!(text.contains("miresidian_invoke(const char *function"));
                assert!(!text.contains("platform=0"));
            }
        }
        for version in [0, 6, 11, u32::MAX] {
            assert!(contract(version).unwrap_err().contains("不支持 ABI"));
        }
    }
}
