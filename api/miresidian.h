#ifndef MIRESIDIAN_H
#define MIRESIDIAN_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#define MIRESIDIAN_API __declspec(dllimport)
#define MIRESIDIAN_CALL __cdecl
#else
#define MIRESIDIAN_API
#define MIRESIDIAN_CALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum {
    MIRESIDIAN_STATUS_OK = 0,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1,
    MIRESIDIAN_STATUS_OPERATION_FAILED = 2,
    MIRESIDIAN_STATUS_INTERNAL_ERROR = 3
};

typedef struct MiresidianTextArray {
    char **values;
    size_t count;
} MiresidianTextArray;

MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);

/* 合并结果：[错误码, 错误信息, ..业务输出]。 */
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke(
    const char *function,
    const char *const *config,
    size_t config_count,
    const char *const *input,
    size_t input_count,
    MiresidianTextArray *output);

/* 业务输出与错误信息分离；失败时 error 为 [错误码, 错误信息]。 */
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke_with_error(
    const char *function,
    const char *const *config,
    size_t config_count,
    const char *const *input,
    size_t input_count,
    MiresidianTextArray *output,
    MiresidianTextArray *error);

/* 仅返回业务输出；失败原因只能由返回状态码获知。 */
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke_without_error(
    const char *function,
    const char *const *config,
    size_t config_count,
    const char *const *input,
    size_t input_count,
    MiresidianTextArray *output);

MIRESIDIAN_API void MIRESIDIAN_CALL miresidian_free_text_array(
    MiresidianTextArray *value);

#ifdef __cplusplus
}
#endif

#endif
