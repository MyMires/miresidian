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

enum { MIRESIDIAN_STATUS_OK = 0, MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1, MIRESIDIAN_STATUS_OPERATION_FAILED = 2, MIRESIDIAN_STATUS_INTERNAL_ERROR = 3 };

/*
 * Config、Input、Output 与 Error 均为以 NULL 结尾的 UTF-8 字符串数组。
 * Config、Input 可传 NULL 表示空数组。Output、Error 由 DLL 写入；调用方不得释放，
 * 应在下一次写入同一输出通道前读取并复制其中的文本。
 */
MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke(const char *Function, const char *const *Config, const char *const *Input, char ***Output);
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke_with_error(const char *Function, const char *const *Config, const char *const *Input, char ***Output, char ***Error);
MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke_without_error(const char *Function, const char *const *Config, const char *const *Input, char ***Output);

#ifdef __cplusplus
}
#endif

#endif
