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

#define MIRESIDIAN_ABI_VERSION 9

#ifdef __cplusplus
extern "C" {
#endif

enum {
    MIRESIDIAN_PLATFORM_RUNTIME_SELECT = 0,
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
 * @brief 查询当前节点 DLL 的 C ABI 版本，无需初始化设备。
 * @return ABI 版本号，当前为 9；不是框架或节点项目的语义化版本。
 * @note 框架版本和节点版本分别通过 GetFrameworkVersion、GetProjectVersion
 *       查询，从调用响应的 output[0] 读取。本函数不替换调用入口的响应缓存。
 */
MIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);

/**
 * @brief 使用节点运行时选择的平台适配器执行查询、生命周期或业务操作。
 *
 * InitDevice 通过 Config 中的保留“平台适配”配置选择已注册的平台。
 * GetFunctions、GetInitConfig、GetFrameworkVersion、GetProjectVersion 和
 * GetPortNO 不要求先初始化设备；普通设备操作须先成功调用 InitDevice。
 *
 * @param function 操作名称，如 "GetFunctions"、"InitDevice" 或节点公开的方法名。
 *        必须为非 NULL、以 NUL 结尾的 UTF-8 C 字符串。
 * @param config_json Config 参数通道；以 NUL 结尾的 UTF-8 JSON 字符串数组，
 *        如 "[\"COM3\",\"9600\"]"。NULL 等同于 "[]"，空字符串不是合法 JSON。
 * @param input_json Input 参数通道；格式和 NULL 规则同 config_json。
 *        两个数组的元素均须为字符串，且各自按参数声明顺序排列。
 * @return DLL 持有、以 NUL 结尾的 UTF-8 JSON 响应；成功和失败均通过此响应返回。
 *         调用方无需分配输出缓冲区，不得修改或释放返回指针，应立即复制文本。
 *
 * 所有非 NULL 输入指针须在本次调用期间保持可读，输入内存仍归调用方所有。
 * NULL function、非法 UTF-8 或 JSON 通过响应报告参数错误；无效地址或缺少
 * NUL 终止符不属于可校验的输入，调用方必须保证指针和缓冲区有效。
 *
 * 三个调用入口共用同一线程的响应缓存。返回文本只在同一线程下一次调用
 * miresidian_invoke、miresidian_invoke_with 或 miresidian_invoke_request、
 * 线程退出或 DLL 卸载前有效，
 * 以最早发生者为准。跨调用保存结果时，必须复制到调用方自己的内存。
 *
 * code 为稳定机器代码，成功时为 OK；失败时例如 NOT_INITIALIZED、INVALID_ARGUMENT。
 * status 是 MIRESIDIAN_STATUS_* 状态分类；调用方按 status/code 判断结果，
 * 不应解析诊断文本 message。output 始终为有序字符串数组，无输出或失败时为 []。
 * JSON 字段顺序固定为 status、code、message、output。成功时为：
 *   {"status":0,"code":"OK","message":"","output":["业务输出"]}
 * 失败时为：
 *   {"status":2,"code":"NOT_INITIALIZED","message":"设备尚未初始化","output":[]}
 *
 * @see miresidian_invoke_with
 */
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json);
/**
 * @brief 按平台路由值执行操作，与 miresidian_invoke 共享节点运行时。
 * @param platform MIRESIDIAN_PLATFORM_RUNTIME_SELECT（0）沿用 miresidian_invoke：
 *        GetInitConfig/GetFunctions 发布“平台适配”选项，宿主呈现供用户选择，
 *        InitDevice 按 Config 顺序传入所选平台文本，后续使用已选平台。
 *        0 不弹出界面；未传选择项时沿用默认平台，存活实例期间不可切换平台。
 *        非零值为已编译进该 DLL 并注册的平台枚举值。
 *        MIRESIDIAN_PLATFORM_TB_LV_TEST_PLATFORM（1）为内置 TB 平台；第三方平台
 *        编号须不小于 MIRESIDIAN_PLATFORM_CUSTOM_MIN（65536），且必须已注册。
 * @param function 操作名称；非 NULL、以 NUL 结尾的 UTF-8 C 字符串。
 * @param config_json UTF-8 JSON 字符串数组或 NULL；仅非零平台路径的 InitDevice Config
 *        不发布也不接受“平台适配”选择器，业务配置及其他框架配置仍按原契约传入。
 * @param input_json UTF-8 JSON 字符串数组或 NULL；NULL 等同于 "[]"。
 * @return 与 miresidian_invoke 相同的 DLL 持有响应；JSON 结构、输入指针要求、
 *         所有权及线程内有效期相同。任一调用入口都会替换当前线程的上一份响应。
 * @note 无效的保留编号通过响应返回 INVALID_PLATFORM_ID；合法但未注册的平台
 *       返回 UNKNOWN_PLATFORM_ADAPTER，二者均属于参数错误。此函数不会注册平台。
 * @see miresidian_invoke
 */
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke_with(
    uint32_t platform,
    const char *function,
    const char *config_json,
    const char *input_json);

