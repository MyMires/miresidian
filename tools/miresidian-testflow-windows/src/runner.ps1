<#
.SYNOPSIS
  由框架生成的自包含 BAT 内嵌执行，调用 Miresidian Windows DLL。

.NOTES
  此文件是框架工具的内嵌执行模板，不会写入节点项目。
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$DllPath,

    [Parameter(Position = 1)]
    [string]$Function = 'GetFunctions',

    [Parameter(Position = 2)]
    [string]$Config = '[]',

    [Parameter(Position = 3)]
    [string]$Input = '[]',

    [switch]$AbiVersion,
    [switch]$Pretty,
    [switch]$FromEnvironment,
    [string]$FlowFile,
    [string]$DllName = 'miresidian_serial.dll'
)

$ErrorActionPreference = 'Stop'
# BAT switches the console to UTF-8; use the same encoding for formatted output.
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

# 此模板由框架生成的 BAT 以内嵌方式执行，所有运行参数均来自 BAT 写入的环境变量。
$FromEnvironment = $true

# BAT uses environment variables so cmd.exe cannot consume JSON's double quotes.
if ($FromEnvironment) {
    $DllPath = $env:MIRESIDIAN_DLL_PATH
    $Function = $env:MIRESIDIAN_FUNCTION
    $Config = $env:MIRESIDIAN_CONFIG_JSON
    $Input = $env:MIRESIDIAN_INPUT_JSON
    $DllName = $env:MIRESIDIAN_DLL_NAME
    $FlowFile = $env:MIRESIDIAN_FLOW_FILE
}

if ([string]::IsNullOrWhiteSpace($DllPath)) {
    throw '必须提供 DLL 路径。'
}

if (Test-Path -LiteralPath $DllPath -PathType Container) {
    $architectureDirectory = if ([Environment]::Is64BitProcess) {
        'windows-x64'
    } else {
        'windows-x86'
    }
    if ([string]::IsNullOrWhiteSpace($DllName)) {
        throw 'DLL_PATH 为目录时，必须提供 DLL 文件名。'
    }
    $candidate = Get-ChildItem -LiteralPath $DllPath -Filter $DllName -File -Recurse |
        Where-Object { $_.Directory.Name -eq $architectureDirectory } |
        Select-Object -First 1
    if ($null -eq $candidate) {
        throw "在目录中找不到 $architectureDirectory 的 $DllName：$DllPath"
    }
    $DllPath = $candidate.FullName
}

if (-not (Test-Path -LiteralPath $DllPath -PathType Leaf)) {
    throw "找不到 DLL：$DllPath"
}

