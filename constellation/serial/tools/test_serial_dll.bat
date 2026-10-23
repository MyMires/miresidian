@echo off
setlocal

rem pushd maps a UNC script folder (for example \\server\share\tools) to a
rem temporary drive letter because cmd.exe cannot use a UNC path as its CWD.
pushd "%~dp0" >nul 2>&1
if errorlevel 1 (
    echo ERROR: Cannot open this BAT file's folder.
    goto :end
)
set "DID_PUSHD=1"
chcp 65001 >nul

rem ================================================================
rem Edit the UTF-8 flow file, then double-click this file.
rem A flow is InitDevice("COM1"), Method("input"), CloseDevice().
rem ================================================================
set "FLOW_FILE=serial_smoke.flow"

rem Fixed node configuration. Keep DLL selection out of the test-input area.
set "MIRESIDIAN_DLL_PATH=..\..\..\dist"
set "MIRESIDIAN_DLL_NAME=miresidian_serial.dll"

if not exist "invoke_miresidian_dll.ps1" (
    echo ERROR: invoke_miresidian_dll.ps1 was not found.
    goto :end
)

if not exist "%FLOW_FILE%" (
    echo ERROR: Flow file was not found: %FLOW_FILE%
    goto :end
)

if not exist "%MIRESIDIAN_DLL_PATH%" (
    echo ERROR: DLL directory was not found: %MIRESIDIAN_DLL_PATH%
    goto :end
)

rem Environment variables preserve paths and JSON; do not pass them through cmd.exe arguments.
set "MIRESIDIAN_FLOW_FILE=%FLOW_FILE%"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "invoke_miresidian_dll.ps1" -FromEnvironment -Pretty

:end
echo.
pause
if defined DID_PUSHD popd >nul 2>&1
endlocal
