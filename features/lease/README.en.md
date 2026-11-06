# Miresidian Lease

[中文](README.md) · [English](README.en.md)

`miresidian_lease` 0.17.1 provides device-independent configuration leases and shared physical resource proxies. Rust MSRV is **1.89**. The configuration lease protocol remains **v2**; resource proxies use a separate **v2** protocol.

Node projects normally enable the capability through the root crate:

```toml
[dependencies]
miresidian = { version = "=0.25.1", features = ["lease"] }
```

Standalone communication providers can depend on `miresidian_lease = { version = "=0.17.1", features = ["local-ipc"] }`.

## Choose the appropriate mechanism

| Mechanism | Responsibility |
| --- | --- |
| Runtime `lease(...)` declaration | Automatic ownership of one shared node object and its state |
| `LocalLease` | Resource configuration compatibility and session validation |
| `shared::SharedResource` / `ResourceHandler` | One physical connection, complete transaction serialization, and proxy reference lifetime |
| Direct communication | Provider-owned connection without a claim of shared-resource coordination |

LocalLease used alone does not share device handles. Automatic runtime leases share the whole node object. For independent business objects, a provider must explicitly acquire a shared proxy and open the device inside the factory passed to `SharedResource::acquire`. Only the resource owner executes that factory. Other clients attach to it using an identical resource key, configuration encoding, and transaction protocol identifier. Node project names, DLL filenames, and Handles are not physical resource identities.

Put the complete send/wait/receive exchange in one `transact` call. Separate calls do not preserve a transaction lock between them. Each node keeps its own business state and proxy. Closing one proxy preserves other clients; the last proxy releases the physical connection before the owner's configuration lease.

The runtime Lease/Direct selector chooses automatic node ownership or local construction. It does not rewrite explicit communication providers. Providers sharing one connection among independent business objects should own their SharedResource without a duplicate resource-level node lease; nesting the two protocols on the same resource is rejected.

## Failure and deployment boundaries

Sent-request timeouts and disconnects invalidate that proxy without automatic reconnection or transaction replay. A submitted device action may still be running. Handler panic faults the owner; close old proxies before reinitializing. Device I/O requires its own timeouts. Proxy calls are synchronous and need an appropriate blocking bridge when used from native async business methods.

Configuration IPC requests have a two-second deadline; configuration lock acquisition and proxy handshake waits have five-second limits. Each proxy request/response is bounded to 16 MiB. Participants must share the required user environment and temporary directory. Do not delete or replace live configuration or owner-lock files. The service module is retained until process exit on Windows/Unix.

Local IPC user scoping is not deployment authentication. Preserve appropriate pipe/socket and directory access controls. Existing node binaries require provider integration and rebuilding.

See the [provider contract and runnable example](../../docs/shared-resources.md), [runtime integration](README.md), and [changelog](CHANGELOG.en.md). Cross-process and native dynamic-library simulations validate ownership and serialization, not real device communication.

## Parameter roles

Declare the lifetime policy with `node("Name", lease(activation = required))`. Inside `init_device`, `config("Address", lease(key))` selects one or more resource identity fields. Other Config parameters form the compatibility configuration automatically; `lease(ignore)` excludes a parameter from both. At least one explicit resource key is required; configuration may be empty. Instance keys are selected independently with instance roles.

Single-field identities retain their existing canonical text. Composite identities use ordered, length-delimited values with a version prefix. `ResourceKey::from_parts` and `LeaseRequest::from_parameter_keys` expose this encoding for communication providers. Agree on value order, normalization, and configuration field names/order across providers. The legacy resource/configuration declaration and single-field API remain supported and cannot be mixed with parameter lease roles. See the [full contract and migration](../../docs/config-identity.md).

## 0.17.1: automatic runtime ownership and complete API docs

Framework 0.25.1 connects existing lease annotations to SharedResource automatically in Lease mode. One owner constructs the node, all references forward operations to it, and the last closes and drops it before releasing its lease. Node state is shared; full normalized Config, Init Input text, and the named node interface must match. Differing ignored Config values are rejected for automatic node sharing. Independent business objects must instead share a communication-layer provider. Direct mode keeps local construction.

ResourceHandler::close(request) runs only for the last session; errors and panics still allow destruction and lease release. SharedResource::close_with supplies the final close request and returns its response (empty for non-final references). Disconnect cleanup supplies an empty request. transact_with_timeout(None) waits for completion without an implicit business deadline. Resource protocol v2 adds a close payload/response and uses miresidian-resource-v2 endpoints and lock directories; stop all v1 owners and clients before upgrading. Configuration lease v2 and C ABI v7 are unchanged. Verify exact registry availability before using dependency examples.

## Exact-version documentation

0.17.1 enables all features for docs.rs only. The default feature set, local-ipc opt-in, runtime behavior, and resource/configuration protocols are unchanged. See the [full API](https://docs.rs/miresidian_lease/0.17.1/miresidian_lease/), [SharedResource methods](https://docs.rs/miresidian_lease/0.17.1/miresidian_lease/shared/struct.SharedResource.html), and [matching source/manuals](https://github.com/MyMires/miresidian/tree/miresidian_lease-v0.17.1/features/lease). Verify registry publication and docs.rs completion separately; use matching source while documentation is pending.