# P/Invoke 只能接收原生 Windows 文件系统路径。ProviderPath 会移除 UNC 路径上的 PowerShell Provider 前缀；随后再安全转换为 C# 字符串字面量。
$resolvedDllPath = (Resolve-Path -LiteralPath $DllPath).ProviderPath
$csharpDllPath = $resolvedDllPath.Replace('\', '\\').Replace('"', '\"')
$source = @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class MiresidianDllCaller
{
    [DllImport("$csharpDllPath", EntryPoint = "miresidian_invoke", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern IntPtr InvokeLegacyNative(IntPtr function, IntPtr configJson, IntPtr inputJson);

    [DllImport("$csharpDllPath", EntryPoint = "miresidian_invoke", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern IntPtr InvokeRequestNative(IntPtr requestJson);

    [DllImport("$csharpDllPath", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    public static extern UInt32 miresidian_abi_version();

    private static IntPtr Utf8Pointer(string value)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(value + "\0");
        IntPtr pointer = Marshal.AllocHGlobal(bytes.Length);
        Marshal.Copy(bytes, 0, pointer, bytes.Length);
        return pointer;
    }

    // DLL 在下一次调用前持有返回内容；在这里同步复制，且绝不释放该返回指针。
    private static string CopyUtf8(IntPtr pointer)
    {
        if (pointer == IntPtr.Zero) throw new InvalidOperationException("miresidian_invoke 返回了空指针");
        int length = 0;
        while (Marshal.ReadByte(pointer, length) != 0) length++;
        byte[] bytes = new byte[length];
        Marshal.Copy(pointer, bytes, 0, length);
        return Encoding.UTF8.GetString(bytes);
    }

    public static string InvokeRequest(string requestJson)
    {
        IntPtr pointer = Utf8Pointer(requestJson);
        try { return CopyUtf8(InvokeRequestNative(pointer)); }
        finally { Marshal.FreeHGlobal(pointer); }
    }

    public static string InvokeLegacy(string function, string configJson, string inputJson)
    {
        IntPtr functionPointer = IntPtr.Zero;
        IntPtr configPointer = IntPtr.Zero;
        IntPtr inputPointer = IntPtr.Zero;
        try
        {
            functionPointer = Utf8Pointer(function);
            configPointer = Utf8Pointer(configJson);
            inputPointer = Utf8Pointer(inputJson);
            return CopyUtf8(InvokeLegacyNative(functionPointer, configPointer, inputPointer));
        }
        finally
        {
            if (functionPointer != IntPtr.Zero) Marshal.FreeHGlobal(functionPointer);
            if (configPointer != IntPtr.Zero) Marshal.FreeHGlobal(configPointer);
            if (inputPointer != IntPtr.Zero) Marshal.FreeHGlobal(inputPointer);
        }
    }
}
"@

function Invoke-NodeRequest([string]$Function, [string]$ConfigJson, [string]$InputJson) {
    # 同名导出在 ABI v10 改为单参数，必须先核对版本再选择函数指针签名。
    $abi = [MiresidianDllCaller]::miresidian_abi_version()
    if ($abi -eq 10) {
        $request = [ordered]@{
            platform = $(if ($null -eq $script:FlowPlatform) { 0 } else { $script:FlowPlatform })
            function = $Function
            config = @(ConvertFrom-Json -InputObject $ConfigJson)
            input = @(ConvertFrom-Json -InputObject $InputJson)
        }
        return [MiresidianDllCaller]::InvokeRequest((ConvertTo-Json -InputObject $request -Depth 32 -Compress))
    }
    if ($abi -ge 7 -and $abi -le 9) {
        return [MiresidianDllCaller]::InvokeLegacy($Function, $ConfigJson, $InputJson)
    }
    throw "不支持 ABI $abi，未调用不匹配的函数签名。"
}

function Assert-StringArray([string]$Json, [string]$Name) {
    # The ABI accepts only JSON arrays whose every element is a string.
    # Keep the validated source text so its UTF-8 JSON representation is passed unchanged.
    if (-not $Json.TrimStart().StartsWith('[')) {
        throw "$Name 必须是 JSON 字符串数组。"
    }
    $values = ConvertFrom-Json -InputObject $Json
    foreach ($value in @($values)) {
        if ($value -isnot [string]) {
            throw "$Name 必须是 JSON 字符串数组。"
        }
    }
}

function Format-Response([string]$Response) {
    # Pretty mode is intentionally a presentation layer: it does not alter the DLL response.
    Write-Output ('原始响应：' + $Response)
    $payload = $Response | ConvertFrom-Json
    $status = [int]$payload.status
    $statusText = switch ($status) {
        0 { '成功' }
        1 { '参数无效' }
        2 { '操作失败' }
        3 { '内部错误' }
        default { '未知状态' }
    }
    $message = [string]$payload.message
    if ([string]::IsNullOrWhiteSpace($message)) {
        $message = '（无）'
    }
    $code = [string]$payload.code
    if ([string]::IsNullOrWhiteSpace($code)) { $code = '（旧版响应未提供）' }
    $outputItems = @($payload.output)

    Write-Output ''
    Write-Output '==================== DLL 调用结果 ===================='
    Write-Output ('状态 : {0} ({1})' -f $statusText, $status)
    Write-Output ('代码 : {0}' -f $code)
    Write-Output ('消息 : {0}' -f $message)
    Write-Output ('输出 : {0} 项' -f $outputItems.Count)
    if ($outputItems.Count -eq 0) {
        Write-Output '  （无）'
    } else {
        for ($index = 0; $index -lt $outputItems.Count; $index++) {
            Write-Output ('  [{0}] {1}' -f ($index + 1), [string]$outputItems[$index])
        }
    }
    Write-Output '========================================================'
}

function Test-ResponseFailure($Payload) {
    return ([int]$Payload.status -ne 0 -or ($null -ne $Payload.PSObject.Properties['code'] -and [string]$Payload.code -cne 'OK'))
}

function Get-SuccessPayload([string]$Response, [string]$Context) {
    try { $payload = $Response | ConvertFrom-Json } catch { throw "$Context 返回了无法解析的 JSON：$($_.Exception.Message)" }
    if ($null -eq $payload.status) { throw "$Context 返回 JSON 中缺少 status。" }
    if (Test-ResponseFailure $payload) {
        $message = [string]$payload.message
        if ([string]::IsNullOrWhiteSpace($message)) { $message = '（无错误说明）' }
        $classification = "status=$($payload.status)"
        if (-not [string]::IsNullOrWhiteSpace([string]$payload.code)) {
            $classification += ", code=$($payload.code)"
        }
        throw "$Context 失败（$classification）：$message"
    }
    return $payload
}

function Get-Definitions([string]$Definitions) {
    if ([string]::IsNullOrWhiteSpace($Definitions)) { return @() }
    return @($Definitions.Split(',') | ForEach-Object { $_.Trim() })
}

function Get-FlowMethodDefinitions {
    $response = Invoke-NodeRequest 'GetFunctions' '[]' '[]'
    Format-Response $response | Out-Host
    $payload = Get-SuccessPayload $response 'GetFunctions'
    $methods = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    foreach ($signature in @($payload.output)) {
        if ([string]$signature -notmatch '^(?<name>[^()]+)\((?<input>.*)\)\((?<output>.*)\)$') { continue }
        $methods[$matches.name] = Get-Definitions $matches.input
    }
    if ($methods.Count -eq 0) { throw 'GetFunctions 未返回可识别的方法签名。' }
    return $methods
}

function Get-DefaultConfigValue([string]$Definition) {
    $separator = $Definition.IndexOf(':')
    if ($separator -lt 0) { return $null }
    $suffix = $Definition.Substring($separator + 1)
    foreach ($choice in $suffix.Split('/')) {
        $dash = $choice.IndexOf('-')
        if ($dash -gt 0 -and $dash -lt ($choice.Length - 1)) {
            return $choice.Substring(0, $dash).Trim()
        }
    }
    return $suffix
}

function Complete-ConfigArguments([object[]]$Arguments, [string[]]$Definitions, [int]$LineNumber) {
    if ($Arguments.Count -gt $Definitions.Count) { throw "流程第 $LineNumber 行 Config 参数过多：期望至多 $($Definitions.Count) 个，实际 $($Arguments.Count) 个。" }
    $completed = [System.Collections.Generic.List[string]]::new()
    foreach ($argument in $Arguments) { $completed.Add($argument) }
    for ($index = $Arguments.Count; $index -lt $Definitions.Count; $index++) {
        $definition = $Definitions[$index]
        $defaultValue = Get-DefaultConfigValue $definition
        if ($null -eq $defaultValue) { throw "流程第 $LineNumber 行 Config 缺少必填参数：$definition" }
        $completed.Add($defaultValue)
    }
    return @($completed)
}

function Convert-FlowJson([string]$Text) {
    # 检查解码后的键，不能依赖 ConvertFrom-Json 对重复键的覆盖行为。
    $frames = [Collections.Generic.List[object]]::new()
    for ($i = 0; $i -lt $Text.Length; $i++) {
        $c = $Text[$i]
        if ($c -eq '{') { $frames.Add([Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)); continue }
        if ($c -eq '[') { $frames.Add($null); continue }
        if ($c -eq '}' -or $c -eq ']') {
            if ($frames.Count -eq 0) { throw 'JSON 括号不匹配。' }
            $frames.RemoveAt($frames.Count - 1); continue
        }
        if ($c -ne '"') { continue }
        $start = $i
        $i++
        while ($i -lt $Text.Length) {
            if ([int][char]$Text[$i] -lt 32) { throw 'JSON 字符串不允许未转义控制字符。' }
            if ($Text[$i] -eq '\') {
                $i++
                if ($i -ge $Text.Length) { throw 'JSON 转义未结束。' }
                if ($Text[$i] -ceq 'u') {
                    if ($i + 4 -ge $Text.Length -or $Text.Substring($i+1,4) -cnotmatch '^[0-9a-fA-F]{4}$') { throw '无效 Unicode 转义。' }
                    $unit = [Convert]::ToInt32($Text.Substring($i+1,4),16)
                    $i += 4
                    if ($unit -ge 0xd800 -and $unit -le 0xdbff) {
                        if ($i+6 -ge $Text.Length -or $Text.Substring($i+1,2) -cne '\u' -or $Text.Substring($i+3,4) -cnotmatch '^[dD][c-fC-F][0-9a-fA-F]{2}$') { throw 'Unicode 高代理项必须配对。' }
                        $i += 6
                    } elseif ($unit -ge 0xdc00 -and $unit -le 0xdfff) { throw 'Unicode 低代理项缺少高代理项。' }
                } elseif (-not '"/bfnrt'.Contains([string]$Text[$i])) { throw '无效 JSON 转义。' }
                $i++; continue
            }
            if ($Text[$i] -eq '"') { break }
            $i++
        }
        if ($i -ge $Text.Length) { throw 'JSON 字符串未结束。' }
        $next = $i + 1
        while ($next -lt $Text.Length -and [char]::IsWhiteSpace($Text[$next])) { $next++ }
        if ($next -lt $Text.Length -and $Text[$next] -eq ':') {
            $key = ConvertFrom-Json -InputObject ('[' + $Text.Substring($start, $i - $start + 1) + ']')
            if ($frames.Count -gt 0 -and $null -ne $frames[$frames.Count - 1]) {
                if (-not $frames[$frames.Count - 1].Add([string]$key)) { throw "重复 JSON 键：$key" }
            }
        }
    }
    # PowerShell 5.1 的 JSON 解析器可能接受尾逗号，先在字符串外拒绝。
    $outside = [regex]::Replace($Text, '"(?:[^"\\]|\\.)*"', '""')
    if ($outside -match ',\s*[}\]]') { throw 'JSON 不允许尾逗号。' }
    # .NET 各版本的 JSON 宽容规则不同：仅允许标准 JSON 词法。
    $token = [regex]'\G(?:[ \t\r\n]+|""|[{}\[\]:,]|(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)(?![\w.+-]))'
    $compact = [regex]::Replace($outside,'[ \t\r\n]','')
    if ($compact -match '(?<!"):') { throw 'JSON 对象键必须使用双引号。' }
    $position=0
    while ($position -lt $outside.Length) {
        $match=$token.Match($outside,$position)
        if (-not $match.Success) { throw '无效 JSON 词法；不允许注释、单引号或非标准数字。' }
        $position += $match.Length
    }
    return [PSCustomObject]@{ Value = (ConvertFrom-Json -InputObject ('{"value":' + $Text + '}')).value }
}

function Get-FlowStatements([string]$Text) {
    $buffer = [Text.StringBuilder]::new()
    $stack = [Collections.Generic.Stack[char]]::new()
    $quoted = $false; $escaped = $false; $start = 0; $lineNumber = 0
    foreach ($line in ($Text.TrimStart([char]0xfeff) -split '\r?\n')) {
        $lineNumber++
        $trimmed = $line.Trim()
        if (-not $quoted -and ($trimmed.StartsWith('#') -or $trimmed.StartsWith('//'))) { continue }
        if ($buffer.Length -eq 0 -and $trimmed.Length -eq 0) { continue }
        if ($buffer.Length -eq 0) { $start = $lineNumber }
        foreach ($c in $line.ToCharArray()) {
            if ($quoted) {
                if ($escaped) { $escaped = $false }
                elseif ($c -eq '\') { $escaped = $true }
                elseif ($c -eq '"') { $quoted = $false }
            } else {
                if ($c -eq '"') { $quoted = $true }
                elseif ('([{'.Contains([string]$c)) { $stack.Push($c) }
                elseif (')]}'.Contains([string]$c)) {
                    $opening = '([{'[')]}'.IndexOf($c)]
                    if ($stack.Count -eq 0 -or $stack.Pop() -ne $opening) { throw "流程第 $lineNumber 行括号不匹配。" }
                }
            }
        }
        [void]$buffer.AppendLine($line)
        if (-not $quoted -and $stack.Count -eq 0) {
            [PSCustomObject]@{ Text = $buffer.ToString(); Line = $start; EndLine = $lineNumber }
            [void]$buffer.Clear()
        }
    }
    if ($buffer.Length -ne 0) { throw "流程第 $start 行语句未结束。" }
}

function Get-ChannelNumber([string]$Text) {
    if ($Text -cnotmatch '^ch[1-9][0-9]*$') { throw "无效通道：$Text" }
    $number = 0
    if (-not [int]::TryParse($Text.Substring(2), [ref]$number) -or $number -gt 256) { throw '最多支持 256 通道。' }
    return $number
}

function Expand-ChannelSelector([string]$Text) {
    if ($Text.Contains('-')) {
        $parts = $Text.Split('-')
        if ($parts.Count -ne 2) { throw "无效通道范围：$Text" }
        $first = Get-ChannelNumber $parts[0]; $last = Get-ChannelNumber $parts[1]
        if ($first -gt $last) { throw '通道范围必须升序。' }
        return $first..$last
    }
    foreach ($part in $Text.Split(',')) { Get-ChannelNumber $part }
}

function Assert-FlowStrings($Values) {
    if ($Values -isnot [array]) { throw '参数必须为字符串数组。' }
    foreach ($v in $Values) { if ($v -isnot [string]) { throw '参数必须全部为字符串。' } }
}

function Expand-ChannelMap($Object, [switch]$ConfigMap, [switch]$Expected) {
    if ($Object -isnot [PSCustomObject]) { throw '通道映射必须为对象。' }
    $map = @{}
    foreach ($p in $Object.PSObject.Properties) {
        if ($Expected -and $p.Value -is [string]) { }
        elseif ($null -eq $p.Value -and -not $ConfigMap -and -not $Expected) { }
        else { Assert-FlowStrings $p.Value }
        foreach ($n in @(Expand-ChannelSelector $p.Name)) {
            if ($map.ContainsKey($n)) { throw "重叠通道：ch$n" }
            $map[$n] = $p.Value
        }
    }
    if ($map.Count -eq 0) { throw '通道对象不能为空。' }
    return $map
}

function Parse-FlowStatement($Statement) {
    $text = $Statement.Text.Trim()
    $open = $text.IndexOf('(')
    if ($open -lt 1) { throw '缺少方法调用括号。' }
    $name = $text.Substring(0, $open).Trim()
    if ($name -match '[\s(){}\[\]]') { throw '无效方法名。' }
    $quoted = $false; $escaped = $false; $depth = 1; $close = -1
    for ($i = $open + 1; $i -lt $text.Length; $i++) {
        $c = $text[$i]
        if ($quoted) {
            if ($escaped) { $escaped = $false }
            elseif ($c -eq '\') { $escaped = $true }
            elseif ($c -eq '"') { $quoted = $false }
        } else {
            if ($c -eq '"') { $quoted = $true }
            elseif ($c -eq '(') { $depth++ }
            elseif ($c -eq ')') { $depth--; if ($depth -eq 0) { $close = $i; break } }
        }
    }
    if ($close -lt 0) { throw '缺少结束括号。' }
    $values = (Convert-FlowJson ('[' + $text.Substring($open + 1, $close - $open - 1) + ']')).Value
    $mapped = $values.Count -eq 1 -and $values[0] -is [PSCustomObject]
    $channels = $null
    if ($mapped) { $channels = Expand-ChannelMap $values[0] -ConfigMap:($name -ieq 'Config') }
    else { Assert-FlowStrings $values }
    $tail = $text.Substring($close + 1).Trim()
    $expected = $null; $expectedChannels = $null
    if ($tail.Length -ne 0) {
        if (-not $tail.StartsWith('=>')) { throw '调用后只允许 => 预期对象。' }
        if ($name -ieq 'Config' -or $name -ieq 'Delay') { throw 'Config/Delay 不是 DLL 断言目标。' }
        $expected = (Convert-FlowJson $tail.Substring(2)).Value
        if ($expected -isnot [PSCustomObject]) { throw '预期必须为对象。' }
        if ($null -eq $expected.PSObject.Properties['status'] -and $null -eq $expected.PSObject.Properties['code']) { throw '预期必须至少包含 status 或 code。' }
        foreach ($p in $expected.PSObject.Properties) {
            switch -CaseSensitive ($p.Name) {
                'status' { if ($p.Value -isnot [int] -and $p.Value -isnot [long]) { throw 'status 必须为整数。' } }
                'code' { if ($p.Value -isnot [string]) { throw 'code 必须为字符串。' } }
                'output' {
                    if ($p.Value -is [PSCustomObject]) { $expectedChannels = Expand-ChannelMap $p.Value -Expected }
                    else { Assert-FlowStrings $p.Value }
                }
                default { throw "无效预期字段：$($p.Name)" }
            }
        }
    }
    return [PSCustomObject]@{ Name=$name; Arguments=@($values); Mapped=$mapped; Channels=$channels; Expected=$expected; ExpectedChannels=$expectedChannels; LineNumber=$Statement.Line; EndLine=$Statement.EndLine }
}

function Test-ExactArray($Left, $Right) {
    if (@($Left).Count -ne @($Right).Count) { return $false }
    for ($i=0; $i -lt @($Left).Count; $i++) { if ($Left[$i] -cne $Right[$i]) { return $false } }
    return $true
}

function Assert-FlowExpected($Call, $Actual, $Outputs, [bool]$Batch) {
    $expected = $Call.Expected
    if ($null -eq $expected) {
        if ($null -ne $Actual -and (Test-ResponseFailure $Actual)) { throw "DLL 执行失败：status=$($Actual.status), code=$($Actual.code), message=$($Actual.message)" }
        return
    }
    Write-Output ('预期：' + (ConvertTo-Json -InputObject $expected -Depth 32 -Compress))
    if ($null -ne $Actual -and (Test-ResponseFailure $Actual)) {
        $explicitFailure = ($null -ne $expected.PSObject.Properties['status'] -and $expected.status -ne 0) -or ($null -ne $expected.PSObject.Properties['code'] -and $expected.code -cne 'OK')
        if (-not $explicitFailure) { throw '实际执行失败，预期未明确声明失败状态或错误代码。' }
    }
    if ($null -eq $Actual) { throw '没有实际 DLL 调用，不能断言响应 status/code。' }
    foreach ($field in @('status','code')) {
        if ($null -ne $expected.PSObject.Properties[$field]) {
            if ($null -eq $Actual.PSObject.Properties[$field] -or $Actual.$field -cne $expected.$field) { throw "断言失败：$field 预期=$($expected.$field)，实际=$($Actual.$field)" }
        }
    }
    if ($Batch) { Write-Output '本次通道 output 按 TB 编码文本比较；不反向拆分返回字段。' }
    if ($null -ne $expected.PSObject.Properties['output']) {
        if ($null -ne $Call.ExpectedChannels) {
            foreach ($n in @($Call.ExpectedChannels.Keys | Sort-Object)) {
                if (-not $Outputs.ContainsKey($n)) { throw "断言失败：ch$n 没有返回槽位。" }
                $want = $Call.ExpectedChannels[$n]; $got = $Outputs[$n]
                if ($Batch -or $want -is [string]) {
                    if (($want -join ',') -cne ($got -join ',')) { throw "断言失败：ch$n 预期=$($want -join ',')，实际=$($got -join ',')" }
                } elseif (-not (Test-ExactArray $want $got)) {
                    throw "断言失败：ch$n 预期=$(ConvertTo-Json -InputObject $want -Compress)，实际=$(ConvertTo-Json -InputObject $got -Compress)"
                }
            }
        } else {
            if ($Call.Mapped -and ($Batch -or $Outputs.Count -gt 1)) { throw '多通道 output 预期必须使用通道映射。' }
            if (-not (Test-ExactArray $expected.output @($Actual.output))) { throw "断言失败：output 预期=$(ConvertTo-Json -InputObject $expected.output -Compress)，实际=$(ConvertTo-Json -InputObject @($Actual.output) -Compress)" }
        }
    }
    if (Test-ResponseFailure $Actual) { Write-Output '实际执行失败；符合预期，断言通过。' } else { Write-Output '断言通过。' }
}

function Invoke-Flow([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "找不到流程文件：$Path" }
    $resolvedPath = (Resolve-Path -LiteralPath $Path).ProviderPath
    $statements = @(Get-FlowStatements ([IO.File]::ReadAllText($resolvedPath, [Text.Encoding]::UTF8)))
    if ($statements.Count -eq 0) { throw '流程至少需要一个方法调用。' }
    # 全流程先解析；对象模式的元数据和生命周期也明确走 TB 平台。
    $calls = @($statements | ForEach-Object {
        $sourceStatement = $_
        try { Parse-FlowStatement $sourceStatement } catch { throw "流程第 $($sourceStatement.Line)-$($sourceStatement.EndLine) 行解析失败：$($_.Exception.Message)" }
    })
    $script:FlowPlatform = 0
    if (@($calls | Where-Object { $_.Mapped -or $null -ne $_.ExpectedChannels }).Count -gt 0) { $script:FlowPlatform = 1 }
    $selected = $null; $shared = $true; $initialized = [Collections.Generic.Dictionary[string,bool]]::new([StringComparer]::Ordinal); $mainFailure = $null; $cleanupFailure = $null
    try {
        $methods = Get-FlowMethodDefinitions
        $response = Invoke-NodeRequest 'GetInitConfig' '[]' '[]'
        Format-Response $response
        $definitions = @((Get-SuccessPayload $response 'GetInitConfig').output)
        foreach ($call in $calls) {
            Write-Output ("流程第 {0}-{1} 行：{2}" -f $call.LineNumber, $call.EndLine, $call.Name)
            if ($call.Name -ieq 'Delay') {
                if ($call.Mapped -or $call.Arguments.Count -ne 1 -or $call.Arguments[0] -cnotmatch '^[0-9]+$') { throw 'Delay 必须接收一个毫秒数字符串。' }
                $ms = 0
                if (-not [int]::TryParse($call.Arguments[0], [ref]$ms)) { throw 'Delay 超出 0..2147483647 范围。' }
                Start-Sleep -Milliseconds $ms; continue
            }
            if ($call.Name -ieq 'Config') {
                $replacement = @{}
                if ($call.Mapped) {
                    foreach ($n in $call.Channels.Keys) { $replacement[$n] = @(Complete-ConfigArguments $call.Channels[$n] $definitions $call.LineNumber) }
                } else { $replacement[1] = @(Complete-ConfigArguments $call.Arguments $definitions $call.LineNumber) }
                $selected = $replacement; $shared = -not $call.Mapped
                foreach ($n in @($selected.Keys | Sort-Object)) { Write-Output ('Config ch{0}: {1}' -f $n, (ConvertTo-Json -InputObject @($selected[$n]) -Compress)) }; continue
            }
            if (-not $methods.ContainsKey($call.Name)) { throw "DLL 未声明方法：$($call.Name)" }
            $lifecycle = $call.Name -cin @('InitDevice','CloseDevice')
            $global = $call.Name -cin @('GetFunctions','GetFrameworkVersion','GetProjectVersion','GetInitConfig','GetPortNO','CloseAllDevices')
            if (-not $global -and $null -eq $selected) { throw '调用前必须先 Config。' }
            $slots = @{}
            if ($global) {
                if ($call.Mapped) { throw '全局方法不接受通道映射。' }
                $slots[1] = @($call.Arguments)
            } elseif ($call.Mapped) { $slots = $call.Channels }
            elseif ($lifecycle -and -not $shared) {
                if ($null -eq $selected) { throw '必须先 Config。' }
                foreach ($n in $selected.Keys) { $slots[$n] = @($call.Arguments) }
            } else { $slots[1] = @($call.Arguments) }
            $highest = [int](@($slots.Keys | Sort-Object)[-1])
            $requests = [Collections.Generic.List[object]]::new()
            $uniqueLifecycle = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
            $configs = @{}
            foreach ($n in @($slots.Keys | Sort-Object)) {
                if ($null -eq $slots[$n]) { continue }
                $cfg = @()
                if (-not $global) {
                    if ($null -eq $selected) { throw '调用前必须先 Config。' }
                    $configChannel = if ($shared) { 1 } else { $n }
                    if (-not $selected.ContainsKey($configChannel)) { throw "ch$n 缺少 Config。" }
                    $cfg = @($selected[$configChannel])
                }
                $key = ConvertTo-Json -InputObject $cfg -Compress
                $configs[$n] = $key
                if (-not $global -and -not $lifecycle -and -not $initialized.ContainsKey($key)) { throw "ch$n 所选 Config 尚未成功 InitDevice。" }
                $inputKey = ConvertTo-Json -InputObject @($slots[$n]) -Compress
                if ($lifecycle -and $uniqueLifecycle.ContainsKey($key)) {
                    if ($uniqueLifecycle[$key].Input -cne $inputKey) { throw '同一 Config 的生命周期 Input 冲突，未调用本行 DLL。' }
                    [void]$uniqueLifecycle[$key].Channels.Add($n); continue
                }
                $request = [PSCustomObject]@{ Config=$key; Input=$inputKey; Channels=[Collections.Generic.List[int]]::new(); Values=@($slots[$n]) }
                $request.Channels.Add($n)
                $requests.Add($request)
                if ($lifecycle) { $uniqueLifecycle[$key] = $request }
            }
            $distinctConfigs = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
            foreach ($key in $configs.Values) { [void]$distinctConfigs.Add($key) }
            $batch = -not $global -and -not $lifecycle -and $call.Mapped -and $highest -gt 1 -and ($distinctConfigs.Count -le 1) -and ($requests.Count -gt 0 -or $shared)
            if ($batch) {
                $batchInput = [Collections.Generic.List[string]]::new(); $batchInput.Add('共享')
                for ($n=1; $n -le $highest; $n++) {
                    if (-not $slots.ContainsKey($n) -or $null -eq $slots[$n]) { $batchInput.Add(''); continue }
                    $values = @($slots[$n])
                    if ($values.Count -eq 0) { throw "ch$n 的零参数调用无法用 TB 多通道编码。" }
                    foreach ($v in $values) {
                        if ($v.Length -eq 0 -or $v.Contains(',') -or $v -cne $v.Trim()) { throw "ch$n 参数无法无损编码为 TB 多通道输入。" }
                    }
                    $batchInput.Add(($values -join ','))
                }
                $batchConfig = if ($requests.Count -gt 0) { $requests[0].Config } else {
                    if ($null -eq $selected) { throw '必须先 Config。' }
                    ConvertTo-Json -InputObject @($selected[1]) -Compress
                }
                $requests.Clear()
                $request = [PSCustomObject]@{ Config=$batchConfig; Input=(ConvertTo-Json -InputObject @($batchInput) -Compress); Channels=@(); Values=@() }
                $requests.Add($request)
            } else {
                foreach ($request in $requests) {
                    if ($request.Values.Count -gt 0 -and $request.Values[0] -ceq '共享') { throw '普通调用首参数“共享”为 TB 保留标记。' }
                }
            }
            $outputs = @{}; $actual = $null; $failed = $false
            if ($call.Mapped) {
                for ($n=1; $n -le $highest; $n++) {
                    $state = if (-not $slots.ContainsKey($n) -or $null -eq $slots[$n]) { 'skipped' } else { 'not_started' }
                    Write-Output ("ch{0}: {1}" -f $n, $state)
                }
            }
            foreach ($request in $requests) {
                $submittedChannels = if ($batch) { @($slots.Keys | Where-Object { $null -ne $slots[$_] } | Sort-Object) } else { @($request.Channels) }
                foreach ($n in $submittedChannels) { Write-Output ("ch{0}: submitted" -f $n) }
                try { $response = Invoke-NodeRequest $call.Name $request.Config $request.Input }
                catch {
                    foreach ($n in $submittedChannels) { Write-Output ("ch{0}: submitted_result_unknown" -f $n) }
                    throw
                }
                # 原始响应先展示：错误、多个返回参数、未知字段都不能因断言被隐藏。
                Format-Response $response
                $payload = (Convert-FlowJson $response).Value
                if ($null -eq $payload.status) { throw 'DLL 响应缺少 status。' }
                if ($null -eq $actual -or (Test-ResponseFailure $payload)) { $actual = $payload }
                if ($batch) {
                    $returned = @($payload.output)
                    for ($n=1; $n -le $returned.Count; $n++) { $outputs[$n] = [string]$returned[$n-1]; Write-Output ("ch{0}: returned {1}" -f $n, $returned[$n-1]) }
                    foreach ($n in $submittedChannels) {
                        if (-not $outputs.ContainsKey($n)) { Write-Output ("ch{0}: submitted_no_result（不能据此判断无副作用）" -f $n) }
                    }
                } else {
                    foreach ($n in $request.Channels) { $outputs[$n] = @($payload.output); Write-Output ("ch{0}: returned {1}" -f $n, (ConvertTo-Json -InputObject @($payload.output) -Compress)) }
                }
                if (-not (Test-ResponseFailure $payload)) {
                    if ($call.Name -ceq 'InitDevice') { $initialized[$request.Config] = $true }
                    elseif ($call.Name -ceq 'CloseDevice') { [void]$initialized.Remove($request.Config) }
                    elseif ($call.Name -ceq 'CloseAllDevices') { $initialized.Clear() }
                } else { $failed = $true; break }
            }
            if ($failed) { Write-Output '本行后续独立调用未开始；已提交请求的通道状态以 DLL 响应为准。' }
            Assert-FlowExpected $call $actual $outputs $batch
        }
    } catch {
        $location = if ($null -ne $call) { "流程第 $($call.LineNumber)-$($call.EndLine) 行：" } else { "流程准备阶段：" }
        $mainFailure = $location + $_.Exception.Message
    }
    finally {
        try {
            Write-Output '流程结束：释放本 DLL runtime 的全部设备引用。'
            $response = Invoke-NodeRequest 'CloseAllDevices' '[]' '[]'
            Format-Response $response
            Get-SuccessPayload $response '自动 CloseAllDevices' | Out-Null
        } catch { $cleanupFailure = $_.Exception.Message }
    }
    if ($null -ne $mainFailure -or $null -ne $cleanupFailure) {
        throw "流程错误：$mainFailure；清理错误：$cleanupFailure"
    }
}

try {
    Add-Type -TypeDefinition $source -ErrorAction Stop
    if ($AbiVersion) {
        [MiresidianDllCaller]::miresidian_abi_version()
        exit 0
    }

    if (-not [string]::IsNullOrWhiteSpace($FlowFile)) {
        Invoke-Flow $FlowFile
        exit 0
    }

    # 尽早报告命令行 JSON 传参错误；实际文本仍原样交给 DLL。
    Assert-StringArray $Config 'config'
    Assert-StringArray $Input 'input'

    $result = Invoke-NodeRequest $Function $Config $Input
    if ($Pretty) {
        Format-Response $result
    } else {
        $result
    }
} catch {
    [Console]::Error.WriteLine("调用失败：$($_.Exception.Message)")
    exit 1
}
