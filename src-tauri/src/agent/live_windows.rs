// 顶层窗口枚举 (Live Windows)
// 通过 Win32 EnumWindows 枚举当前可见的顶层窗口，供前端展示

#[tauri::command]
pub fn list_live_windows() -> Vec<serde_json::Value> {
    // Enumerate top-level windows via Win32 EnumWindows
    #[cfg(target_os = "windows")]
    {
        let mut windows = Vec::new();
        unsafe {
            extern "system" {
                fn EnumWindows(callback: unsafe extern "system" fn(isize, isize) -> i32, lparam: isize) -> i32;
                fn IsWindowVisible(hwnd: isize) -> i32;
                fn GetWindowTextLengthW(hwnd: isize) -> i32;
                fn GetWindowTextW(hwnd: isize, buf: *mut u16, max: i32) -> i32;
                fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
            }
            unsafe extern "system" fn enum_proc(hwnd: isize, lparam: isize) -> i32 {
                let windows = &mut *(lparam as *mut Vec<serde_json::Value>);
                if IsWindowVisible(hwnd) == 0 { return 1; }
                let len = GetWindowTextLengthW(hwnd);
                if len == 0 { return 1; }
                let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
                GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                windows.push(serde_json::json!({
                    "id": format!("win-{}", hwnd),
                    "title": title,
                    "pid": pid,
                    "hwnd": hwnd,
                }));
                1
            }
            EnumWindows(enum_proc, &mut windows as *mut _ as isize);
        }
        windows
    }
    #[cfg(not(target_os = "windows"))]
    { vec![] }
}
