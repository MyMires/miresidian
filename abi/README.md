# miresidian_abi

[文档中心](../docs/README.md) · [C ABI 完整契约](../docs/c-abi.md)

本检出为 **0.17.1 / ABI v10** 源码候选，尚未发布。节点通常通过根框架使用本 crate；宿主按实际 DLL 报告的 ABI 绑定函数，不从框架语义版本推断签名。

唯一业务调用为：

```c
uint32_t miresidian_abi_version(void);
const char *miresidian_invoke(const char *request_json);
```

先查询版本确认 **10**，再绑定单参数函数。v7-v9 的同名三参数函数、旧 miresidian_invoke_with 和 miresidian_invoke_request 不属于 v10；本次不保留旧调用别名。

请求如 `{"platform":1,"function":"GetFunctions"}`。function 必填，config/input 缺省 []；platform 缺省或 0（MIRESIDIAN_PLATFORM_DEFAULT）使用 DLL 固定默认平台，非零指定已注册平台。Config 不再包含用户可选的“平台适配”。平台编号、状态码、JSON 响应字段、线程内返回指针的所有权与复制要求见头文件及完整契约。

返回指针由 DLL 持有，须在同线程下一次业务调用前复制，不得释放；版本查询不替换响应缓存。未知/重复字段、显式 null、类型错误和非法请求均返回稳定错误响应。宿主必须迁移函数指针签名及旧 Config 布局，不能把 v10 当成 v9 的直接兼容替换。

本候选不作为可直接安装版本；旧项目继续查其已发布精确版本资料。历史 0.15.0/v8 资料保留在 [docs.rs](https://docs.rs/miresidian_abi/0.15.0/miresidian_abi/)，不据此推断 v10 可用性。

## English

Source candidate 0.17.1 defines ABI v10 and is unpublished. Its only invocation export is `miresidian_invoke(const char *request_json)`; retain `miresidian_abi_version()` for checking the signature before invocation. Old split-argument exports and the old request-export name are removed. JSON platform zero or omission selects the DLL's fixed default; nonzero selects a registered adapter. Config never selects a platform. Hosts must migrate the function-pointer type and Config layout. Response ownership, field order, stable platform identifiers and immediate-copy requirements remain as documented in the header.

0.17.1 仅将公共源码说明统一为业务节点/资源语义，状态码、平台常量、C 头文件和 ABI v10 签名均保持。
