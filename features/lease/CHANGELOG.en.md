# Changelog

[中文](CHANGELOG.md) · [English](CHANGELOG.en.md)

## [0.17.3] - 2026-11-11

- Update the exact codec dependency to 0.18.0. Lease/shared-resource APIs and protocols v2 remain unchanged; synchronize candidate-version guidance.

## [0.17.2] - 2026-11-11

- Update the existing exact codec dependency to 0.17.0 for the framework public-type rename. Lease and shared-resource APIs and protocols v2 remain unchanged.
- Synchronize source-candidate dependency guidance without rewriting published-version documentation.

## [0.17.1] - 2026-11-06

- Enable all features for docs.rs and provide the documentation URL so optional local_ipc/shared APIs, final-close callbacks, and proxy methods receive exact-version online docs.
- Synchronize bilingual dependency/API/source guidance and complete the previously missing English 0.17.0 summary. Runtime behavior, default dependency features, and protocols are unchanged; verify registry and documentation results independently.

## [0.17.0] - 2026-11-06

- Add ResourceHandler::close and SharedResource::close_with to deliver the final session's close request and response, with disconnect reference cleanup and destruction before lease release even after errors.
- Add transact_with_timeout(None) without implicit business deadlines; LocalLease request deadlines remain two seconds.
- Advance resource protocol, endpoints, and owner-lock directories to v2; stop v1 resource participants before upgrading. Configuration lease v2 and C ABI v7 remain unchanged. Framework 0.25.0 integrates automatic node ownership without macro syntax changes.

## [0.16.0] - 2026-11-05

- Add ResourceKey::from_parts and LeaseRequest::from_parameter_keys for ordered composite resource identities and empty compatibility configurations.
- Preserve ordinary single-field keys and configuration bytes. Reserve the versioned composite prefix, rejecting singleton impersonation; separate value-object modules without changing IPC protocols or ownership.
- Validation: default workspace (107), all-feature workspace (219), two supplemental joint-role/FFI regressions, isolated instances (4), isolated lease (2), doctest, strict Clippy/Rustdoc, Rust 1.89 and Release build passed. The complete nine-file skill is synchronized. Linux simulations and in-process C ABI were tested; native Windows loading and real hardware were not tested.

## [0.15.3] - 2026-11-05

- Depend exactly on codec 0.16.0. Generated requests share typed canonical text with the runtime; wire protocols and ownership remain unchanged.

## [0.15.2] - 2026-11-05

- Depend exactly on codec 0.15.0; lease protocol v2, resource protocol v1, ownership and transaction semantics are unchanged.

## [0.15.1] - 2026-11-04

- Clarify `LocalLease` versus `SharedResource`, runtime Lease/Direct selector scope, and Windows/Unix module retention.
- Add Chinese/English package documentation and current release history; update root-crate examples to 0.20.1.
- Runtime code and public APIs are unchanged from 0.15.0. Configuration lease protocol v2 and resource proxy protocol v1 remain unchanged. Reuse existing ownership, cross-DLL/process, timeout, and failure validation; verify documentation and the registry package for this release.

## [0.15.0] - 2026-11-04

- Add device-independent shared resource owners and byte transaction proxies across DLLs/processes. Only the owner runs the factory and opens the connection; the last proxy releases it.
- Add bounded proxy framing, configuration/protocol checks, owner election locks, module retention, and initialization rollback. Sessions invalidate without automatic reconnect/replay after timeout, disconnect, or owner loss.
- Preserve the existing configuration lease v2 contract. Validate exclusive-resource concurrency, last close, independent processes, owner loss, factory failure, business error/panic, and request timeout. Native dynamic library validation uses exclusive files rather than hardware.

Earlier entries are maintained in the [Chinese changelog](CHANGELOG.md).
