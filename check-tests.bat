@echo off
cd /d D:\Chronos-Shadow\chronos-shadow
set CARGO_HOME=D:\rust\.cargo
set CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
set PATH=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;D:\rust\.cargo\bin;C:\Program Files\nodejs;C:\WINDOWS\system32;C:\WINDOWS
set CARGO_BUILD_JOBS=1
D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe check --tests --release --manifest-path src-tauri/Cargo.toml
