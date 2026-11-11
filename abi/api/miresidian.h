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

#define MIRESIDIAN_ABI_VERSION 10

#ifdef __cplusplus
extern "C" {
#endif

enum {
    MIRESIDIAN_PLATFORM_DEFAULT = 0,
    MIRESIDIAN_PLATFORM_TB_LV_TEST_PLATFORM = 1,
    MIRESIDIAN_PLATFORM_CUSTOM_MIN = 65536
};

enum {
    MIRESIDIAN_STATUS_OK = 0,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT = 1,
    MIRESIDIAN_STATUS_OPERATION_FAILED = 2,
    MIRESIDIAN_STATUS_INTERNAL_ERROR = 3
};

/**
 * @brief 查询 ABI 版本；v10 的 miresidian_invoke 使用单个 JSON 请求参数。
 * @note 宿主必须先核对 ABI，不能用旧版三参数函数指针调用 v10 同名入口。
 */
MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);

/**
 * @brief 唯一业务调用入口，接收完整 JSON 请求并返回 DLL 持有的 JSON 响应。
 * @param request_json 非 NULL、NUL 结尾的 UTF-8 JSON 对象，调用期间保持可读。
 *        function：必填非空字符串，操作名精确匹配。
 *        config/input：可省略的有序字符串数组，默认 []。
 *        platform：可省略的 uint32 整数；0/省略为 DLL 默认平台，非零指定已注册平台。
 *        1 为内置 TB，2..65535 保留，第三方从 65536 开始。
 *        平台由本次调用确定，Config 不发布也不接受平台选择项。
 *        活跃实例期间不能改用其他平台；关闭全部实例后可重新指定。
 *
 * 拒绝未知/重复字段、null、错误类型及非对象请求。platform 不接受字符串、布尔、
 * 小数、负数或溢出。无效保留编号返回 INVALID_PLATFORM_ID，合法未注册编号返回
 * UNKNOWN_PLATFORM_ADAPTER；NULL/非法 UTF-8/JSON/字段返回 INVALID_ARGUMENT。
 *
 * @return 固定 JSON 对象：status、code、message、output。output 恒为字符串数组。
 *         成功例：{"status":0,"code":"OK","message":"","output":["结果"]}。
 *         失败时 output 为 []，宿主按 status/code 判断结果，不解析 message。
 *         DLL 持有返回指针，宿主立即复制，不得修改或释放；同线程下一次调用、
 *         线程退出或 DLL 卸载使指针失效。版本查询不替换响应缓存。
 *
 * 输入内存归调用方；无效地址或缺少 NUL 不能安全校验。可展开 panic 转为错误响应。
 * @example {"platform":1,"function":"GetFunctions"}
 * @example {"platform":1,"function":"InitDevice","config":["COM3"],"input":[]}
 * @since ABI v10：原完整请求入口更名；旧分参数调用入口已移除。
 */
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke(const char *request_json);

#ifdef __cplusplus
}
#endif

#endif
