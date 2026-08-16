// Windows 原生钩子接口 — 键盘/鼠标监听
//
// 通过 SetWindowsHookExW 实现低级别键盘和鼠标事件监听，
// 用于 Shadow Mode（影子结对驾驶）的非侵入式随航监控。
//
// 回调中：累加全局原子计数 + 通过 mpsc 通道转发事件给应用层消费。
// 低级别钩子需要消息泵，spawn_hook_listener 在后台线程安装钩子并循环取消息。
// 仅在 #[cfg(target_os = "windows")] 时编译真实实现，其他平台返回空操作。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Windows 钩子类型常量
#[allow(dead_code)]
mod hook_types {
    pub const WH_KEYBOARD_LL: i32 = 13;
    pub const WH_MOUSE_LL: i32 = 14;
}

/// 钩子回调函数类型
#[cfg(target_os = "windows")]
type HookProc = unsafe extern "system" fn(code: i32, w_param: usize, l_param: isize) -> isize;

/// Windows 消息结构（消息泵用）
#[cfg(target_os = "windows")]
#[repr(C)]
struct Msg {
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

/// Windows FFI 声明
#[cfg(target_os = "windows")]
mod ffi {
    use super::{HookProc, Msg};

    #[link(name = "user32")]
    extern "system" {
        pub fn SetWindowsHookExW(
            id_hook: i32,
            lpfn: HookProc,
            h_mod: isize,
            dw_thread_id: u32,
        ) -> isize;

        pub fn UnhookWindowsHookEx(hhk: isize) -> i32;

        pub fn CallNextHookEx(
            hhk: isize,
            n_code: i32,
            w_param: usize,
            l_param: isize,
        ) -> isize;

        pub fn GetMessageW(
            lp_msg: *mut Msg,
            h_wnd: isize,
            w_msg_filter_min: u32,
            w_msg_filter_max: u32,
        ) -> i32;

        pub fn TranslateMessage(lp_msg: *const Msg) -> i32;

        pub fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetModuleHandleW(lp_module_name: *const u16) -> isize;
    }
}

/// 键盘事件数据（从 KBDLLHOOKSTRUCT 提取）
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub vk_code: u32,
    pub scan_code: u32,
    pub flags: u32,
    pub is_key_up: bool,
}

/// 鼠标事件数据
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub button: u32,
    pub is_up: bool,
}

/// 钩子事件（回调 → 应用层消费）
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// 键盘按下/抬起
    Key { vk_code: u32, is_key_up: bool },
    /// 鼠标移动/按键（message 为 Windows 鼠标消息，如 0x0200 移动、0x0201 左键按下）
    Mouse { x: i32, y: i32, message: u32 },
}

// ─── 全局状态（钩子回调与主线程共享） ─────────────────────────────

static KEY_COUNT: AtomicU64 = AtomicU64::new(0);
static MOUSE_COUNT: AtomicU64 = AtomicU64::new(0);
static EVENT_TX: OnceLock<Sender<HookEvent>> = OnceLock::new();

/// 初始化事件通道，返回接收端（应在安装钩子前调用一次）
pub fn init_event_channel() -> Receiver<HookEvent> {
    let (tx, rx) = channel();
    let _ = EVENT_TX.set(tx);
    rx
}

/// 累计键盘事件数
pub fn key_event_count() -> u64 {
    KEY_COUNT.load(Ordering::Relaxed)
}

/// 累计鼠标事件数
pub fn mouse_event_count() -> u64 {
    MOUSE_COUNT.load(Ordering::Relaxed)
}

// ─── Windows 钩子管理器 ──────────────────────────────────────────

/// Windows 钩子管理器
pub struct WinHookManager {
    /// 键盘钩子句柄
    keyboard_hook: Option<isize>,
    /// 鼠标钩子句柄
    mouse_hook: Option<isize>,
    /// 是否已安装
    installed: bool,
}

impl WinHookManager {
    pub fn new() -> Self {
        Self {
            keyboard_hook: None,
            mouse_hook: None,
            installed: false,
        }
    }

    /// 安装键盘钩子（仅在 Windows 上有效）
    #[cfg(target_os = "windows")]
    pub fn install_keyboard_hook(&mut self) -> Result<(), String> {
        if self.keyboard_hook.is_some() {
            return Ok(()); // Already installed
        }

        unsafe {
            let h_mod = ffi::GetModuleHandleW(std::ptr::null());
            if h_mod == 0 {
                return Err("GetModuleHandleW failed".into());
            }

            let hook = ffi::SetWindowsHookExW(
                hook_types::WH_KEYBOARD_LL,
                keyboard_proc as HookProc,
                h_mod,
                0,
            );

            if hook == 0 {
                return Err("SetWindowsHookExW (keyboard) failed".into());
            }

            self.keyboard_hook = Some(hook);
        }
        self.installed = true;
        Ok(())
    }

