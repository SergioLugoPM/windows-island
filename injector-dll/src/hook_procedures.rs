//! Hook procedures for intercepting Windows API calls in the injected DLL.
//!
//! This module provides the `hooked_get_sys_color` function, which is registered
//! as a replacement for the Win32 `GetSysColor` API. When Explorer.exe (or any
//! process we are injected into) asks Windows for a system color, our hook
//! intercepts the call, checks whether the requested color index has a dark-theme
//! override, and either returns the override or delegates to the real
//! `GetSysColor` from user32.dll.
//!
//! Phase 2 stubs (`install_hooks` / `uninstall_hooks`) are present but not yet
//! wired to an actual IAT/inline hooking engine; that wiring happens in a later
//! task once the detour library is chosen.

use crate::iat_patcher::{patch_iat_for_get_sys_color, unpatch_iat, ORIGINAL_GET_SYS_COLOR};
use crate::message_handler;
use crate::theme_handler::DARK_THEME_COLORS;

// ---------------------------------------------------------------------------
// Public hook entry-point
// ---------------------------------------------------------------------------

/// Hooked replacement for the Win32 `GetSysColor(nIndex)` API.
///
/// # Calling convention
/// Uses `extern "system"` to match the Win32 ABI (`__stdcall` on x86,
/// identical to C calling convention on x86-64).  The signature mirrors
/// `GetSysColor` exactly so it can be used as a drop-in replacement in an
/// Import Address Table (IAT) hook.
///
/// # Safety
/// - This function is `unsafe` because it may call the original `GetSysColor`
///   via the `windows` crate, which is an FFI call into user32.dll.
/// - Callers must ensure the function is only ever invoked on a Windows thread
///   that has initialised the Win32 subsystem (which is always true inside a
///   process that has already loaded user32.dll).
#[no_mangle]
pub unsafe extern "system" fn hooked_get_sys_color(n_index: i32) -> u32 {
    // 1. Check our dark-theme override table first.
    if let Some(override_color) = get_override_color(n_index) {
        return override_color;
    }

    // 2. No override — call through to the real GetSysColor via the stored
    //    original pointer retrieved by patch_iat_for_get_sys_color().
    if let Some(orig_fn) = ORIGINAL_GET_SYS_COLOR {
        return (orig_fn)(n_index);
    }

    // 3. Fallback: original pointer not yet set (e.g. hook fired before
    //    patch_iat_for_get_sys_color ran). Return a neutral dark colour so
    //    the UI does not show an obviously broken white colour.
    0x1a1a1a
}

// ---------------------------------------------------------------------------
// Helper: color lookup
// ---------------------------------------------------------------------------

/// Search `DARK_THEME_COLORS` for an override entry matching `color_index`.
///
/// Returns `Some(rgb)` when an override is registered, `None` otherwise.
/// Using a linear scan over the static slice is intentional: the slice is
/// short (fewer than 20 entries) so the overhead is negligible compared with
/// the cost of the Win32 FFI call it avoids.
pub fn get_override_color(color_index: i32) -> Option<u32> {
    DARK_THEME_COLORS
        .iter()
        .find(|&&(idx, _)| idx == color_index)
        .map(|&(_, color)| color)
}

// ---------------------------------------------------------------------------
// Phase 2 stubs — hook installation / teardown
// ---------------------------------------------------------------------------

/// Install the `GetSysColor` hook into the host process.
///
/// Phase 3: resolves the original `GetSysColor` symbol from user32.dll and
/// stores it in [`ORIGINAL_GET_SYS_COLOR`] so that `hooked_get_sys_color`
/// can call through to the real implementation.
///
/// Full IAT patching (writing the replacement pointer into the PE import
/// table of the host executable) is deferred to **Phase 4** once the
/// IAT-patching engine is implemented.
pub fn install_hooks() -> Result<(), String> {
    patch_iat_for_get_sys_color(hooked_get_sys_color)?;

    // Install message hook for window events
    message_handler::install_message_hook()?;

    Ok(())
}

/// Remove the `GetSysColor` hook and restore the original function pointer.
///
/// Phase 3: clears [`ORIGINAL_GET_SYS_COLOR`] so the hook stub falls back
/// to the safe dark-colour constant rather than calling through a potentially
/// stale pointer.
///
/// Full IAT restoration (writing the original pointer back into the PE import
/// table) is deferred to **Phase 4**.
pub fn uninstall_hooks() -> Result<(), String> {
    unpatch_iat()?;

    message_handler::uninstall_message_hook()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_found_for_registered_index() {
        // COLOR_WINDOW (index 3) is in DARK_THEME_COLORS → 0x1a1a1a
        assert_eq!(get_override_color(3), Some(0x1a1a1a));
    }

    #[test]
    fn no_override_for_unknown_index() {
        // Index 99 is not in our table
        assert_eq!(get_override_color(99), None);
    }

    #[test]
    fn override_found_for_highlight() {
        // COLOR_HIGHLIGHT (index 10) → 0x646464
        assert_eq!(get_override_color(10), Some(0x646464));
    }
}
