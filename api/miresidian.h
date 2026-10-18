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

typedef struct MiresidianOutputV1 {
    char **values;
    size_t count;
} MiresidianOutputV1;

MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);

MIRESIDIAN_API int32_t MIRESIDIAN_CALL miresidian_invoke_v1(
    const char *function,
    const char *const *config,
    size_t config_count,
    const char *const *input,
    size_t input_count,
    MiresidianOutputV1 *output);

MIRESIDIAN_API void MIRESIDIAN_CALL miresidian_free_output_v1(
    MiresidianOutputV1 *output);

MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_last_error_v1(void);

#ifdef __cplusplus
}
#endif

#endif
