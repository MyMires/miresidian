param(
    [string]$Version = "latest",
    [string]$Archive,
    [string]$InstallDir
)
$ErrorActionPreference = 'Stop'
$arguments = @((Join-Path $PSScriptRoot 'install.py'), '--version', $Version)
if ($Archive) { $arguments += @('--archive', $Archive) }
if ($InstallDir) { $arguments += @('--install-dir', $InstallDir) }
if (Get-Command py -ErrorAction SilentlyContinue) {
    & py -3 @arguments
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    & python @arguments
} else {
    throw '需要 Python 3.11+；也可手动解压完整工具套件并将目录加入 PATH。'
}
exit $LASTEXITCODE