/**
 * @brief 从完整 JSON 请求执行操作，与两个既有调用入口共享节点运行时。
 * @param request_json 非 NULL、NUL 结尾的 UTF-8 C 字符串，内容为单个 JSON 对象。
 *        function：必填非空字符串，操作名称精确匹配。
 *        config/input：可省略的字符串数组，默认 []，各按参数声明顺序排列。
 *        platform：可省略的 uint32 数值，须使用 JSON 整数字面量。
 *        不接受 null、重复/未知字段或错误类型；数组不可再次编码为字符串。
 *
 * 省略 platform 或传 0 时沿用 miresidian_invoke：GetInitConfig/GetFunctions 发布
 * “平台适配”选项，由宿主呈现给用户，InitDevice 通过保留 Config 选择平台，后续
 * 使用运行时已选适配器。未传选择项时沿用默认平台；存活实例期间不可切换平台。
 * 非零 platform 沿用 miresidian_invoke_with 的显式平台路径，InitDevice Config
 * 不发布也不接受“平台适配”选择器。
 *
 * platform 的 0 为运行时选择流程；非零为稳定编号：1 为 TB 内置平台，2..65535 当前保留，
 * 第三方从 65536 开始且必须已注册。禁止字符串（包括 "1"）、布尔、小数、负数和溢出。
 * 无效保留编号返回 INVALID_PLATFORM_ID，合法未注册编号返回 UNKNOWN_PLATFORM_ADAPTER；
 * 缺失 function、空 function、NULL/非法 UTF-8/JSON、错误字段类型返回 INVALID_ARGUMENT。
 * 上述错误 status 均为 MIRESIDIAN_STATUS_INVALID_ARGUMENT，output 为 []。
 *
 * @return 与 miresidian_invoke 相同的 DLL 持有响应，字段顺序为 status/code/message/output。
 *         调用方立即复制，不得修改或释放。任一调用入口的下一次同线程调用、线程退出或
 *         DLL 卸载会使此前响应指针失效，以最早发生者为准。ABI 版本查询不替换响应缓存。
 * @note 输入内存归调用方，本次同步调用期间必须保持可读。无效地址或缺少 NUL 终止符
 *       无法校验，调用方必须保证缓冲区有效。可展开 panic 转换为错误响应。
 * @example {"function":"GetFunctions","config":[],"input":[]}
 * @example {"platform":0,"function":"GetInitConfig"}
 * @example {"platform":1,"function":"GetFunctions"}
 * @since C ABI v8；v9 起 platform=0 进入 Config 平台选择流程。调用签名及响应生命周期保留。
 */
MIRESIDIAN_API const char *MIRESIDIAN_CALL miresidian_invoke_request(
    const char *request_json);

#ifdef __cplusplus
}
#endif

#endif
