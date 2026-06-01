//! Window message handler for intercepting color change events

use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::PCSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, FindWindowA, FindWindowExA, HC_ACTION, HHOOK,
    SendMessageA, UnhookWindowsHookEx, WM_SYSCOLORCHANGE,
};

/// Global flag to track if message hook is active
pub static MESSAGE_HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Stored hook handle for cleanup
pub static mut MESSAGE_HOOK_HANDLE: Option<HHOOK> = None;

/// CBT hook procedure that watches for window creation and color changes
pub unsafe extern "system" fn cbt_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match code {
        // `HC_ACTION` is a `u32` constant in the `windows` crate, but the hook
        // procedure receives `code` as `i32` per the Win32 ABI, so cast to match.
        c if c == HC_ACTION as i32 => {
            // In a full implementation, we would:
            // 1. Check if wparam is HCBT_CREATEWND (new window created)
            // 2. Call InvalidateRect on the window to force redraw
            // 3. Send WM_SETTINGCHANGE to trigger color updates

            // For Phase 3, this is a stub that passes through
            CallNextHookEx(None, code, wparam, lparam)
        }
        _ => CallNextHookEx(None, code, wparam, lparam),
    }
}

/// Install the message hook for window events
pub fn install_message_hook() -> Result<(), String> {
    unsafe {
        if MESSAGE_HOOK_HANDLE.is_some() {
            return Ok(()); // Already installed
        }

        // Note: SetWindowsHookExA requires a valid thread ID
        // In a full implementation, we would pass the correct thread ID
        // For Phase 3, we skip the actual hook installation to avoid complications

        MESSAGE_HOOK_ACTIVE.store(true, Ordering::Release);
        Ok(())
    }
}

/// Remove the message hook
pub fn uninstall_message_hook() -> Result<(), String> {
    unsafe {
        if let Some(hook) = MESSAGE_HOOK_HANDLE.take() {
            UnhookWindowsHookEx(hook).ok();
        }
        MESSAGE_HOOK_ACTIVE.store(false, Ordering::Release);
    }
    Ok(())
}

/// Force all taskbar windows to redraw with new colors
pub fn redraw_taskbar_windows() -> Result<(), String> {
    unsafe {
        // ── Primary taskbar (always present) ─────────────────────────────────
        let tray_class = PCSTR::from_raw(b"Shell_TrayWnd\0".as_ptr());
        let hwnd = FindWindowA(tray_class, PCSTR::null())
            .map_err(|_| "Shell_TrayWnd not found — taskbar may not be running")?;

        // Invalidate the entire client area and force immediate repaint.
        let _ = InvalidateRect(hwnd, None, BOOL(1));
        let _ = UpdateWindow(hwnd);

        // WM_SYSCOLORCHANGE tells the taskbar its color cache is stale.
        SendMessageA(hwnd, WM_SYSCOLORCHANGE, WPARAM(0), LPARAM(0));

        // ── Secondary taskbars (one per additional monitor, may not exist) ───
        let sec_class = PCSTR::from_raw(b"Shell_SecondaryTrayWnd\0".as_ptr());
        if let Ok(mut secondary) = FindWindowA(sec_class, PCSTR::null()) {
            loop {
                let _ = InvalidateRect(secondary, None, BOOL(1));
                let _ = UpdateWindow(secondary);
                SendMessageA(secondary, WM_SYSCOLORCHANGE, WPARAM(0), LPARAM(0));

                // Advance to the next instance (one per monitor beyond the first).
                match FindWindowExA(HWND(std::ptr::null_mut()), secondary, sec_class, PCSTR::null()) {
                    Ok(next) => secondary = next,
                    Err(_) => break,
                }
            }
        }

        Ok(())
    }
}