    /// 安装鼠标钩子
    #[cfg(target_os = "windows")]
    pub fn install_mouse_hook(&mut self) -> Result<(), String> {
        if self.mouse_hook.is_some() {
            return Ok(());
        }

        unsafe {
            let h_mod = ffi::GetModuleHandleW(std::ptr::null());
            if h_mod == 0 {
                return Err("GetModuleHandleW failed".into());
            }

            let hook = ffi::SetWindowsHookExW(
                hook_types::WH_MOUSE_LL,
                mouse_proc as HookProc,
                h_mod,
                0,
            );

            if hook == 0 {
                return Err("SetWindowsHookExW (mouse) failed".into());
            }

            self.mouse_hook = Some(hook);
        }
        self.installed = true;
        Ok(())
    }

    /// 卸载所有钩子
    #[cfg(target_os = "windows")]
    pub fn uninstall_all(&mut self) {
        if let Some(hook) = self.keyboard_hook.take() {
            unsafe {
                ffi::UnhookWindowsHookEx(hook);
            }
        }
        if let Some(hook) = self.mouse_hook.take() {
            unsafe {
                ffi::UnhookWindowsHookEx(hook);
            }
        }
        self.installed = false;
    }

    // Non-Windows stubs
    #[cfg(not(target_os = "windows"))]
    pub fn install_keyboard_hook(&mut self) -> Result<(), String> {
        Err("Windows hooks only available on Windows".into())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn install_mouse_hook(&mut self) -> Result<(), String> {
        Err("Windows hooks only available on Windows".into())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn uninstall_all(&mut self) {
        self.installed = false;
    }

    pub fn is_installed(&self) -> bool {
        self.installed
    }
}

impl Drop for WinHookManager {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        self.uninstall_all();
    }
}

// ─── 后台监听线程（安装钩子 + 消息泵） ────────────────────────────

/// 在后台线程安装键盘/鼠标钩子并运行消息泵。
///
/// 低级别钩子要求安装线程持续派发消息，否则回调不会触发。
/// 返回线程句柄，线程退出时自动卸载钩子。
#[cfg(target_os = "windows")]
pub fn spawn_hook_listener() -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("win-hooks".into())
        .spawn(|| {
            let mut mgr = WinHookManager::new();
            if let Err(e) = mgr.install_keyboard_hook() {
                tracing::warn!("[WinHook] keyboard hook install failed: {}", e);
            }
            if let Err(e) = mgr.install_mouse_hook() {
                tracing::warn!("[WinHook] mouse hook install failed: {}", e);
            }
            // 消息泵：GetMessageW 阻塞直到收到消息，返回 0 表示 WM_QUIT
            unsafe {
                let mut msg: Msg = std::mem::zeroed();
                while ffi::GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    ffi::TranslateMessage(&msg);
                    ffi::DispatchMessageW(&msg);
                }
            }
            mgr.uninstall_all();
            tracing::info!("[WinHook] hook listener thread exiting");
        })
        .expect("failed to spawn win-hooks thread")
}

#[cfg(not(target_os = "windows"))]
pub fn spawn_hook_listener() -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {})
}

// ─── 钩子回调（最低级别处理） ──────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    // n_code < 0 → 必须传递给 CallNextHookEx
    if n_code < 0 {
        return ffi::CallNextHookEx(0, n_code, w_param, l_param);
    }

    // KBDLLHOOKSTRUCT: vk_code = *(u32*)(l_param)
    let vk_code = *(l_param as *const u32);
    // WM_KEYDOWN = 0x0100, WM_KEYUP = 0x0101
    let is_key_up = w_param == 0x0101;

    KEY_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(HookEvent::Key { vk_code, is_key_up });
    }

    // Always pass to next hook
    ffi::CallNextHookEx(0, n_code, w_param, l_param)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if n_code < 0 {
        return ffi::CallNextHookEx(0, n_code, w_param, l_param);
    }

    // MSLLHOOKSTRUCT: pt 在结构体开头（x, y 两个 i32）
    let pt_x = *((l_param + 0) as *const i32);
    let pt_y = *((l_param + 4) as *const i32);
    let message = w_param as u32;

    MOUSE_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(HookEvent::Mouse { x: pt_x, y: pt_y, message });
    }

    ffi::CallNextHookEx(0, n_code, w_param, l_param)
}
