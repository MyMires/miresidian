# 内部验证脚本；不生成到用户节点项目。兼容 Windows PowerShell 5.1。
param([string]$Runner = "$PSScriptRoot/../src/runner.ps1")
$ErrorActionPreference = 'Stop'
$tokens=$null; $errors=$null
$ast = [Management.Automation.Language.Parser]::ParseInput([IO.File]::ReadAllText((Resolve-Path $Runner).ProviderPath,[Text.Encoding]::UTF8), [ref]$tokens, [ref]$errors)
if ($errors.Count -gt 0) { throw ($errors | Out-String) }
foreach ($function in $ast.FindAll({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] }, $false)) {
    . ([scriptblock]::Create($function.Extent.Text))
}
function Check([bool]$Ok, [string]$Message) { if (-not $Ok) { throw $Message } }
function Reject([scriptblock]$Action, [string]$Message) {
    $rejected=$false
    try { & $Action | Out-Null } catch { $rejected=$true }
    Check $rejected $Message
}
# Verify ABI dispatch against a strict host stand-in, independently of flow mocks.
Add-Type -TypeDefinition @'
public static class MiresidianDllCaller {
    public static uint Abi = 10;
    public static int RequestCalls = 0, LegacyCalls = 0;
    public static string Request, Config, Input;
    public static uint miresidian_abi_version() { return Abi; }
    public static string InvokeRequest(string request) { RequestCalls++; Request=request; return "{}"; }
    public static string InvokeLegacy(string method, string config, string input) { LegacyCalls++; Config=config; Input=input; return "{}"; }
}
'@
$script:FlowPlatform=1
Invoke-NodeRequest 'Echo' '["a"]' '["x"]' | Out-Null
$req=(Convert-FlowJson ([MiresidianDllCaller]::Request)).Value
Check ($req.platform -eq 1 -and $req.function -ceq 'Echo') 'ABI10 应使用单 JSON TB 入口'
foreach ($abi in @(7,8,9)) {
    [MiresidianDllCaller]::Abi=$abi
    Invoke-NodeRequest 'Echo' '["a"]' '["x"]' | Out-Null
}
Check ([MiresidianDllCaller]::LegacyCalls -eq 3 -and [MiresidianDllCaller]::RequestCalls -eq 1) 'ABI7-9只能走旧签名'
[MiresidianDllCaller]::Abi=11
Reject { Invoke-NodeRequest 'Echo' '[]' '[]' } '未知 ABI 必须在 native invoke 前拒绝'
Check ([MiresidianDllCaller]::LegacyCalls -eq 3 -and [MiresidianDllCaller]::RequestCalls -eq 1) '未知 ABI 不能错误调用'
Check (Test-ResponseFailure ([PSCustomObject]@{status=0;code='OPERATION_FAILED'})) '实际非 OK code 必须判为执行失败'
Check (-not (Test-ResponseFailure ([PSCustomObject]@{status=0}))) '旧响应不伪造 code'
$script:Native = [Collections.Generic.List[object]]::new()
$script:FailInput = $null
$script:CleanupFails = $false
$script:FailInitConfig = $null
function Invoke-NodeRequest([string]$Function,[string]$ConfigJson,[string]$InputJson) {
    $cfg=(Convert-FlowJson $ConfigJson).Value; $values=(Convert-FlowJson $InputJson).Value
    $script:Native.Add([PSCustomObject]@{Name=$Function;Config=@($cfg);Input=@($values);Platform=$script:FlowPlatform})
    $output=@(); $status=0; $code='OK'; $message=''
    switch -CaseSensitive ($Function) {
        'GetFunctions' { $output=@('InitDevice(resource,Handle)(result)','CloseDevice()()','CloseAllDevices()()','Echo(text)(first,second)','GetFunctions()()','GetInitConfig()()') }
        'GetInitConfig' { $output=@('resource','Handle:default') }
        'Echo' {
            if ($values.Count -gt 0 -and $values[0] -ceq '共享') {
                $output=@($values | Select-Object -Skip 1 | ForEach-Object { if ($_ -eq '') { '' } else { "$_,two" } })
            } else { $output=@($values) + @('two') }
        }
    }
    if ($Function -eq 'Echo' -and $values.Count -gt 0 -and $values[0] -ceq $script:FailInput) { $status=2; $code='OPERATION_FAILED'; $message='test failure'; $output=@('failure-output','details') }
    if ($Function -eq 'InitDevice' -and $cfg.Count -gt 0 -and $cfg[0] -ceq $script:FailInitConfig) { $status=2; $code='OPERATION_FAILED' }
    if ($Function -eq 'CloseAllDevices' -and $script:CleanupFails) { $status=2; $code='OPERATION_FAILED' }
    return ConvertTo-Json -InputObject @{status=$status;code=$code;message=$message;output=@($output)} -Depth 32 -Compress
}
function Run([string]$Text) {
    $script:Native.Clear()
    $path=Join-Path ([IO.Path]::GetTempPath()) ('testflow-' + [Guid]::NewGuid().ToString('N') + '.testflow')
    try { [IO.File]::WriteAllText($path,$Text,[Text.UTF8Encoding]::new($false)); Invoke-Flow $path }
    finally { Remove-Item -LiteralPath $path -Force }
}
foreach ($bad in @('{"x":1,"x":2}','{"ch1":[],"\u0063h1":[]}','{"x":[],}','{"x":/*comment*/1}',"{'x':1}",'{"x":01}','{true:1}','{"x":NaN}','{"x":+1}','{"x":"\v"}','{"x":"\U0061"}','{"x":"\ud800"}','{"x":"\udc00"}')) {
    Reject { Convert-FlowJson $bad } "JSON 应拒绝：$bad"
}
foreach ($bad in @('ch0','ch01','ch257','ch1, ch3','ch3-ch1','ch1,ch2-ch3','ch99999999999999')) {
    Reject { Expand-ChannelSelector $bad } "通道应拒绝：$bad"
}
Check ((@(Expand-ChannelSelector 'ch1-ch256')).Count -eq 256) '范围上限'
foreach ($bad in @('Echo({})','Config({"ch1":null})','Echo({"ch1":[],"ch1-ch2":[]})','Echo() => {"output":[]}','Echo() => {"code":1}','Config() => {"code":"OK"}')) {
    Reject { $s=@(Get-FlowStatements $bad); Parse-FlowStatement $s[0] } "语法应拒绝：$bad"
}
$text=@'
Config("resource")
InitDevice()
Echo({
  "ch1,ch3":["AA"],
  "ch6":null
}) => {"code":"OK","output":{"ch1,ch3":["AA","two"],"ch6":""}}
'@
$log=@(Run $text)
$echo=@($script:Native | Where-Object Name -eq 'Echo')
Check ($echo.Count -eq 1) '相同 Config 必须一次真实批调用'
Check (Test-ExactArray $echo[0].Input @('共享','AA','','AA','','','')) '空洞和尾 null 必须保留'
Check ($echo[0].Platform -eq 1) '对象模式明确 TB platform=1'
Check (($log -join "`n").Contains('断言通过')) '多输出断言'
Check ($script:Native[$script:Native.Count-1].Name -eq 'CloseAllDevices') '最终关闭全部本地引用'
$log=@(Run "Config({`"ch1,ch3`": [`"resource`",`"h1`"],`"ch2`": [`"resource`",`"h2`"]})`nInitDevice()`nEcho({`"ch1-ch3`":[`"x`"]}) => {`"status`":0,`"output`":{`"ch1-ch3`":[`"x`",`"two`"]}}`nCloseDevice()")
Check (@($script:Native | Where-Object Name -eq 'InitDevice').Count -eq 2) '生命周期相同配置去重'
Check (@($script:Native | Where-Object Name -eq 'CloseDevice').Count -eq 2) '关闭去重'
$echo=@($script:Native | Where-Object Name -eq 'Echo')
Check ($echo.Count -eq 3) '不同 Config 按通道独立调用，不重组批次'
Check (Test-ExactArray @($echo | ForEach-Object {$_.Config[1]}) @('h1','h2','h1')) '数值通道顺序'
$log=@(Run "Config(`"resource`")`nInitDevice()`nEcho(`"one`",`"two,three`",`"`",`" x `")")
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 1) '普通参数不受 TB 编码限制'
Check (-not (($log -join "`n").Contains('断言通过'))) '无箭头为执行模式'
Check (($log -join "`n").Contains('two,three')) '执行模式展示实际输出'
Check (@($script:Native | Where-Object Name -eq 'Echo')[0].Platform -eq 0) '旧普通流程保持默认平台'
foreach ($inputText in @('[]','["a,b"]','[""]','[" x"]')) {
    Reject { Run "Config(`"resource`")`nInitDevice()`nEcho({`"ch2`":$inputText})" } '批次不可编码必须拒绝'
    Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 0) '编码失败不能调用本行业务'
}
Reject { Run "Config({`"ch2`":[`"r`"]})`nInitDevice()`nEcho(`"x`")" } '普通调用不能偷偷选择 ch2'
Reject { Run "Config({`"ch1,ch2`":[`"r`"]})`nInitDevice({`"ch1`":[`"a`"],`"ch2`":[`"b`"]})" } '同配置不同生命周期输入应拒绝'
Check (@($script:Native | Where-Object Name -eq 'InitDevice').Count -eq 0) '生命周期冲突整行预检'
$script:FailInput='bad'
$log=@(Run "Config({`"ch1`":[`"r`",`"h1`"],`"ch2`":[`"r`",`"h2`"]})`nInitDevice()`nEcho({`"ch1`":[`"bad`"],`"ch2`":[`"later`"]}) => {`"code`":`"OPERATION_FAILED`",`"output`":{`"ch1`":[`"failure-output`",`"details`"]}}`nEcho(`"recovery`")")
$echo=@($script:Native | Where-Object Name -eq 'Echo')
Check ($echo.Count -eq 2 -and $echo[1].Input[0] -eq 'recovery') '预期失败后继续，不执行失败行剩余通道或重试'
Check (($log -join "`n").Contains('failure-output')) '失败也输出实际结果'
Reject { Run "Config(`"r`")`nInitDevice()`nEcho(`"bad`")" } '执行模式失败必须非零'
Check ($script:Native[$script:Native.Count-1].Name -eq 'CloseAllDevices') '异常也清理'
$script:FailInput=$null
$log=@(Run "Config(`"r`")`nInitDevice()`nEcho({`"ch1`":null})")
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 0) 'ch1:null 不调用'
Reject { Run "Config(`"r`")`nInitDevice()`nEcho({`"ch1`":null}) => {`"code`":`"OK`"}" } '无调用不能伪造状态'
$log=@(Run "Config(`"r`")`nInitDevice()`nEcho({`"ch3`":null}) => {`"code`":`"OK`",`"output`":{`"ch1-ch3`":`"`"}}")
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 1) '共享配置全 null 仍验证真实 TB 跳过'
$script:CleanupFails=$true
Reject { Run "Config(`"r`")`nInitDevice()" } '清理失败必须非零'
$script:CleanupFails=$false
$log=@(Run "Config({`"ch1`": [`"a`"],`"ch2`": [`"A`"]})`nInitDevice()`nEcho({`"ch1-ch2`":[`"x`"]})")
Check (@($script:Native | Where-Object Name -eq 'InitDevice').Count -eq 2) 'Config 区分大小写，不能错误去重'
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 2) '不同大小写 Config 不合并批次'
$log=@(Run "Config({`"ch1`": [`"a`"],`"ch2`": [`"a`",`"default`"]})`nInitDevice()`nEcho({`"ch1-ch2`":[`"x`"]})")
Check (@($script:Native | Where-Object Name -eq 'InitDevice').Count -eq 1) '补齐后的完整相同 Config 去重'
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 1) '默认项两种写法生成同一批次'
Reject { Run "Config(`"r`")`nInitDevice()`nEcho(`"x`") => {`"code`":`"WRONG`"}" } '验证器必须拒绝错误代码'
Reject { Run "Config(`"r`")`nInitDevice()`nEcho(`"x`") => {`"code`":`"OK`",`"output`":[`"wrong`",`"two`"]}" } '验证器必须拒绝错误输出'
Reject { Run "Config(`"r`")`nInitDevice()`nEcho({`"ch2`":[`"x`"]}) => {`"code`":`"OK`",`"output`":{`"ch3`":`"`"}}" } '不能补造未返回槽位'
$script:FailInitConfig='bad'
$log=@(Run "Config({`"ch1`": [`"good`"],`"ch2`": [`"bad`"],`"ch3`": [`"later`"]})`nInitDevice() => {`"code`":`"OPERATION_FAILED`"}`nEcho(`"alive`") => {`"code`":`"OK`"}")
Check (@($script:Native | Where-Object Name -eq 'InitDevice').Count -eq 2) '部分初始化失败后不继续未提交配置'
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 1) '预期初始化失败保留已成功配置'
Reject { Run "Config({`"ch1`": [`"good`"],`"ch2`": [`"bad`"]})`nInitDevice() => {`"code`":`"OPERATION_FAILED`"}`nEcho({`"ch2`":[`"x`"]})" } '不能把失败初始化记录为成功'
Check (@($script:Native | Where-Object Name -eq 'Echo').Count -eq 0) '未初始化业务在调用前拒绝'
$script:FailInitConfig=$null
$script:FailInput='bad'; $script:CleanupFails=$true
$message=''
try { Run "Config(`"r`")`nInitDevice()`nEcho(`"bad`")" | Out-Null } catch { $message=$_.Exception.Message }
Check ($message.Contains('DLL 执行失败') -and $message.Contains('自动 CloseAllDevices')) '主错误与清理错误均需保留'
$script:FailInput=$null; $script:CleanupFails=$false
Write-Output 'PASS: TestFlow parser, dispatch, expectations, failures and cleanup'
