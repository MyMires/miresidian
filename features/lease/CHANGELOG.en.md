# Changelog

[中文](CHANGELOG.md) · [English](CHANGELOG.en.md)

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
