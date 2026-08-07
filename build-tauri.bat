@echo off
cd /d D:\Chronos-Shadow\chronos-shadow

set CARGO_HOME=D:\rust\.cargo
set CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
set PATH=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;D:\rust\.cargo\bin;C:\Program Files\nodejs;C:\WINDOWS\system32;C:\WINDOWS
set RUSTC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe
set CARGO_BUILD_JOBS=1
set WIX=C:\Users\Administrator\AppData\Local\tauri\WixTools

echo ========================================
echo  Chronos-Shadow Tauri Build
echo ========================================
echo.

node_modules\.bin\tauri.cmd build
