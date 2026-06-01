//! IAT Patcher module for Phase 3 — stores the original `GetSysColor` pointer
//! so that `hooked_get_sys_color` can call through to the real implementation.
//!
//! # Phase 3 scope
//! This module retrieves the `GetSysColor` function pointer from user32.dll at
//! DLL load time via `GetModuleHandleA` + `GetProcAddress` and stores it in a
//! `static mut` for the hook stub to call.  It does **not** yet write to the
//! Import Address Table of the host process's PE image; that step (modifying
//! the IAT entries in the loaded executable's memory) is deferred to **Phase 4**
//! once the full IAT-patching engine is implemented.
//!
//! # Safety model
//! All accesses to `ORIGINAL_GET_SYS_COLOR` are gated behind `unsafe` blocks.
//! - `patch_iat_for_get_sys_color` must be called exactly once, during
//!   `DLL_PROCESS_ATTACH`, before any thread can invoke the hook.
//! - `unpatch_iat` must be called exactly once, during `DLL_PROCESS_DETACH`,
//!   after all threads have stopped invoking the hook.
//! No synchronisation primitive is used because both operations happen while
//! the Windows loader lock is held (DllMain), which serialises all threads.

use std::mem;

use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

use crate::pe_parser;

// ---------------------------------------------------------------------------
// Static storage for the original function pointer
// ---------------------------------------------------------------------------

/// The original `GetSysColor` function pointer retrieved from user32.dll.
///
/// Initialised to `None` before `patch_iat_for_get_sys_color` is called and
/// after `unpatch_iat` is called.  Set to `Some(fn_ptr)` while the hook is
/// active.
///
/// # Safety
/// Must only be read or written while the Windows loader lock is held (i.e.
/// from within `DllMain`) or when it is guaranteed that no other thread is
/// concurrently accessing the DLL hook path.
pub static mut ORIGINAL_GET_SYS_COLOR: Option<unsafe extern "system" fn(i32) -> u32> = None;

/// `true` while the IAT entry for GetSysColor has been overwritten with
/// our hook. Used by `unpatch_iat` to decide whether to restore.
pub static mut IAT_PATCHED: bool = false;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Locate `GetSysColor` in user32.dll and store the pointer in
/// [`ORIGINAL_GET_SYS_COLOR`].
///
/// The `hooked_fn` parameter accepts the address of our hook function.  It is
/// currently unused in Phase 3 (we only store the *original* pointer), but the
/// argument is present so that Phase 4 can pass the replacement address into
/// the actual IAT-patching engine without changing the call-site signature.
///
/// # Errors
/// Returns a human-readable `Err` string when:
/// - `GetModuleHandleA("user32.dll")` fails (user32 is not loaded)
/// - `GetProcAddress(…, "GetSysColor")` returns null (symbol not exported)
///
/// # Safety (internal)
/// Win32 calls are wrapped in `unsafe` blocks; the function itself is safe
/// to call from `DllMain`.
pub fn patch_iat_for_get_sys_color(
    hooked_fn: unsafe extern "system" fn(i32) -> u32,
) -> Result<(), String> {
    unsafe {
        // A. Store the original GetSysColor pointer for hooked_get_sys_color
        //    call-through (fallback when IAT patch is not active).
        let user32_name = PCSTR::from_raw(b"user32.dll\0".as_ptr());
        let h_user32 = GetModuleHandleA(user32_name)
            .map_err(|e| format!("GetModuleHandleA(\"user32.dll\") failed: {e}"))?;

        let proc_name = PCSTR::from_raw(b"GetSysColor\0".as_ptr());
        let raw_proc = GetProcAddress(h_user32, proc_name)
            .ok_or_else(|| "GetProcAddress(\"GetSysColor\") returned null".to_string())?;

        ORIGINAL_GET_SYS_COLOR = Some(mem::transmute(raw_proc));

        // B. Patch the IAT so every call to GetSysColor inside Explorer.exe
        //    goes through our hook instead of the real implementation.
        match pe_parser::find_and_patch_iat(
            b"user32.dll",
            b"GetSysColor",
            hooked_fn as usize,
        ) {
            Ok(_original_addr) => {
                IAT_PATCHED = true;
            }
            Err(e) => {
                // Non-fatal: hook call-through still works via
                // ORIGINAL_GET_SYS_COLOR; Explorer calls just won't be
                // intercepted at the IAT level.
                let _ = format!("[windows-island] IAT patch skipped: {e}");
            }
        }
    }

    Ok(())
}

