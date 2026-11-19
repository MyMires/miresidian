# miresidian_testflow_windows

[文档中心](../../docs/README.md) · [CLI](../miresidian-cli/README.md)

本说明对应 **0.14.1**；源码提交、registry发行与在线文档可用性分别核对。工具把节点项目的 `.testflow` 生成同名、自包含的 Windows BAT。只维护原流程文件；不需要预期旁文件、报告文件或外置 PowerShell 脚本。

在声明 `cdylib` 的节点项目根目录运行：

```text
cargo miresidian-testflow-windows tools/smoke.testflow
cargo miresidian-testflow-windows --check tools/smoke.testflow
cargo miresidian-testflow-windows --help
cargo miresidian-testflow-windows --version
```

统一 CLI 仍可转发 `cargo miresidian testflow --os windows tools/smoke.testflow`；须实际安装对应工具，旧套件不会因此自动固定新后端。安装前查阅 [registry 发行记录](https://crates.io/crates/miresidian_testflow_windows/versions)，不要将未发布候选当作可安装版本。

BAT 相对于自身目录查找流程和 `../dist/**/windows-x64|windows-x86/<cdylib>.dll`，支持 UNC 启动目录。生成器不构建 DLL、不访问设备。BAT 使用 Windows PowerShell，默认结束后暂停；自动化设置 `MIRESIDIAN_TESTFLOW_NO_PAUSE=1` 后跳过暂停并保留退出码。

## 调用、Config 和多通道

保留普通单通道调用。以下三个对象示例是**同一种通道映射语法**，分别选择一个通道、离散集合和连续范围：

```text
发送数据("AA", "HEX")
发送数据({"ch1": ["AA", "HEX"]})
发送数据({"ch1,ch3": ["AA", "HEX"]})
发送数据({"ch1-ch6": ["AA", "HEX"]})
```

普通调用在共用 Config 下执行一次；逐通道 Config 下只使用 ch1，缺少 ch1 报错。映射值是字符串参数数组，`null` 表示跳过；最大显式通道和中间空位都会保留，最大 256 通道。编号从1开始，不允许前导零、空白、倒序、重复键和重叠选择。对象不能为空。零参数使用 `[]`，受下述 TB 批次编码限制。

```text
# 共用配置；实际参数必须按 DLL 的 GetInitConfig 填写
Config("资源A", "其他配置")
InitDevice()
发送数据({"ch1,ch3": ["AA", "HEX"], "ch6": null})
CloseDevice()

# 每个通道选择自己的配置；相同配置可以用集合或范围表达
Config({"ch1,ch3": ["资源A", "其他配置"], "ch2": ["资源B", "其他配置"]})
InitDevice()
发送数据({"ch1-ch3": ["AA", "HEX"]})
CloseDevice()
```

示例只说明语法，不代表具体串口 DLL 的完整参数。`Config` 选择配置，不隐式初始化/关闭；映射形式整条验证后替换当前映射，缺失尾部项按既有元数据默认值补齐。配置按补齐后的字符串数组**精确比较**，不猜测地址别名、Handle 或实例身份。独立配置需要 DLL 本身支持多实例，成功 Init 响应不能证明对象一定新建。

`InitDevice(...)` / `CloseDevice(...)` 的普通 Input 作用于当前全部配置；对象 Input 只作用于选中且非 null 的通道。Config 和 Input 分别传入 DLL。同一行相同 Config、相同 Input 去重；相同 Config、不同生命周期 Input 整行拒绝。关闭 Input 原样透传，实际参数契约由 DLL 校验。切换 Config 不会释放之前的对象。

同一业务语句中，非 null 通道的完整 Config 相同且最高通道大于1时，发送**一次真实 TB 批调用**，使用 DLL 自身的 Sequential/Parallel/Broadcast 策略。Broadcast 的源仍是全局 ch1，不自动补源。不同 Config 则按通道号分别调用，首个失败后不派发剩余通道；这条路径不代表 DLL 并行或广播。

TB 批次以英文逗号编码每通道 Input，因此零参数数组、含英文逗号、空字符串、需要保留首尾空白的参数会在该行调用前被拒绝。普通调用及跨 Config 普通调用可以保留这些值；普通调用首参数 `"共享"` 是 TB 保留标记，会拒绝。可编码的错误参数数量交给 DLL 验证，不擅自补业务参数。

仅 ch1 映射走普通调用；`ch1:null` 不调用。共用 Config 的多个全 null 槽位仍发送一次真实跳过批次；逐通道 Config 全 null 时不调用，不能断言不存在的响应。

## 执行结果与内联断言

**没有 `=>` 是执行模式**：始终显示真实 `status/code/message/output`，包括多个返回参数、失败输出和批次每个返回槽位；不作业务预期断言。DLL 失败仍会终止流程。旧 DLL 没有 code 时按 status 处理，不编造 code。

**带 `=>` 是断言模式**：先显示相同的完整实际结果，再比较声明的预期。只断言 code 也不会隐藏实际 output：

```text
发送数据("AA", "HEX") => {"code": "OK"}

# 普通响应按原始数组逐项精确比较，展示多个返回参数
测量("直流") => {
    "status": 0,
    "output": ["12.0", "0.5", "正常"]
}

# 多通道预期同样支持单通道、集合和范围
测量({"ch1-ch3": ["直流"]}) => {
    "code": "OK",
    "output": {
        "ch1,ch3": ["12.0", "0.5", "正常"],
        "ch2": ["11.9", "0.6", "正常"]
    }
}
```

预期只允许整数 status、字符串 code、output，至少声明 status 或 code。多通道 output 使用映射，只检查列出的通道；值可以是完整返回文本或多个返回参数的字符串数组。跨 Config 响应保留每通道原始数组，可以逐项精确比较。**真实 TB 批次仅返回槽位编码文本**：数组预期先用英文逗号连接后比较，不把实际文本反向拆分；字段含逗号时不提供无损字段边界断言。诊断会标明文本比较。不存在的返回槽位不会补造为空。

明确预期的错误匹配后可继续下一语句，实际失败和“符合预期”分别展示；不重试该语句，也不补执行其剩余通道。预期成功却失败、未声明的失败、输出不匹配和清理错误均非零退出。语句可以跨行，支持独占行 `#` / `//` 注释；JSON 不接受注释或尾逗号，字符串中的括号和 `=>` 按文本处理。

`Delay("500")` 是工具等待指令，范围0..2147483647毫秒；Config/Delay 不能附 DLL 预期。`--check` 不加载 DLL，检查语法及配置/初始化请求的基本顺序；不证明 Init 成功、配置归一化后的对象存在或方法签名正确，这些由实际 DLL 验证。

## 生命周期、ABI 与租约验证边界

每个 BAT 独占一个宿主进程及本地 DLL runtime。正常结束、意外错误和断言失败都在 finally 以空 Config/Input 调用 `CloseAllDevices`，显示清理实际结果；主错误和清理错误都保留。不支持接管同一 runtime 中未知外部调用者的对象。强杀不保证 finally。

最终清理释放本宿主引用，不强制关闭其他宿主的租约。部分初始化成功后匹配预期错误时保留成功请求，允许后续检查/显式恢复，最后统一清理；不承诺跨配置事务回滚。需要业务关闭参数时显式 `CloseDevice(...)`，自动清理不保证设备完成业务复位。

当前最新版ABI10是本次功能与发布验收基准，不要求历史ABI实际DLL测试；已有旧入口兼容分支不构成历史版本验收承诺。

ABI 10 对象流程（含 Config 对象或通道预期）明确使用现有 `platform:1` TB 契约，连同元数据和生命周期；原字符串流程仍用 `platform:0` 默认平台。ABI7–9保留三参数入口，对象流程要求 DLL 默认采用 TB 契约。其他 ABI 在调用前拒绝。不修改框架、宏、ABI 或租约协议，不支持任意自定义平台元数据。

工具可以调用与断言多通道和租约相关返回，但**单个流程不能单独证明跨宿主租约正确**。工具内部测试使用临时 registry 节点、独占文件锁、打开/关闭事件和计数，配合多个 DLL/进程验证共享和最后引用释放；不要求节点项目增加验证文件。无硬件夹具不代替真实设备验收。

## 开发验证

```text
cargo test -p miresidian_testflow_windows --locked
cargo clippy -p miresidian_testflow_windows --all-targets --locked -- -D warnings
python tools/miresidian-testflow-windows/tests/native.py --powershell <PowerShell路径>
```

`TESTFLOW_POWERSHELL` 可指定 cargo test 所用 PowerShell；默认在 Windows 查找 powershell.exe，其他系统查找 pwsh。缺少解释器会明确提示行为测试未运行。原生验证脚本只在临时目录生成独立夹具，依赖已发布的精确框架版本，无 path/git/patch 覆盖。Windows 运行新生成 BAT，其他系统直接运行同一内嵌模板。Linux/PowerShell7原生调用通过不替代 Windows x64/x86 的 PowerShell5.1、BAT、PInvoke 实测；具体证据与未完成项记录在[本次方案](../../plans/2026-11-19-testflow-multichannel.md)。Node skill按实际工具版本查阅本说明，不固定工具语法或随工具版本自动修订。
