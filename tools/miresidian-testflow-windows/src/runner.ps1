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
    [DllImport("$csharpDllPath", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern IntPtr miresidian_invoke(IntPtr function, IntPtr configJson, IntPtr inputJson);

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

    public static string Invoke(string function, string configJson, string inputJson)
    {
        IntPtr functionPointer = IntPtr.Zero;
        IntPtr configPointer = IntPtr.Zero;
        IntPtr inputPointer = IntPtr.Zero;
        try
        {
            functionPointer = Utf8Pointer(function);
            configPointer = Utf8Pointer(configJson);
            inputPointer = Utf8Pointer(inputJson);
            return CopyUtf8(miresidian_invoke(functionPointer, configPointer, inputPointer));
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

function Get-SuccessPayload([string]$Response, [string]$Context) {
    try { $payload = $Response | ConvertFrom-Json } catch { throw "$Context 返回了无法解析的 JSON：$($_.Exception.Message)" }
    if ($null -eq $payload.status) { throw "$Context 返回 JSON 中缺少 status。" }
    if ([int]$payload.status -ne 0) {
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
    $response = [MiresidianDllCaller]::Invoke('GetFunctions', '[]', '[]')
    $payload = Get-SuccessPayload $response 'GetFunctions'
    $methods = @{}
    foreach ($signature in @($payload.output)) {
        if ([string]$signature -notmatch '^(?<name>[^()]+)\((?<input>.*)\)\((?<output>.*)\)$') { continue }
        $methods[$matches.name] = Get-Definitions $matches.input
    }
    if ($methods.Count -eq 0) { throw 'GetFunctions 未返回可识别的方法签名。' }
    return $methods
}

function Parse-FlowLine([string]$Line, [int]$LineNumber) {
    $statement = $Line.Trim()
    if ([string]::IsNullOrWhiteSpace($statement) -or $statement.StartsWith('#') -or $statement.StartsWith('//')) { return $null }
    if ($statement -notmatch '^(?<name>[^()\s]+)\s*\((?<arguments>.*)\)\s*$') { throw "流程第 $LineNumber 行不是有效的方法调用：$Line" }
    $arguments = $matches.arguments.Trim()
    try { $values = if ([string]::IsNullOrWhiteSpace($arguments)) { @() } else { @(ConvertFrom-Json -InputObject "[$arguments]") } }
    catch { throw "流程第 $LineNumber 行的参数必须是 JSON 字符串：$($_.Exception.Message)" }
    foreach ($value in $values) {
        if ($value -isnot [string]) { throw "流程第 $LineNumber 行的参数必须全部是字符串。" }
    }
    return [PSCustomObject]@{ Name = $matches.name; Arguments = @($values); LineNumber = $LineNumber }
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

function Invoke-Flow([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "找不到流程文件：$Path" }
    $resolvedPath = (Resolve-Path -LiteralPath $Path).ProviderPath
    $methods = Get-FlowMethodDefinitions
    $configResponse = [MiresidianDllCaller]::Invoke('GetInitConfig', '[]', '[]')
    $configDefinitions = @((Get-SuccessPayload $configResponse 'GetInitConfig').output)
    $selectedConfig = $null
    $initializedConfigs = @{}
    $lines = [System.IO.File]::ReadAllLines($resolvedPath, [System.Text.UTF8Encoding]::new($false))

    try {
        for ($index = 0; $index -lt $lines.Length; $index++) {
            $call = Parse-FlowLine $lines[$index] ($index + 1)
            if ($null -eq $call) { continue }
            if ($call.Name -ieq 'Delay') {
                if ($call.Arguments.Count -ne 1) {
                    throw ('流程第 {0} 行 Delay 必须接收一个毫秒数字符串，例如 Delay("500")。' -f $call.LineNumber)
                }
                $delayText = [string]$call.Arguments[0]
                if ($delayText -notmatch '^\d+$') {
                    throw "流程第 $($call.LineNumber) 行 Delay 的毫秒数必须是非负整数：$delayText"
                }
                try {
                    $milliseconds = [Convert]::ToInt32($delayText, [Globalization.CultureInfo]::InvariantCulture)
                } catch {
                    throw "流程第 $($call.LineNumber) 行 Delay 的毫秒数超出支持范围：$delayText"
                }
                Write-Output ''
                Write-Output ('-------------------- 流程第 {0} 行：Delay --------------------' -f $call.LineNumber)
                Write-Output ("等待 {0} ms。" -f $milliseconds)
                Start-Sleep -Milliseconds $milliseconds
                continue
            }
            if ($call.Name -ieq 'Config') {
                $selectedConfig = Complete-ConfigArguments $call.Arguments $configDefinitions $call.LineNumber
                Write-Output ''
                Write-Output ('-------------------- 流程第 {0} 行：Config --------------------' -f $call.LineNumber)
                Write-Output ('已选择 Config：{0}' -f (ConvertTo-Json -InputObject $selectedConfig -Compress))
                continue
            }
            if (-not $methods.ContainsKey($call.Name)) {
                throw "流程第 $($call.LineNumber) 行调用了 DLL 未声明的方法：$($call.Name)"
            }

            $expected = @($methods[$call.Name])
            $config = @()
            $input = @($call.Arguments)
            $configKey = $null
            if ($call.Name -eq 'InitDevice') {
                if ($null -eq $selectedConfig) {
                    throw "流程第 $($call.LineNumber) 行 InitDevice 前必须先使用 Config(...) 选择配置。"
                }
                $config = @($selectedConfig)
                $configKey = ConvertTo-Json -InputObject $config -Compress
            } elseif ($call.Name -in @('GetFunctions', 'GetFrameworkVersion', 'GetProjectVersion', 'GetInitConfig', 'GetPortNO')) {
                if ($input.Count -ne $expected.Count) {
                    throw "流程第 $($call.LineNumber) 行 $($call.Name) 参数数量不匹配：期望 $($expected.Count) 个，实际 $($input.Count) 个。"
                }
            } else {
                if ($null -eq $selectedConfig) {
                    throw "流程第 $($call.LineNumber) 行 $($call.Name) 前必须先使用 Config(...) 选择配置。"
                }
                $config = @($selectedConfig)
                $configKey = ConvertTo-Json -InputObject $config -Compress
                if ($call.Name -ne 'CloseDevice' -and -not $initializedConfigs.ContainsKey($configKey)) {
                    throw "流程第 $($call.LineNumber) 行 $($call.Name) 所选 Config 尚未成功 InitDevice。"
                }
            }

            Write-Output ''
            Write-Output ('-------------------- 流程第 {0} 行：{1} --------------------' -f $call.LineNumber, $call.Name)
            $response = [MiresidianDllCaller]::Invoke(
                $call.Name,
                (ConvertTo-Json -InputObject $config -Compress),
                (ConvertTo-Json -InputObject $input -Compress)
            )
            Get-SuccessPayload $response ("流程第 {0} 行 {1}" -f $call.LineNumber, $call.Name) | Out-Null
            Format-Response $response

            if ($call.Name -eq 'InitDevice') {
                $initializedConfigs[$configKey] = @($config)
            } elseif ($call.Name -eq 'CloseDevice') {
                $initializedConfigs.Remove($configKey)
            }
        }
    } finally {
        foreach ($entry in $initializedConfigs.GetEnumerator()) {
            try {
                $config = @($entry.Value)
                Write-Output ''
                Write-Output '流程结束时自动调用 CloseDevice。'
                $response = [MiresidianDllCaller]::Invoke('CloseDevice', (ConvertTo-Json -InputObject $config -Compress), '[]')
                Get-SuccessPayload $response '自动 CloseDevice' | Out-Null
                Format-Response $response
            } catch {
                [Console]::Error.WriteLine("自动 CloseDevice 失败：$($_.Exception.Message)")
            }
        }
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

    $result = [MiresidianDllCaller]::Invoke($Function, $Config, $Input)
    if ($Pretty) {
        Format-Response $result
    } else {
        $result
    }
} catch {
    [Console]::Error.WriteLine("调用失败：$($_.Exception.Message)")
    exit 1
}
