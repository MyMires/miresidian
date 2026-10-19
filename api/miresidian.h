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

#ifdef __cplusplus
extern "C" {
#endif

enum {
    MIRESIDIAN_STATUS_OK = 0,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1,
    MIRESIDIAN_STATUS_OPERATION_FAILED = 2,
    MIRESIDIAN_STATUS_INTERNAL_ERROR = 3,
    MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL = 4
};

/*
 * function 为 UTF-8 C 字符串；config_json、input_json 是 UTF-8 JSON 字符串数组。
 * output_json 指向调用方分配的可写 UTF-8 缓冲区，容量包含末尾 NUL 字符。
 * DLL 不分配、不持有、也不释放调用方内存。
 *
 * 返回值为状态码。JSON 字段顺序固定为 output、error。成功时 output_json 为：
 *   {"output":[..业务输出],"error":null}
 * 失败时 output_json 为：
 *   {"output":[],"error":"错误信息"}
 */
MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json,
    char *output_json,
    uint32_t output_capacity);

#ifdef __cplusplus
}
#endif

#endif
