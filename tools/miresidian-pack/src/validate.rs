//! 项目结构、交付文件与已发布依赖的检查边界。
mod dependencies;
mod project;

pub(crate) use dependencies::validate_resolved_sources;
pub(crate) use project::{check_project, validate_node_name};
