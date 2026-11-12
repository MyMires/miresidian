//! 从业务 Rustdoc 生成受管 Markdown；保留区域外的部署说明。
mod source;
#[cfg(test)]
mod tests;

use crate::model::NodeRelease;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const START: &str = "<!-- miresidian:business-docs:start -->";
const END: &str = "<!-- miresidian:business-docs:end -->";

/// 构建前只读提取并检查受管区域；不展开宏，也不执行节点代码。
///
/// 返回本次源码参考。所选架构均构建成功后才允许写入，避免编译失败更新交付说明。
pub(crate) fn prepare(node: &NodeRelease) -> Result<String, String> {
    let methods = source::read_methods(&node.project_directory.join("src"))?;
    let mut section = format!(
        "## 业务方法文档（自动生成）\n\n节点版本：{}；框架版本：{}。\n\n\
         本节来自业务方法的 Rust 文档注释与声明。仅供源码参考：未展开宏或条件编译，\
         带条件的方法不一定在当前 DLL 启用；实际操作名、参数顺序及追加字段以该 DLL 的 \
         GetFunctions/GetInitConfig 查询为准。请在源码维护业务说明，本区域会在构建时更新。\n\n",
        node.project_version, node.miresidian_version,
    );
    if methods.is_empty() {
        section.push_str("未发现可静态识别的 operation 方法；宏生成或属性别名不会在此展开。\n\n");
    }
    for method in methods {
        section.push_str(&format!(
            "### {}\n\n源码：`{}`\n\n",
            method.name,
            method
                .path
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
        ));
        if !method.conditions.is_empty() {
            section.push_str("上层条件声明（未求值）：\n\n");
            section.push_str(&code_block(&method.conditions.join("\n")));
        }
        section.push_str(&code_block(&method.declaration));
        section.push_str(&nested_headings(&method.documentation));
        section.push_str("\n\n");
    }
    if section.contains(START) || section.contains(END) {
        return Err("业务文档注释不能包含 Pack 保留的 business-docs 区域标记".into());
    }
    // 在耗时编译之前检查原文；写入时仍重新读取，保留构建期间的手工编辑。
    merge(&read_guide(node)?, &section)?;
    Ok(section)
}

/// 编译成功后合并当前说明，并通过同目录临时文件替换，失败不截断原文。
pub(crate) fn write(node: &NodeRelease, section: &str) -> Result<(), String> {
    let path = guide_path(node);
    let original = read_guide(node)?;
    let updated = merge(&original, section)?;
    if updated == original {
        return Ok(());
    }
    let permissions = fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .permissions();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_extension(format!(
        "md.miresidian-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("无法创建文档临时文件 {}：{e}", temporary.display()))?;
    let result = (|| {
        file.write_all(updated.as_bytes())?;
        file.set_permissions(permissions)?;
        drop(file);
        fs::rename(&temporary, &path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("无法更新业务方法文档 {}：{error}", path.display()));
    }
    Ok(())
}

fn guide_path(node: &NodeRelease) -> PathBuf {
    node.project_directory
        .join("docs")
        .join(format!("{}.md", node.node_name))
}

fn read_guide(node: &NodeRelease) -> Result<String, String> {
    let path = guide_path(node);
    fs::read_to_string(&path).map_err(|e| format!("无法读取节点说明 {}：{e}", path.display()))
}

/// 首次追加，以后仅替换完整、唯一的标记对；异常区域不能猜测覆盖。
fn merge(guide: &str, section: &str) -> Result<String, String> {
    let starts = guide.match_indices(START).collect::<Vec<_>>();
    let ends = guide.match_indices(END).collect::<Vec<_>>();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let separator = if guide.ends_with("\n\n") {
                ""
            } else if guide.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            Ok(format!("{guide}{separator}{START}\n{section}{END}\n"))
        }
        ([(start, _)], [(end, _)]) if start < end => Ok(format!(
            "{}{START}\n{section}{END}{}",
            &guide[..*start],
            &guide[*end + END.len()..],
        )),
        _ => Err(
            "节点说明中的 business-docs 标记缺失、重复或顺序错误；原文未修改，请修复标记对后重试"
                .into(),
        ),
    }
}

/// 选择不会被声明中的反引号提前结束的 Markdown 代码围栏。
fn code_block(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.saturating_add(1).max(3));
    format!("{fence}rust\n{text}\n{fence}\n\n")
}

/// 使 Rustdoc 章节位于方法标题下；围栏内的示例和隐藏行保持原样。
fn nested_headings(documentation: &str) -> String {
    let mut fence: Option<(char, usize)> = None;
    documentation
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
            if let Some(marker) = marker {
                let length = trimmed.chars().take_while(|c| *c == marker).count();
                if length >= 3 {
                    match fence {
                        None => fence = Some((marker, length)),
                        Some((active, minimum))
                            if active == marker
                                && length >= minimum
                                && trimmed[length..].trim().is_empty() =>
                        {
                            fence = None
                        }
                        _ => {}
                    }
                    return line.to_owned();
                }
            }
            if fence.is_none() && line.len() - trimmed.len() <= 3 {
                let level = trimmed.bytes().take_while(|b| *b == b'#').count();
                if (1..=6).contains(&level) && trimmed.as_bytes().get(level) == Some(&b' ') {
                    return format!("{}{}", "#".repeat((level + 3).min(6)), &trimmed[level..]);
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
