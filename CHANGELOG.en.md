# Changelog

[中文](CHANGELOG.md) · [English](CHANGELOG.en.md)

## [0.20.1] - 2026-11-04

- Complete the Node skill and call documentation for fixed operations, parameter channels, defaults, optional values, collections, object reuse, initialization failures, asynchronous execution, and cancellation boundaries.
- Separate configuration leases, shared connections, and direct access. Document provider-owned factories, shared wire contracts, complete transactions, proxy references, and the scope of the runtime Lease/Direct selector.
- Add a runnable simulated provider, documentation navigation, version-pinned skill links, and Chinese/English release materials. Synchronize the complete installed skill.
- Bump the root documentation package to 0.20.1 and the lease documentation package to 0.15.1. Runtime code and public APIs remain those of 0.20.0/0.15.0. C ABI v7, configuration leases v2, and resource proxies v1 are unchanged. Other crates, tools, and independent nodes retain their versions.
- Reuse the preceding runtime validation; validate documentation, skill contents, examples, version gates, and registry package publication for this release. The uncommitted 0.19.5 documentation candidate is incorporated here without publishing that intermediate version.

## [0.20.0] - 2026-11-04

- Add `lease::shared::SharedResource` and communication-layer resource handlers. Compatible clients share one physical resource owner across DLLs/processes while keeping node state independent.
- Serialize complete resource transactions; validate configuration and transaction protocol compatibility. Close the connection after the last proxy, and invalidate old sessions after timeout, disconnect, handler panic, or owner loss without replaying device actions.
- Preserve Config/Handle object identity and reject Lease/Direct changes during reuse.
- Validation: 127 related all-feature tests passed with one existing ignored test; 23 selected default-feature tests passed. Native dynamic library loading used exclusive file handles; Windows x64/x86 compilation, strict Clippy, Rust 1.89, and strict Rustdoc checks passed. No real serial communication was established by these simulations.

Earlier entries are maintained in the [Chinese changelog](CHANGELOG.md).
