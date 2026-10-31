//! 在生成 BAT 前检查流程语法与基本生命周期；方法签名及配置默认值在 DLL 运行时验证。
use std::collections::HashSet;

pub(crate) fn validate(text: &str) -> Result<(), String> {
    let mut selected: Option<String> = None;
    let mut initialized = HashSet::new();
    let mut calls = 0;
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let error = |message: &str| format!("流程第 {} 行：{message}", index + 1);
        let (name, arguments) = line
            .split_once('(')
            .ok_or_else(|| error("应为 方法名(\"参数\")"))?;
        let name = name.trim();
        if name.is_empty()
            || name.chars().any(|c| c.is_whitespace() || c == ')')
            || !arguments.ends_with(')')
        {
            return Err(error("无效的方法调用"));
        }
        let values: Vec<String> =
            serde_json::from_str(&format!("[{}]", &arguments[..arguments.len() - 1]))
                .map_err(|_| error("参数必须全部为 JSON 字符串"))?;
        calls += 1;
        match name {
            name if name.eq_ignore_ascii_case("Config") => {
                selected = Some(serde_json::to_string(&values).map_err(|e| e.to_string())?);
            }
            name if name.eq_ignore_ascii_case("Delay") => {
                if values.len() != 1
                    || values[0].is_empty()
                    || !values[0].bytes().all(|c| c.is_ascii_digit())
                    || values[0].parse::<i32>().is_err()
                {
                    return Err(error("Delay 必须接收一个 0..2147483647 范围的毫秒数字符串"));
                }
            }
            "GetFunctions"
            | "GetFrameworkVersion"
            | "GetProjectVersion"
            | "GetInitConfig"
            | "GetPortNO" => {}
            "InitDevice" => {
                initialized.insert(
                    selected
                        .clone()
                        .ok_or_else(|| error("InitDevice 前必须先 Config(...)"))?,
                );
            }
            "CloseDevice" => {
                let key = selected
                    .as_ref()
                    .ok_or_else(|| error("CloseDevice 前必须先 Config(...)"))?;
                initialized.remove(key);
            }
            "CloseAllDevices" => {
                initialized.clear();
            }
            _ => {
                let key = selected
                    .as_ref()
                    .ok_or_else(|| error("业务调用前必须先 Config(...)"))?;
                if !initialized.contains(key) {
                    return Err(error("所选 Config 尚未 InitDevice"));
                }
            }
        }
    }
    if calls == 0 {
        return Err("流程至少需要一个方法调用".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 验证参数与配置生命周期() {
        validate(
            "GetFunctions()\nConfig()\nInitDevice()\nEcho(\"hello\")\nDelay(\"1\")\nCloseDevice()",
        )
        .unwrap();
        validate("Config(\"a\")\nInitDevice()\nConfig(\"b\")\nInitDevice()\nConfig(\"a\")\nEcho(\"a,b)\")").unwrap();
        for text in [
            "InitDevice()",
            "Config()\nEcho(\"x\")",
            "Echo(未加引号)",
            "Delay(\"-1\")",
            "Delay(\"2147483648\")",
            "# 空流程",
            "Config()\nInitDevice()\nCloseAllDevices()\nEcho(\"x\")",
        ] {
            assert!(validate(text).is_err(), "{text}");
        }
    }
}
