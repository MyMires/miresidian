//! 四项 feature 简写组合；内存回显示例，不访问真实设备。
//!
//! `cargo build --example macro_shortcuts --all-features --locked` 编译为动态库。
//! 新语法仍逐方法、逐参数声明；资源键与实例键各自显式标记。

use miresidian::MiresidianResult;

#[miresidian::node(
    "宏简写回显",
    instances(multiple, config, selectable(sequential)),
    lease(selectable(required))
)]
struct EchoNode {
    calls: usize,
}

impl EchoNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("对象:demo", instance(key), lease(key))] name: String,
    ) -> MiresidianResult<Self> {
        let _ = name;
        Ok(Self { calls: 0 })
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("回显")]
    #[miresidian::output("数据", format(utf8, hex, base64, selector = "输出格式"))]
    #[miresidian::output("次数")]
    async fn echo(
        &mut self,
        #[miresidian::input("数据", batch, format(utf8, hex, base64, selector = "输入格式"))]
        data: String,
    ) -> MiresidianResult<(String, usize)> {
        self.calls += 1;
        Ok((data, self.calls))
    }
}
