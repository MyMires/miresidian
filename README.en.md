# Miresidian

[中文](README.md) · [English](README.en.md)

Miresidian is a Rust framework for building node plugins with a stable C ABI. Independent node projects produce DLLs for hosts written in C, C#, Python, LabVIEW, and other languages. The framework provides node metadata, parameter codecs, lifecycle management, platform adapters, and concurrent execution. Device protocols and physical communication belong to the node's communication provider.

Current source: **0.29.0**. Rust MSRV: **1.89**. C ABI: **v9**. Configuration lease protocol: **v2**. Shared resource protocol: **v2**. This public-type rename source candidate is unpublished. The example below deliberately uses published **0.26.0**, whose registry and exact-version documentation were previously rechecked on 2026-11-07. Independent nodes use published versions without local overrides; verify availability again when choosing another version.

## Getting started

Documentation is organized by topic in the [documentation index](docs/README.md). Each rule is defined in its owning guide; skill instructions and other guides refer to it rather than keeping independent copies.

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
miresidian = "=0.26.0"
```

Declare a node with `#[miresidian::node("Name")]` and mark its initialization, close, and business methods with `#[miresidian::operation]`. Macros generate metadata, dispatch, and DLL exports, including `const char *miresidian_invoke_request(const char *request_json)`. The new entry takes one request object such as `{"function":"GetFunctions"}`. In source 0.29.0 / ABI v9, omitted or zero `platform` follows Config selection: query `{"platform":0,"function":"GetInitConfig"}`, let the host present the published platform choices, then pass the selection in InitDevice Config. Later zero calls use the selected adapter. Nonzero values select a registered adapter explicitly. The published 0.26.0 / ABI v8 installation example above still rejects zero; omit the field to use Config selection there. Config/Input remain ordered string arrays, defaulting to `[]` only when omitted. All three call entries share runtime state and the thread-local response buffer; copy the response before another call and never free it. See the [minimal node example](README.md#最小节点示例), [node guide](docs/宏驱动开发.md), and [C ABI guide](docs/c-abi.md).

## Feature shorthand in this source candidate

Framework **0.28.0 / macro 0.24.0** adds the following forms. They are not available through the published dependency example above. Keep old annotations when maintaining older nodes; no migration is required.

```rust,ignore
#[miresidian::node("Device", instances(multiple, config), lease)]
#[miresidian::input("Data", batch, format(hex))]
#[miresidian::output("Result", format(utf8, hex, base64, selector = "Result format"))]
```

Bare `instances` or `instances()` means single-object, fixed sequential multi-channel execution; omitting it still rejects multi-channel requests. Multiple objects require explicit `multiple, config` or `multiple, handle`; add a fixed execution mode or `selectable(sequential)` to override the short-form default. Bare `lease` equals `lease(required)`; `lease(disabled)` fixes Direct access and `lease(selectable(required))` exposes selection with Lease as the default. Parameter identity roles stay unchanged.

`format(hex)` fixes one encoding. Multiple formats require an explicit selector, and the first format remains the default. Input and Output share the same syntax and continue to compose with independent `batch`. Cargo features remain explicit. Old long forms, metadata order, errors and resource lifetimes remain compatible; reject duplicate declarations and mixed long/short fields within one group. See the [migration guide](docs/releases/0.28.0.md).

## Optional capabilities

| Feature | Purpose |
| --- | --- |
| `instances` | Explicit Config-key or Handle identity, multiple objects, and multi-channel execution |
| `lease` | Automatic shared node ownership, configuration leases, and communication-layer resource proxies |
| `format` | Explicit input/output text and byte conversions |
| `batch` | Type-driven sequential expansion of strings, booleans, numbers, and supported custom inputs |

The default feature set is empty. Enabling a feature does not by itself activate every policy or rewrite device communication.

## Object reuse and physical resource sharing

Within one DLL, Config binding reuses an object for the same normalized business configuration. Handle binding reuses an object only when the Handle and configuration match. In Direct mode or the communication-provider pattern, different Handles and DLLs retain independent business objects. Automatic Lease mode shares the owner object. Repeated initialization does not increment an object reference count; `CloseDevice` closes the selected object.

