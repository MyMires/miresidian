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
rem Edit only these three test inputs, then double-click this file.
rem FUNCTION is a string; CONFIG and INPUT are JSON string arrays.
rem ================================================================
set "FUNCTION=GetFunctions"
set "CONFIG=[]"
set "INPUT=[]"

rem InitDevice example:
rem set "FUNCTION=InitDevice"
rem set "CONFIG=["COM3","9600","8","None","1","None","1000","1000"]"
rem set "INPUT=[]"

rem Fixed node configuration. Keep DLL selection out of the test-input area.
set "MIRESIDIAN_DLL_PATH=..\dist"
set "MIRESIDIAN_DLL_NAME=miresidian_serial.dll"

if not exist "common\invoke_miresidian_dll.ps1" (
    echo ERROR: common\invoke_miresidian_dll.ps1 was not found.
    goto :end
)

if not exist "%MIRESIDIAN_DLL_PATH%" (
    echo ERROR: DLL directory was not found: %MIRESIDIAN_DLL_PATH%
    goto :end
)

rem Environment variables preserve JSON double quotes; do not pass JSON through cmd.exe arguments.
set "MIRESIDIAN_FUNCTION=%FUNCTION%"
set "MIRESIDIAN_CONFIG_JSON=%CONFIG%"
set "MIRESIDIAN_INPUT_JSON=%INPUT%"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "common\invoke_miresidian_dll.ps1" -FromEnvironment -Pretty

:end
echo.
pause
if defined DID_PUSHD popd >nul 2>&1
endlocal
