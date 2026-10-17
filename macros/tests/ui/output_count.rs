#![allow(unused_imports)]

use miresidian_macros::{node_impl, operation, output};

struct InvalidOutput;

#[node_impl]
impl InvalidOutput {
    fn init_device(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn close_device(&mut self) -> Result<(), ()> {
        Ok(())
    }

    #[operation]
    #[output("结果")]
    fn query(&mut self) -> Result<(String, String), ()> {
        Ok((String::new(), String::new()))
    }
}

fn main() {}
