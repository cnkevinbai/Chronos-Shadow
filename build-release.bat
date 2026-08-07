@echo off
echo ========================================
echo  Chronos-Shadow Production Build
echo  Tauri v2 Release EXE Packaging
echo  (Low memory mode: -j 1, opt-level=2)
echo ========================================
echo.

set CARGO_HOME=D:\rust\.cargo
set CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
set PATH=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;D:\rust\.cargo\bin;C:\WINDOWS\system32;C:\WINDOWS;C:\Program Files\nodejs
set RUSTC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe
set CARGO_BUILD_JOBS=1

echo Building with single-threaded compilation to avoid OOM...
npx tauri build