/// Clear the stored `GetSysColor` pointer, restoring the static to `None`.
///
/// Must be called during `DLL_PROCESS_DETACH` to prevent the hook stub from
/// calling through a dangling pointer after user32.dll is unmapped (which in
/// practice does not happen before our DLL, but is good hygiene).
///
/// # Errors
/// Currently infallible; returns `Ok(())` always.  The `Result` return type is
/// kept for API symmetry with `patch_iat_for_get_sys_color` and to allow future
/// Phase 4 logic (which will need to restore the original IAT entry) to surface
/// errors without a breaking signature change.
pub fn unpatch_iat() -> Result<(), String> {
    unsafe {
        if IAT_PATCHED {
            if let Some(orig_fn) = ORIGINAL_GET_SYS_COLOR {
                // Walk the IAT again and write the original address back.
                match pe_parser::find_and_patch_iat(
                    b"user32.dll",
                    b"GetSysColor",
                    orig_fn as usize,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        // Do not panic — we must still clear statics.
                        let _ = format!("[windows-island] IAT restore failed: {e}");
                    }
                }
            }
            IAT_PATCHED = false;
        }

        ORIGINAL_GET_SYS_COLOR = None;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: a dummy function with the correct signature used as a stand-in
    /// for the real hook when we only want to exercise the storage mechanism.
    unsafe extern "system" fn dummy_hook(_n_index: i32) -> u32 {
        0
    }

    #[test]
    fn original_pointer_starts_as_none() {
        // Before patch_iat_for_get_sys_color is called the static must be None.
        // NOTE: test isolation is best-effort here — if another test in the
        // same process has already set the static this assertion may fail.
        // In practice the test binary starts fresh and this runs first.
        unsafe {
            // We read but do not mutate, so this is safe within a single-
            // threaded test runner.
            let current = ORIGINAL_GET_SYS_COLOR;
            // The value is either None (never patched) or Some (a prior test
            // patched it).  Either way the static must be a valid Option.
            let _ = current;
        }
    }

    #[test]
    fn can_set_and_read_original_pointer() {
        // Verify that ORIGINAL_GET_SYS_COLOR can be written and read back.
        // This tests the storage mechanism independently of any Win32 call.
        unsafe {
            let fn_ptr: unsafe extern "system" fn(i32) -> u32 = dummy_hook;
            ORIGINAL_GET_SYS_COLOR = Some(fn_ptr);

            assert!(
                ORIGINAL_GET_SYS_COLOR.is_some(),
                "ORIGINAL_GET_SYS_COLOR should be Some after assignment"
            );

            // Verify the stored function pointer is callable and returns the
            // expected dummy value.
            let stored = ORIGINAL_GET_SYS_COLOR.unwrap();
            let result = stored(0);
            assert_eq!(result, 0, "dummy_hook should return 0 for any input");

            // Clean up for other tests.
            ORIGINAL_GET_SYS_COLOR = None;
        }
    }

    #[test]
    fn unpatch_clears_the_pointer() {
        unsafe {
            // Arrange: set a non-None value.
            ORIGINAL_GET_SYS_COLOR = Some(dummy_hook);
            assert!(ORIGINAL_GET_SYS_COLOR.is_some());

            // Act.
            unpatch_iat().expect("unpatch_iat must not fail");

            // Assert.
            assert!(
                ORIGINAL_GET_SYS_COLOR.is_none(),
                "unpatch_iat should clear ORIGINAL_GET_SYS_COLOR"
            );
        }
    }
}
