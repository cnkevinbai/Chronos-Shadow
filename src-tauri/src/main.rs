// Chronos-Shadow (时空之影) CS-Agent
// 基于 Tauri v2 的 Windows 桌面智能体核心引擎

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    chronos_shadow_lib::run();
}
