// Windows 原生钩子接口 — 键盘/鼠标监听
//
// 通过 SetWindowsHookExW 实现低级别键盘和鼠标事件监听，
// 用于 Shadow Mode（影子结对驾驶）的非侵入式随航监控。
//
// 仅在 #[cfg(target_os = "windows")] 时编译真实实现，
// 其他平台返回空操作。

/// Windows 钩子类型常量
#[allow(dead_code)]
mod hook_types {
    pub const WH_KEYBOARD_LL: i32 = 13;
    pub const WH_MOUSE_LL: i32 = 14;
}

/// 钩子回调函数类型
#[cfg(target_os = "windows")]
type HookProc = unsafe extern "system" fn(code: i32, w_param: usize, l_param: isize) -> isize;

/// Windows FFI 声明
#[cfg(target_os = "windows")]
mod ffi {
    use super::HookProc;

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

/// Windows 钩子管理器
pub struct WinHookManager {
    /// 键盘钩子句柄
    keyboard_hook: Option<isize>,
    /// 鼠标钩子句柄
    mouse_hook: Option<isize>,
    /// 是否已安装
    installed: bool,
    /// 累计键盘事件
    pub key_count: u64,
    /// 累计鼠标事件
    pub mouse_count: u64,
}

impl WinHookManager {
    pub fn new() -> Self {
        Self {
            keyboard_hook: None,
            mouse_hook: None,
            installed: false,
            key_count: 0,
            mouse_count: 0,
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

            // Use a null callback for low-level hooks — we'd normally process events here
            // In production: implement a proper callback that routes events to the ShadowEngine
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

    // Parse KBDLLHOOKSTRUCT
    // vk_code = *(u32*)(l_param)
    // scan_code = *(u32*)(l_param + 4)
    // flags = *(u32*)(l_param + 8)
    let vk_code = *(l_param as *const u32);
    let _scan_code = *((l_param + 4) as *const u32);
    let _flags = *((l_param + 8) as *const u32);

    // WM_KEYDOWN = 0x0100, WM_KEYUP = 0x0101
    let _is_key_up = w_param == 0x0101;

    // In production: send event to ShadowEngine via channel
    // For now: just track count (would use atomic counter)
    tracing::trace!("[WinHook] Key event: vk={}", vk_code);

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

    // Parse MSLLHOOKSTRUCT
    let pt_x = *((l_param + 0) as *const i32);
    let pt_y = *((l_param + 4) as *const i32);

    tracing::trace!("[WinHook] Mouse event: ({}, {})", pt_x, pt_y);
    ffi::CallNextHookEx(0, n_code, w_param, l_param)
}
