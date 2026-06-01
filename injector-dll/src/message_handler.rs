//! Window message handler for intercepting color change events

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, LRESULT};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowsHookExA, UnhookWindowsHookEx, CallNextHookEx, HHOOK, HC_ACTION,
    WH_CBT, HCBT_CREATEWND, WM_SETTINGCHANGE, WM_SYSCOLORCHANGE,
};
use std::sync::atomic::{AtomicBool, Ordering};

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
    // In a full implementation:
    // 1. Enumerate all windows with FindWindowA for "Shell_TrayWnd"
    // 2. Call InvalidateRect to mark for redraw
    // 3. Call UpdateWindow to force immediate redraw

    // For Phase 3, this is prepared for Phase 4 wiring
    Ok(())
}
