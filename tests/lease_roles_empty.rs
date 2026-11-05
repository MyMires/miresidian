#![cfg(feature = "lease")]

use miresidian::{DeviceNode, MiresidianResult};
mod key_only {
    use super::*;
    #[miresidian::node("无配置资源", lease(activation = required))]
    pub struct Only;
    impl Only {
        #[miresidian::operation]
        fn init_device(
            #[miresidian::config("资源", lease(key))] resource: String,
        ) -> MiresidianResult<Self> {
            let _ = resource;
            Ok(Self)
        }
        #[miresidian::operation]
        fn close_device(&mut self) -> MiresidianResult<()> {
            Ok(())
        }
    }
}
#[test]
fn key_only_lease_allows_empty_configuration() {
    let request = key_only::Only::lease_request(&["COM-test".into()])
        .unwrap()
        .unwrap();
    assert_eq!(request.resource().as_str(), "COM-test");
    assert!(request.configuration().as_bytes().is_empty());
}
