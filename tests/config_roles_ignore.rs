#![cfg(feature = "instances")]
#[path = "support/config_roles.rs"]
#[allow(dead_code)]
mod support;
support::fixture!(selected, [instance(ignore)], [], []);
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_composite_keys() {
    support::selected_keys::<selected::Node>().await;
}
