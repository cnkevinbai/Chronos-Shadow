@echo off
set CARGO_HOME=D:\rust\.cargo
set CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
set RUSTC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe
set RUSTDOC=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustdoc.exe
set CARGO=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe
set PATH=D:\rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;D:\rust\.cargo\bin;%PATH%
npx tauri dev