The existing `lease(...)` annotations now select automatic node ownership in Lease mode. One owner initializes the node, later DLLs attach without calling their constructors, complete business operations are serialized, and only the last reference closes and drops the node. Node state is shared. The node name/interface and all normalized business Config values and Init Input text must agree; filenames, project versions, and Handles do not identify the physical resource. Incompatible participants are rejected.

Direct mode still constructs a local node. For independent business state or different node interfaces sharing a connection, use a common communication provider with `lease::shared::SharedResource`. Stop all resource protocol v1 participants before upgrading to v2. Existing binaries require rebuilding against a published framework release.

See the [resource provider contract and runnable simulated example](docs/shared-resources.md), [instance rules](docs/instances.md), and [lease package](features/lease/README.en.md).

## Validation and releases

The shared resource implementation has been tested using exclusive file handles, independent processes, renamed dynamic libraries, and distinct node types. Linux dynamic loading passed; Windows x64/x86 compilation checks passed. These results do not establish real serial communication or Windows host loading.

The batch/format update is validated with independent feature configurations, typed records, aligned format columns, and native async methods. Unchanged resource-sharing validation remains applicable. Cargo publication verifies normalized packages against registry dependencies. Independent nodes must use published exact framework versions, without path/git dependencies or local overrides.

[Runtime platform selection and v9 migration](docs/releases/0.27.0.md) · [Changelog](CHANGELOG.en.md) · [Documentation index](docs/README.md) · [Build and maintenance](docs/coding-standards.md) · [CLI](tools/miresidian-cli/README.md)

When delivering a Node DLL, keep one user-facing Node DLL at the top of each node's architecture directory and place accompanying third-party and helper runtime DLLs under `dependencies/`. Document the entry filename, architecture, and dependency purpose; no empty directory is needed when there are no such dependencies. See the [delivery layout and loading checks](docs/coding-standards.md#6-注释与文档).


## Typed batch and explicit formats

Batch parsing uses the original Rust argument type, including aliases and qualified paths. Integers retain single values, lists, nonnegative ascending ranges, and mixed input such as `1-3|5|8-10`; strings preserve range-like text, while booleans and floats do not expand ranges. String and bool inputs do not require format annotations. Empty batch text is one record whose validity is determined by the target type.

Format remains a single-value encoding conversion. The macro combines the independent batch and format crates, aligning data and format-selection columns by singleton broadcast or equal length. `A|42|Qw==` with `UTF8|HEX|Base64` produces three String values A, B, and C. Batch output format selectors also accept columns. Fixed single-format annotations create no selector or additional input slot. All input conversion and alignment checks precede business execution; output failures can occur after effects and stop subsequent records without rollback.

Config-bound composite identities and nested parameter roles are described in the [0.24.0 contract and migration](docs/config-identity.md). Instance selection and lease resource identity are independent; unmarked lease fields form compatibility settings.

This checkout contains framework 0.29.0, macro 0.25.0, codec 0.17.0, and unchanged ABI crate 0.16.0 / C ABI v9. Public types are Node, NodeInput, NodeOutput, NodeCollectionElement, NodeScalar and NodeValue. Codec trait methods are parse_text, normalize_text, into_output, parse_collection and format_collection. This is a breaking Rust source rename without old aliases; runtime ownership, platform routing, encoded data and C ABI remain unchanged. See the [type migration](docs/releases/0.29.0.md). The preceding framework 0.26.0, macro 0.23.0, and ABI 0.15.0 were formally published in the release phase recorded in the [release plan](plans/2026-11-06-unified-abi-request.md#发布执行结果). See the published [0.26.0 API](https://docs.rs/miresidian/0.26.0/miresidian/) and [format module](https://docs.rs/miresidian/0.26.0/miresidian/format/index.html). Hosts must review compatibility and add v9 to their supported version set before accepting this candidate. Generate this unpublished candidate's local API with `cargo doc -p miresidian --all-features --no-deps --locked`; source commits, local tags, registry publication, and docs.rs availability remain separate checks.

CLI/toolset source is 0.3.4. Pack's existing new/init templates accept only ABI v7, so explicitly select a compatible published framework such as 0.25.2 when creating a project. Selecting 0.26.0/v8 writes the template but reports incomplete creation; it does not produce a validated v8 template. See the [CLI commands](tools/miresidian-cli/README.md) and migrate existing projects manually using the actual target ABI header.
