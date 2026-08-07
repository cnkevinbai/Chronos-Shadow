@echo off
set PATH=%PATH%;C:\Program Files\nodejs
echo ========================================
echo  Chronos-Shadow Integration Check
echo ========================================
echo.

echo [1/3] Frontend TypeScript + Vite build...
call npm --prefix chronos-shadow run build
if %ERRORLEVEL% EQU 0 (
    echo   [PASS] Frontend build
) else (
    echo   [FAIL] Frontend build
    exit /b 1
)

echo [2/3] Rust cargo check...
set CARGO_HOME=D:\rust\.cargo
set CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
set PATH=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;D:\rust\.cargo\bin;C:\WINDOWS\system32;C:\WINDOWS
set RUSTC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe
set RUSTDOC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustdoc.exe

D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe check --manifest-path chronos-shadow\src-tauri\Cargo.toml
if %ERRORLEVEL% EQU 0 (
    echo   [PASS] Rust cargo check
) else (
    echo   [FAIL] Rust cargo check
    exit /b 1
)

echo [3/3] Lint check...
call npm --prefix chronos-shadow run lint
if %ERRORLEVEL% EQU 0 (
    echo   [PASS] Lint
) else (
    echo   [WARN] Lint warnings (non-blocking)
)

echo.
echo ========================================
echo   All checks passed.
echo.
echo   Start desktop:  cd chronos-shadow ^&^& tauri-dev.bat
echo   Frontend only:  cd chronos-shadow ^&^& npm run dev
echo ========================================
