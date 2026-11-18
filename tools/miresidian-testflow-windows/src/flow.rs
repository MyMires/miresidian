//! 生成前的严格语法检查；DLL 身份、参数默认值及执行结果由运行时判断。
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const MAX_CHANNELS: usize = 256;
type Channels = BTreeMap<usize, Option<Vec<String>>>;

/// 不把 JSON 对象先变为 map：先检查解码后的键，避免重复键被覆盖。
fn json(text: &str) -> Result<Value, String> {
    let value = serde_json::from_str(text).map_err(|e| format!("无效 JSON：{e}"))?;
    let bytes = text.as_bytes();
    let mut objects: Vec<Option<HashSet<String>>> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => objects.push(Some(HashSet::new())),
            b'[' => objects.push(None),
            b'}' | b']' => {
                objects.pop();
            }
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
                let mut next = i + 1;
                while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                    next += 1;
                }
                if bytes.get(next) == Some(&b':') {
                    let key: String =
                        serde_json::from_str(&text[start..=i]).map_err(|e| e.to_string())?;
                    if let Some(Some(keys)) = objects.last_mut()
                        && !keys.insert(key.clone())
                    {
                        return Err(format!("重复 JSON 键：{key}"));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(value)
}

fn channel(text: &str) -> Result<usize, String> {
    let n = text.strip_prefix("ch").ok_or("通道须以 ch 开头")?;
    if n.is_empty() || n.starts_with('0') || !n.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("无效通道：{text}"));
    }
    let n = n.parse::<usize>().map_err(|_| "通道超出范围")?;
    if n > MAX_CHANNELS {
        return Err("最多支持 256 通道".into());
    }
    Ok(n)
}

fn selector(text: &str) -> Result<Vec<usize>, String> {
    if let Some((first, last)) = text.split_once('-') {
        let first = channel(first)?;
        let last = channel(last)?;
        if first > last {
            return Err("通道范围必须升序".into());
        }
        Ok((first..=last).collect())
    } else {
        text.split(',').map(channel).collect()
    }
}

fn strings(value: &Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or("参数必须为字符串数组")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "参数必须全部为字符串".into())
        })
        .collect()
}

fn channels(value: &Value, config: bool) -> Result<Channels, String> {
    let object = value.as_object().ok_or("通道参数必须为对象")?;
    if object.is_empty() {
        return Err("通道对象不能为空".into());
    }
    let mut result = BTreeMap::new();
    for (key, value) in object {
        let args = if value.is_null() && !config {
            None
        } else {
            Some(strings(value)?)
        };
        for n in selector(key)? {
            if result.insert(n, args.clone()).is_some() {
                return Err(format!("重叠通道：ch{n}"));
            }
        }
    }
    Ok(result)
}

/// 按完整括号和字符串扫描，支持跨行；注释仅允许独占行。
fn statements(text: &str) -> Result<Vec<(usize, usize, String)>, String> {
    let mut result = Vec::new();
    let mut buffer = String::new();
    let mut stack = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let trimmed = line.trim();
        if !quoted && (trimmed.starts_with('#') || trimmed.starts_with("//")) {
            continue;
        }
        if buffer.is_empty() && trimmed.is_empty() {
            continue;
        }
        if buffer.is_empty() {
            start = index + 1;
        }
        for c in line.chars() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    quoted = false;
                }
            } else {
                match c {
                    '"' => quoted = true,
                    '(' | '[' | '{' => stack.push(c),
                    ')' | ']' | '}' => {
                        let expected = match c {
                            ')' => '(',
                            ']' => '[',
                            _ => '{',
                        };
                        if stack.pop() != Some(expected) {
                            return Err(format!("流程第 {} 行括号不匹配", index + 1));
                        }
                    }
                    _ => {}
                }
            }
        }
        buffer.push_str(line);
        buffer.push('\n');
        if !quoted && stack.is_empty() {
            result.push((start, index + 1, std::mem::take(&mut buffer)));
        }
    }
    if !buffer.is_empty() {
        return Err(format!("流程第 {start} 行语句未结束"));
    }
    Ok(result)
}

fn expectation(value: &Value) -> Result<(), String> {
    let object = value.as_object().ok_or("=> 后必须为预期对象")?;
    if !object.contains_key("status") && !object.contains_key("code") {
        return Err("预期必须至少包含 status 或 code".into());
    }
    for (key, value) in object {
        match key.as_str() {
            "status" if value.as_i64().is_some() => {}
            "code" if value.is_string() => {}
            "output" if value.is_array() => {
                strings(value)?;
            }
            "output" if value.is_object() => {
                let mut used = HashSet::new();
                if value.as_object().unwrap().is_empty() {
                    return Err("预期通道不能为空".into());
                }
                for (key, v) in value.as_object().unwrap() {
                    if !v.is_string() {
                        strings(v)?;
                    }
                    for n in selector(key)? {
                        if !used.insert(n) {
                            return Err("预期通道重叠".into());
                        }
                    }
                }
            }
            _ => return Err(format!("无效预期字段或类型：{key}")),
        }
    }
    Ok(())
}

