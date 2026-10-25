#ifndef MIRESIDIAN_H
#define MIRESIDIAN_H

#include <stdint.h>

#if defined(_WIN32)
#define MIRESIDIAN_API __declspec(dllimport)
#define MIRESIDIAN_CALL __cdecl
#else
#define MIRESIDIAN_API
#define MIRESIDIAN_CALL
#endif

#define MIRESIDIAN_ABI_VERSION 7

#ifdef __cplusplus
extern "C" {
#endif

enum {
    MIRESIDIAN_PLATFORM_TB_LV_TEST_PLATFORM = 1,
    MIRESIDIAN_PLATFORM_CUSTOM_MIN = 65536
};

enum {
    MIRESIDIAN_STATUS_OK = 0,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1,
    MIRESIDIAN_STATUS_OPERATION_FAILED = 2,
    MIRESIDIAN_STATUS_INTERNAL_ERROR = 3
};

/*
 * function 为 UTF-8 C 字符串；config_json、input_json 是 UTF-8 JSON 字符串数组。
 * 返回值 result_json 由 DLL 在当前线程中持有；调用方应在下一次调用前复制文本，且不得释放指针。
 * 保留的常驻查询包括 GetFrameworkVersion（框架版本）与 GetProjectVersion（项目版本）。
 * 二者都通过 miresidian_invoke 返回 result_json 的 output[0]。
 *
 * JSON 字段顺序固定为 status、message、output。成功时为：
 *   {"status":0,"message":"","output":[..业务输出]}
 * 失败时为：
 *   {"status":错误码,"message":"错误信息","output":[]}
 */
MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json);
/*
 * platform 是已编译进 DLL 的 Platform 枚举数值；`1` 为 `tb_lv_test_platform`，第三方平台必须不小于 `65536`。
 * 此显式选型路径对应 Rust 的 NodeRuntime::invoke_with：InitDevice 的 Config 不得包含平台适配选择器。
 */
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke_with(
    uint32_t platform,
    const char *function,
    const char *config_json,
    const char *input_json);

#define MIRESIDIAN_ABI_VERSION 7

#ifdef __cplusplus
}
#endif

#endif
