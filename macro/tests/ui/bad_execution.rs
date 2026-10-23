//! `#[operation]` 声明未知执行方式关键字时必须在编译期失败。

use miresidian_macro::{node, node_impl, operation};

#[node("非法执行方式节点")]
#[derive(Default)]
struct BadExecutionNode;

#[node_impl]
impl BadExecutionNode {
    fn init_device(&mut self) -> Result<String, ()> {
        Ok(String::new())
    }

    fn close_device(&mut self) -> Result<(), ()> {
        Ok(())
    }

    #[operation("查询", concurrent)]
    fn query(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
