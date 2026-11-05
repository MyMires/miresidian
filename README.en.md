# Miresidian

[中文](README.md) · [English](README.en.md)

Miresidian is a Rust framework for building node plugins with a stable C ABI. Independent node projects produce DLLs for hosts written in C, C#, Python, LabVIEW, and other languages. The framework provides node metadata, parameter codecs, lifecycle management, platform adapters, and concurrent execution. Device protocols and physical communication belong to the node's communication provider.

Current source: **0.24.0**. Rust MSRV: **1.89**. C ABI: **v7**. Configuration lease protocol: **v2**. Shared resource protocol: **v1**. Verify registry availability before updating an independent project.

## Getting started

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
miresidian = "=0.24.0"
```

Declare a node with `#[miresidian::node("Name")]` and mark its initialization, close, and business methods with `#[miresidian::operation]`. Macros generate metadata, dispatch, and DLL exports. See the [minimal node example](README.md#最小节点示例), [node guide](docs/宏驱动开发.md), and [C ABI guide](docs/c-abi.md).

## Optional capabilities

| Feature | Purpose |
| --- | --- |
| `instances` | Explicit Config-key or Handle identity, multiple objects, and multi-channel execution |
| `lease` | Compatible configuration leases and communication-layer shared resource proxies |
| `format` | Explicit input/output text and byte conversions |
| `batch` | Type-driven sequential expansion of strings, booleans, numbers, and supported custom inputs |

The default feature set is empty. Enabling a feature does not by itself activate every policy or rewrite device communication.

## Object reuse and physical resource sharing

Within one DLL, Config binding reuses an object for the same normalized business configuration. Handle binding reuses an object only when the Handle and configuration match. Different Handles and DLLs retain independent business objects. Repeated initialization does not increment an object reference count; `CloseDevice` closes the selected object.

`LocalLease` checks configuration compatibility. It does not make repeated opens of an exclusive serial port succeed. A communication provider must explicitly use `lease::shared::SharedResource` to share one physical connection across objects, DLLs, or projects. All participants must agree on resource identity, configuration encoding, and a versioned transaction protocol. Open the device inside the supplied factory, and perform the full send/wait/receive exchange in one transaction. The last proxy closes the physical connection.

The runtime Lease/Direct selector controls the runtime configuration lease only. It does not automatically switch a provider between proxy and direct communication. Existing binaries require communication-layer integration and rebuilding.

See the [resource provider contract and runnable simulated example](docs/shared-resources.md), [instance rules](docs/instances.md), and [lease package](features/lease/README.en.md).

## Validation and releases

The shared resource implementation has been tested using exclusive file handles, independent processes, renamed dynamic libraries, and distinct node types. Linux dynamic loading passed; Windows x64/x86 compilation checks passed. These results do not establish real serial communication or Windows host loading.

The batch/format update is validated with independent feature configurations, typed records, aligned format columns, and native async methods. Unchanged resource-sharing validation remains applicable. Cargo publication verifies normalized packages against registry dependencies. Independent nodes must use published exact framework versions, without path/git dependencies or local overrides.

[Release notes](docs/releases/0.24.0.md) · [Changelog](CHANGELOG.en.md) · [Documentation index](docs/README.md) · [Build and maintenance](docs/coding-standards.md) · [CLI](tools/miresidian-cli/README.md)


## Typed batch and explicit formats

Batch parsing uses the original Rust argument type, including aliases and qualified paths. Integers retain single values, lists, nonnegative ascending ranges, and mixed input such as `1-3|5|8-10`; strings preserve range-like text, while booleans and floats do not expand ranges. String and bool inputs do not require format annotations. Empty batch text is one record whose validity is determined by the target type.

Format remains a single-value encoding conversion. The macro combines the independent batch and format crates, aligning data and format-selection columns by singleton broadcast or equal length. `A|42|Qw==` with `UTF8|HEX|Base64` produces three String values A, B, and C. Batch output format selectors also accept columns. Fixed single-format annotations create no selector or additional input slot. All input conversion and alignment checks precede business execution; output failures can occur after effects and stop subsequent records without rollback.

Config-bound composite identities and nested parameter roles are described in the [0.24.0 contract and migration](docs/config-identity.md). Instance selection and lease resource identity are independent; unmarked lease fields form compatibility settings.
