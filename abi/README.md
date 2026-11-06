# miresidian_abi

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_abi` 定义 Miresidian 节点 DLL 与宿主之间的稳定 C ABI：状态码、公共 FFI 类型、入口名称与 ABI 版本。

通常节点项目只需依赖 `miresidian`。只有实现宿主适配层、检查 ABI 兼容性，或需要直接使用公共 FFI 类型时，才直接引入本 crate。

本检出为 `miresidian_abi 0.15.1` 的未发布文档补丁，C ABI **v8**、头文件及 Rust/C 签名不变。0.15.0 已在后续授权阶段正式发布，下面使用该已核实版本；本次不发布 0.15.1，也不以本地覆盖替代发行。

```toml
# 已核实的发行版本；采用其他版本前重新核对 registry。
[dependencies]
miresidian_abi = "=0.15.0"
```

v8 新增 `const char *miresidian_invoke_request(const char *request_json)`，保留 v7 的两个调用入口。请求示例 `{"function":"GetFunctions"}`；`function` 必填非空，`config`/`input` 为可省略的有序字符串数组，默认 `[]`。`platform` 为可选 u32 数值：省略沿用默认入口的 Config 平台选择，提供沿用显式平台入口且初始化 Config 不接受平台选择器。编号 `1` 为 TB，`2..65535` 当前保留，第三方从 `65536` 起且须已注册。拒绝 null、错误类型、未知/重复字段；平台错误与普通参数错误见完整契约。

三个调用入口共用同一线程的响应缓存，响应仍为 `status/code/message/output`。返回指针由 DLL 持有，调用方立即复制且不得释放；同线程任一调用入口的下一次调用、线程退出或卸载使原指针失效。ABI 版本查询不替换缓存。已有调用签名不变，宿主若按 ABI 恰等于 7 拒绝加载，须在评估兼容性后增加 v8 支持。

完整 C ABI 调用约定见[C ABI](../docs/c-abi.md)。

精确发行资料：[0.15.0 API](https://docs.rs/miresidian_abi/0.15.0/miresidian_abi/)；本次补丁文档从本检出生成，源码版本不代表已经发布。
