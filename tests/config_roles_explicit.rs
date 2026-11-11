#![cfg(feature = "instances")]
#[path = "support/config_roles.rs"]
#[allow(dead_code)]
mod support;
support::fixture!(selected, [], [instance(key)], [instance(key)]);
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_composite_keys() {
    support::selected_keys::<selected::Node>().await;
}

#[test]
fn default_and_explicit_json_platforms_route_by_the_same_composite_key() {
    use std::ffi::{CStr, CString};
    let call = |explicit_adapter: bool, operation: &str, config: &[&str]| {
        let operation = CString::new(operation).unwrap();
        let config = CString::new(serde_json::to_vec(config).unwrap()).unwrap();
        // SAFETY: input strings remain alive during the call; copy the thread-local response immediately.
        let response = unsafe {
            let pointer = if explicit_adapter {
                ffi_support::request(
                    |request| selected::miresidian_invoke(request),
                    Some(1),
                    operation.as_ptr(),
                    config.as_ptr(),
                    c"[]".as_ptr(),
                )
            } else {
                ffi_support::request(
                    |request| selected::miresidian_invoke(request),
                    None,
                    operation.as_ptr(),
                    config.as_ptr(),
                    c"[]".as_ptr(),
                )
            };
            CStr::from_ptr(pointer).to_owned()
        };
        serde_json::from_slice::<serde_json::Value>(response.to_bytes()).unwrap()
    };
    for (explicit_adapter, port) in [(false, "8000"), (true, "9000")] {
        assert_eq!(
            call(explicit_adapter, "InitDevice", &["0", "ffi-host", port])["code"],
            "OK"
        );
        assert_eq!(
            call(!explicit_adapter, "Count", &["unused", "ffi-host", port])["output"],
            serde_json::json!([format!("ffi-host:{port}:0:1")])
        );
    }
    assert_eq!(
        call(true, "CloseDevice", &["unused", "ffi-host", "8000"])["code"],
        "OK"
    );
    assert_eq!(
        call(false, "Count", &["unused", "ffi-host", "8000"])["code"],
        "UNKNOWN_CHANNEL"
    );
    assert_eq!(call(false, "CloseAllDevices", &[])["code"], "OK");
}

#[path = "support/ffi_request.rs"]
mod ffi_support;
