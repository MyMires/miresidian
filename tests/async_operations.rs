//! Native asynchronous macro bridges work without the optional instances feature.
use miresidian::{Invocation, MiresidianResult, NodeRuntime};
use std::time::Duration;
#[miresidian::node("原生异步方法")]
struct AsyncNode {
    value: u32,
}
impl AsyncNode {
    #[miresidian::operation]
    async fn init_device(value: u32) -> MiresidianResult<Self> {
        tokio::time::sleep(Duration::from_millis(1)).await;
        Ok(Self { value })
    }
    #[miresidian::operation]
    async fn close_device(&mut self) -> MiresidianResult<()> {
        tokio::task::yield_now().await;
        Ok(())
    }
    #[miresidian::operation]
    async fn get_port_no() -> MiresidianResult<Option<String>> {
        tokio::task::yield_now().await;
        Ok(Some("async".into()))
    }
    #[miresidian::operation]
    #[miresidian::output("结果")]
    async fn add(&mut self, increment: u32) -> MiresidianResult<u32> {
        tokio::task::yield_now().await;
        self.value += increment;
        Ok(self.value)
    }
}
fn request(operation: &str, input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        vec!["1".into()],
        input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    )
}
#[test]
fn async_lifecycle_probe_and_business_use_generated_bridges() {
    let runtime = NodeRuntime::<AsyncNode>::new();
    assert_eq!(
        runtime
            .invoke(&request("GetPortNO", &[]))
            .unwrap()
            .as_slice(),
        ["1", "async"]
    );
    runtime.invoke(&request("InitDevice", &[])).unwrap();
    assert_eq!(
        runtime.invoke(&request("Add", &["2"])).unwrap().as_slice(),
        ["3"]
    );
    runtime.invoke(&request("CloseDevice", &[])).unwrap();
    assert!(!runtime.is_initialized());
}
#[test]
fn public_async_api_works_on_an_external_current_thread_executor() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let runtime = NodeRuntime::<AsyncNode>::new();
    executor.block_on(async {
        runtime
            .invoke_async(request("InitDevice", &[]))
            .await
            .unwrap();
        assert_eq!(
            runtime
                .invoke_async(request("Add", &["4"]))
                .await
                .unwrap()
                .as_slice(),
            ["5"]
        );
        runtime
            .invoke_async(request("CloseDevice", &[]))
            .await
            .unwrap();
    });
}
