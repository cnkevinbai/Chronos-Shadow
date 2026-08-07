@echo off
cd /d D:\Chronos-Shadow\chronos-shadow\src-tauri
set WIX=C:\Users\Administrator\AppData\Local\tauri\WixTools
set OUTDIR=target\release\bundle\msi

if not exist "%OUTDIR%" mkdir "%OUTDIR%"

echo Compiling WiX source...
"%WIX%\candle.exe" -nologo -arch x64 -out "%OUTDIR%\\" chronos-shadow.wxs
if %ERRORLEVEL% neq 0 goto :error

echo Linking MSI...
"%WIX%\light.exe" -nologo -out "%OUTDIR%\Chronos-Shadow_0.1.0_x64.msi" "%OUTDIR%\chronos-shadow.wixobj"
if %ERRORLEVEL% neq 0 goto :error

echo.
echo ========================================
echo  MSI built successfully!
echo  %OUTDIR%\Chronos-Shadow_0.1.0_x64.msi
echo ========================================
goto :end

:error
echo.
echo MSI build FAILED!
exit /b 1

:end
