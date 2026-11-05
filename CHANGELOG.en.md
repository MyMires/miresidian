# Changelog

[中文](CHANGELOG.md) · [English](CHANGELOG.en.md)

## [0.23.0] - 2026-11-05

- Add type-driven canonical configuration callbacks shared by instance comparison and lease encoding. Numeric blanks and zero are equivalent, while blank identities and nonkey routing retain their boundaries.
- Map options after recognizing complete escaped collection elements; preserve literal pipe contents.
- Require empty default fields for paths, open text and continuous numeric declarations in the synchronized Node skill and documentation. Scientific notation is actual input, not a default/candidate description.
- Root 0.23.0, codec 0.16.0, macro 0.21.0; patch dependent adapters only. Tools, C ABI v7, lease v2, resource v1 and MSRV 1.89 remain unchanged.

## [0.22.0] - 2026-11-05

- Stop deriving business defaults from descriptions and selecting the first item of complete choice strings. Explicit blank numbers become zero; missing arguments fail. Preserve display-label mapping and explicit framework selector defaults.
- Decode GBK strictly without BOM detection or replacement. Reject nonfinite formatted float scalars, vectors and optional outputs.
- Return UNKNOWN_CHANNEL for missing Handles before initialization and after the last object closes. Complete Pack component/source validation and synchronize the toolset, migration guide and Node skill.
- Root 0.22.0, codec 0.15.0, macro 0.20.0; patch affected adapters and tools independently. C ABI v7, lease v2, resource v1 and MSRV 1.89 remain unchanged. Publication is separate.

## [0.21.0] - 2026-11-05

- Parse plain batch inputs using the original Rust argument's codec traits. Strings, booleans, characters, aliases, and compatible custom elements no longer require format. Preserve integer single/list/range/mixed syntax and float validation.
- Keep batch and format crates independent. Macro composition aligns data and input/output format columns with singleton broadcast or equal length. Fixed single-format annotations generate no selector or additional input slot.
- Add decode_batch/batch_records and select_format_value APIs; retain existing formatted-record helpers. Preserve pre-execution input validation, sequential invocation, stop-on-error, output grouping, and sync/native async dispatch.
- Update implementation comments, feature guides, ABI call documentation, examples, and the complete installed Node skill. Root 0.21.0, macro 0.19.0, batch 0.2.0, and format 0.3.0; unrelated crates, tools, C ABI v7, configuration leases v2, and resource proxies v1 are unchanged. Publication is a separate action.

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
