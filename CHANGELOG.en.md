# Changelog

[中文](CHANGELOG.md) · [English](CHANGELOG.en.md)

## [Unreleased]

## [0.26.1] - 2026-11-07

- Clarify Chinese source documentation for macro expansion, conditional compilation, task ownership, cancellation, initialization cleanup, and module loader references; correct the private fixture ABI note.
- Correct release status, the 0.22.0 parameter migration boundary, Direct/Shared ownership diagrams, workspace inventory, component examples, and new/init directory rules. Explain the existing ABI v7 template restriction with an explicit published framework version.
- Track macro 0.23.1, ABI crate 0.15.1, and instances 0.16.6. Pack 0.13.3 and CLI/toolset 0.3.3 update guidance and the embedded version catalog only. Executable code, Rust/C signatures, C ABI v8, configuration/resource protocols v2, and MSRV 1.89 remain unchanged.
- Erratum: framework 0.26.0, macro 0.23.0, and ABI 0.15.0 were published in the subsequently authorized release phase; exact-version docs are available. Preserve original entries/tags. This patch set is local and unpublished; see plans/2026-11-07-comments-and-documentation.md for evidence.
- Validation: executable source in 30 Rust files is unchanged after excluding comments. Passed 43 component/tool tests, one root doctest, strict all-feature documentation, six example checks, and real registry-backed CLI new/init smoke checks. Version, formatting, and local document reference checks passed; no new Windows loading or hardware validation.

## [0.26.0] - 2026-11-06

- Add `miresidian_invoke_request` with a complete JSON request: required nonempty function, optional ordered Config/Input string arrays, and optional numeric platform. Omission uses existing Config-based platform selection; presence uses explicit selection.
- Reject null, malformed types, unknown/duplicate fields, and invalid platform values. Keep existing responses, error categories, panic isolation, thread-local response ownership, and shared node state across all three call entries.
- Source candidates: framework 0.26.0, macro 0.23.0, ABI crate 0.15.0 / C ABI v8. Preserve both old call signatures; synchronize technical documentation, exact internal dependencies, and the workspace lockfile. Unrelated components/tools/protocols are unchanged.
- Local source/tag delivery only; no publication or independent-node upgrades. No Windows or hardware validation claim. Evidence is recorded in plans/2026-11-06-unified-abi-request.md.

## [0.25.2] - 2026-11-06

- Publish Node DLL layout guidance: keep one user-facing Node DLL at the top of each node's architecture directory and place accompanying third-party and helper runtime DLLs under `dependencies/`. Document the entry filename, architecture, and purpose of the dependencies.
- Synchronize bilingual README files, current exact-version examples, and the node development guide. Include the archived layout policy and Node skill in the release; actual artifact rearrangement still requires dependency resolution and target-platform loading checks. Pack does not gain automatic third-party DLL collection in this release.
- Bump only the framework patch version to 0.25.2. Runtime code, macros, other crates/tools, dependency versions, C ABI v7, configuration lease v2, resource protocol v2, and Rust 1.89 are unchanged. Existing nodes need no upgrade for this guidance update.
- Follow the written release plan to verify the formal package build, provenance, remote tag, registry entry, and exact-version online documentation. Record actual publication results separately in that plan.
- Validation: version gates, diffs, 69 local references across six documents, and strict all-feature Rustdoc passed. Root API and format pages show 0.25.2. Manifests and lockfiles change only the root version; runtime, component, and tool code is unchanged.

## [0.25.1] - 2026-11-06

- Track exact lease 0.17.1 and complete its optional shared-resource API documentation build configuration and exact-version source/API entries; synchronize bilingual guidance and the shared-resource manual.
- Runtime Lease/Direct behavior, macro syntax/version, other components, C ABI v7, configuration lease v2, resource protocol v2, and MSRV are unchanged. Reuse the 0.25.0 runtime and dynamic-library validation.
- Follow the written release plan to publish only framework 0.25.1 and lease 0.17.1, retaining previous tags; verify package contents, pushed source, and docs.rs independently. Actual release results are recorded in the plan.

## Plan-first workflow policy - 2026-11-06

