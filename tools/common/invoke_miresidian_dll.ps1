<#
.SYNOPSIS
  无需安装 Python，直接调用 Miresidian Windows DLL。

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\invoke_miresidian_dll.ps1 .\miresidian_serial.dll
  powershell -ExecutionPolicy Bypass -File .\invoke_miresidian_dll.ps1 .\miresidian_serial.dll InitDevice '["COM3","9600","8","None","1","None","1000","1000"]' '[]'
  powershell -ExecutionPolicy Bypass -File .\invoke_miresidian_dll.ps1 .\miresidian_serial.dll -AbiVersion
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
    [string]$DllName = 'miresidian_serial.dll'
)

$ErrorActionPreference = 'Stop'
# BAT switches the console to UTF-8; use the same encoding for formatted output.
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

# BAT uses environment variables so cmd.exe cannot consume JSON's double quotes.
if ($FromEnvironment) {
    $DllPath = $env:MIRESIDIAN_DLL_PATH
    $Function = $env:MIRESIDIAN_FUNCTION
    $Config = $env:MIRESIDIAN_CONFIG_JSON
    $Input = $env:MIRESIDIAN_INPUT_JSON
    $DllName = $env:MIRESIDIAN_DLL_NAME
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

# P/Invoke 要求 DLL 路径嵌入声明；将 Windows 路径安全地转换成 C# 字符串字面量。
$resolvedDllPath = (Resolve-Path -LiteralPath $DllPath).Path
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
    $outputItems = @($payload.output)

    Write-Output ''
    Write-Output '==================== DLL 调用结果 ===================='
    Write-Output ('状态 : {0} ({1})' -f $statusText, $status)
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

try {
    Add-Type -TypeDefinition $source -ErrorAction Stop
    if ($AbiVersion) {
        [MiresidianDllCaller]::miresidian_abi_version()
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
