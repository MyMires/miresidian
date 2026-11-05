# Miresidian Lease

[中文](README.md) · [English](README.en.md)

`miresidian_lease` 0.15.3 provides device-independent configuration leases and shared physical resource proxies. Rust MSRV is **1.89**. The configuration lease protocol remains **v2**; resource proxies use a separate **v1** protocol.

Node projects normally enable the capability through the root crate:

```toml
[dependencies]
miresidian = { version = "=0.23.0", features = ["lease"] }
```

Standalone communication providers can depend on `miresidian_lease = { version = "=0.15.3", features = ["local-ipc"] }`.

## Choose the appropriate mechanism

| Mechanism | Responsibility |
| --- | --- |
| Runtime `lease(...)` declaration / `LocalLease` | Resource configuration compatibility and session validation |
| `shared::SharedResource` / `ResourceHandler` | One physical connection, complete transaction serialization, and proxy reference lifetime |
| Direct communication | Provider-owned connection without a claim of shared-resource coordination |

Compatible leases do not share device handles. A provider must explicitly acquire a shared proxy and open the device inside the factory passed to `SharedResource::acquire`. Only the resource owner executes that factory. Other clients attach to it using an identical resource key, configuration encoding, and transaction protocol identifier. Node project names, DLL filenames, and Handles are not physical resource identities.

Put the complete send/wait/receive exchange in one `transact` call. Separate calls do not preserve a transaction lock between them. Each node keeps its own business state and proxy. Closing one proxy preserves other clients; the last proxy releases the physical connection before the owner's configuration lease.

The runtime Lease/Direct selector changes the runtime configuration lease only. It does not rewrite an already chosen communication provider. A shared-only provider generally does not need a duplicate runtime lease declaration. If both mechanisms are used, their resource keys and configuration bytes must match exactly, including the macro encoding's field names and ordering.

## Failure and deployment boundaries

Sent-request timeouts and disconnects invalidate that proxy without automatic reconnection or transaction replay. A submitted device action may still be running. Handler panic faults the owner; close old proxies before reinitializing. Device I/O requires its own timeouts. Proxy calls are synchronous and need an appropriate blocking bridge when used from native async business methods.

Configuration IPC requests have a two-second deadline; configuration lock acquisition and proxy handshake waits have five-second limits. Each proxy request/response is bounded to 16 MiB. Participants must share the required user environment and temporary directory. Do not delete or replace live configuration or owner-lock files. The service module is retained until process exit on Windows/Unix.

Local IPC user scoping is not deployment authentication. Preserve appropriate pipe/socket and directory access controls. Existing node binaries require provider integration and rebuilding.

See the [provider contract and runnable example](../../docs/shared-resources.md), [runtime integration](README.md), and [changelog](CHANGELOG.en.md). Cross-process and native dynamic-library simulations validate ownership and serialization, not real device communication.