- Require a written task design and implementation plan before framework/component/tool or Node creation, updates, fixes, refactoring, and policy maintenance. Read-only investigation and plan writing precede implementation, including template initialization and small fixes.
- Bind implementation to the plan's concrete architecture, interfaces, resource/state/lifecycle boundaries, file scope, version assessment, steps, and acceptance evidence. Revise the plan before deviating and satisfy applicable confirmation requirements; do not backfill plans or weaken acceptance criteria to conceal gaps.
- Update the complete Node skill, planning reference, review checks, maintenance rules, and UI metadata, then synchronize the installed copy. Record this task's plan before policy edits.
- Version impact: none; policy, task-plan, and skill changes only. Product code, macro syntax, templates, dependencies, lockfiles, tool commands, versions, and existing release tags are unchanged.
- Validation: both complete 5-file skill inventories and SHA-256 match; restricted YAML/UI metadata, 35 local links/anchors across 13 Markdown files, version gates, and diff checks pass. Reviewed five task scenarios. The generic validator could not run without PyYAML; no unrelated builds were run.

## [0.25.0] - 2026-11-06

- Existing lease annotations now acquire one node owner across DLLs/processes: construct once, forward complete operations, close/drop only on the final reference. Macro syntax and macro crate 0.22.0 are unchanged.
- Node state and initialization output are shared. Full normalized Config, Init Input text, and the named interface must match. Direct construction remains local; independent business state uses a communication-layer provider.
- Lease 0.17.0 adds final-close payloads/responses and optional transaction deadlines. Resource protocol v2 requires stopping v1 participants before upgrading; configuration lease v2 and C ABI v7 remain unchanged.
- Added direct-open DLL fixtures and native loading verification. See release notes for validation; no physical COM5 test, registry publishing, or independent-node dependency changes in this task.

## [0.24.1] - 2026-11-06

- Correct stale Config[0] identity guidance in instance documentation and generated Node templates; resolve contracts against each project's actual framework version.
- Complete lifecycle/discovery signatures, cleanup boundaries and found/not-found/error mappings. Add precise release-source navigation, older component-documentation fallbacks and manual Windows GNU setup.
- Enable all-feature docs.rs builds for the public format API. Require versioned components to update their own README and framework releases to update relevant core guides; validate docs.rs metadata and version template/toolset changes. Preserve historical policy snapshots and independent skill maintenance.
- Source versions: runtime 0.24.1, Pack 0.13.2 and CLI/toolset 0.3.2. Other crate versions, runtime APIs, C ABI v7, lease v2 and resource protocol v1 are unchanged. Installation guidance queries registry releases rather than assuming source versions are installable.
- Validation: 29 version-gate tests, 33 tool tests and two real-registry template integrations passed. Strict all-feature Rustdoc exposes the public format API and discovery contract; CI runs the same documentation check. Local references and Windows configuration syntax passed. Registry publication and online documentation availability remain separate checks.

## Guidance clarification - 2026-11-06

- Documentation clarification (no version impact): use consistent representations for actual values and display labels within each parameter's choices; prefer `/` in generated paths and examples. Review findings against declared usage constraints, actual scenarios and impact. Synchronize the complete installed Node skill; runtime, API, ABI, dependencies and tool behavior are unchanged.

## [0.24.0] - 2026-11-05

- Document the contract before implementation and add nested init Config roles instance(key/ignore) and lease(key/ignore), without standalone parameter attributes.
- Config-bound instances default to all business Config. Explicit keys form an allowlist; ignore-only annotations exclude fields. Sync/async operations, multi-channel requests and close share composite identity, with conflicting reinitialization settings rejected.
- Leases require explicit resource keys; other Config fields are compatibility settings unless ignored. Preserve legacy declarations and ordinary single-key bytes; add versioned, length-delimited composite identities.
- Independently version runtime 0.24.0, macros 0.22.0 and lease 0.16.0; instances 0.16.5 only corrects shipped explanations. Other components/tools, C ABI v7, lease v2, resource v1 and Rust 1.89 remain unchanged. Synchronize the complete Node skill; standalone projects retain published dependencies.
- Validation: default workspace (107), all-feature workspace (219), two supplemental joint-role/FFI regressions, isolated instances (4), isolated lease (2), doctest, strict Clippy/Rustdoc, Rust 1.89 and Release build passed. The complete nine-file skill is synchronized. Linux simulations and in-process C ABI were tested; native Windows loading and real hardware were not tested.

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
