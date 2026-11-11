#![cfg(feature = "instance")]
#[path = "support/config_roles.rs"]
#[allow(dead_code)]
mod support;
support::fixture!(all, [], [], []);
#[test]
fn default_all_keys() {
    support::default_keys::<all::Node>();
}