pub(crate) fn validate(text: &str) -> Result<(), String> {
    let statements = statements(text)?;
    if statements.is_empty() {
        return Err("流程至少需要一个方法调用".into());
    }
    let mut config: Option<Channels> = None;
    let mut init_requested = false;
    for (line, end_line, statement) in statements {
        let check = || -> Result<bool, String> {
            let open = statement.find('(').ok_or("缺少方法调用括号")?;
            let name = statement[..open].trim();
            if name.is_empty()
                || name
                    .chars()
                    .any(|c| c.is_whitespace() || "(){}[]".contains(c))
            {
                return Err("无效方法名".into());
            }
            let mut quote = false;
            let mut escape = false;
            let mut depth = 1;
            let mut close = None;
            for (offset, c) in statement[open + 1..].char_indices() {
                if quote {
                    if escape {
                        escape = false;
                    } else if c == '\\' {
                        escape = true;
                    } else if c == '"' {
                        quote = false;
                    }
                } else {
                    match c {
                        '"' => quote = true,
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(open + 1 + offset);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let close = close.ok_or("缺少结束括号")?;
            let tail = statement[close + 1..].trim();
            if !tail.is_empty() {
                let expected = tail.strip_prefix("=>").ok_or("调用后只允许 => 预期对象")?;
                if name.eq_ignore_ascii_case("Config") || name.eq_ignore_ascii_case("Delay") {
                    return Err("Config/Delay 不是 DLL 断言目标".into());
                }
                expectation(&json(expected)?)?;
            }
            let arguments = json(&format!("[{}]", &statement[open + 1..close]))?;
            let values = arguments.as_array().unwrap();
            let mapped = values.len() == 1 && values[0].is_object();
            let is_config = name.eq_ignore_ascii_case("Config");
            if mapped
                && matches!(
                    name,
                    "GetFunctions"
                        | "GetFrameworkVersion"
                        | "GetProjectVersion"
                        | "GetInitConfig"
                        | "GetPortNO"
                        | "CloseAllDevices"
                )
            {
                return Err("全局方法不接受通道映射".into());
            }
            let args = if mapped {
                channels(&values[0], is_config)?
            } else {
                BTreeMap::from([(1, Some(strings(&arguments)?))])
            };
            if name.eq_ignore_ascii_case("Delay") {
                if mapped {
                    return Err("Delay 不支持通道".into());
                }
                let a = args[&1].as_ref().ok_or("Delay 不支持通道")?;
                if a.len() != 1
                    || a[0].is_empty()
                    || !a[0].bytes().all(|b| b.is_ascii_digit())
                    || a[0].parse::<i32>().is_err()
                {
                    return Err("Delay 必须接收 0..2147483647 毫秒字符串".into());
                }
            }
            Ok(args.values().any(Option::is_some))
        };
        let active = check().map_err(|e| format!("流程第 {line}-{end_line} 行：{e}"))?;
        // 静态检查不将 Init 成功或失败预期当成已成功初始化；留给 DLL 实际结果。
        let open = statement.find('(').unwrap();
        let name = statement[..open].trim();
        if name == "InitDevice" {
            init_requested = true;
        }
        if name == "CloseAllDevices" {
            init_requested = false;
        }
        if name.eq_ignore_ascii_case("Config") {
            let close = statement.rfind(')').unwrap();
            let values = json(&format!("[{}]", &statement[open + 1..close]))?;
            let shared = !(values.as_array().unwrap().len() == 1 && values[0].is_object());
            config = Some(if shared {
                BTreeMap::from([(1, Some(strings(&values)?))])
            } else {
                channels(&values[0], true)?
            });
        } else if !name.eq_ignore_ascii_case("Delay")
            && !matches!(
                name,
                "GetFunctions"
                    | "GetFrameworkVersion"
                    | "GetProjectVersion"
                    | "GetInitConfig"
                    | "GetPortNO"
                    | "CloseAllDevices"
            )
        {
            let selected = config
                .as_ref()
                .ok_or_else(|| format!("流程第 {line}-{end_line} 行：调用前必须先 Config(...)"))?;
            if selected.is_empty() {
                return Err("缺少通道配置".into());
            }
            if active && !init_requested && name != "CloseDevice" {
                return Err(format!(
                    "流程第 {line}-{end_line} 行：业务调用前必须先请求 InitDevice"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selectors_are_bounded_and_disjoint() {
        for key in [
            "ch0",
            "ch01",
            "ch257",
            "ch2-ch1",
            "ch1,ch2-ch3",
            "ch1, ch2",
            "ch9999999999999999999999",
        ] {
            assert!(selector(key).is_err(), "{key}");
        }
        assert_eq!(selector("ch1-ch3").unwrap(), vec![1, 2, 3]);
        assert!(channels(&json(r#"{"ch1-ch3":[],"ch2":[]}"#).unwrap(), false).is_err());
    }
    #[test]
    fn duplicate_decoded_keys_are_rejected() {
        for text in [
            r#"{"ch1":[],"ch1":[]}"#,
            r#"{"ch1":[],"\u0063h1":[]}"#,
            r#"{"output":{"ch1":"a","ch1":"b"}}"#,
        ] {
            assert!(json(text).is_err());
        }
        assert!(json(r#"[{"x":1},{"x":2}]"#).is_ok());
    }
    #[test]
    fn multiline_calls_and_expectations() {
        validate("Config()\nEcho({\"ch1\":null})").unwrap();
        validate("Config()\nInitDevice()\n发送数据({\n\"ch1,ch3\": [\"AA\", \"HEX\"],\n\"ch6\":null\n}) => {\n\"code\":\"OK\",\"output\":{\"ch1,ch3\":[\"a\",\"b\"]}\n}\nEcho(\"=>)\\\"\")").unwrap();
        for text in [
            "# empty",
            "Config({})",
            "Config({\"ch1\":null})",
            "Config() => {\"code\":\"OK\"}",
            "Delay(\"-1\")",
            "Delay({\"ch2\":[]})",
            "GetFunctions({\"ch1\":[]})",
            "Config()\nEcho() => {\"output\":[]}",
            "Config()\nEcho() => {\"code\":\"OK\",\"extra\":1}",
            "Config()\nEcho(\"a\"])",
            "Config()\nEcho(1)",
        ] {
            assert!(validate(text).is_err(), "{text}");
        }
    }
}
