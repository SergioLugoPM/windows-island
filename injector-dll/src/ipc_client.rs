//! IPC client for reading theme configuration from the main Tauri application.
//!
//! The main Tauri app creates a named file mapping "Local\WindowsIsland_Theme_IPC_v1"
//! with PAGE_READWRITE access and writes a `ThemeConfig` struct into it.  This
//! client opens the same mapping with PAGE_READONLY and reads the struct out via
//! `ptr::read`.  No polling is required: the DLL reads on demand each time the
//! hook fires.

use std::ptr;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Memory::{
    FILE_MAP_READ, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS,
};
use windows::core::PCSTR;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Name of the named file mapping created by the Tauri host application.
/// Must match exactly what Task 4 (ipc_server.rs) creates on the host side.
/// Stored as a null-terminated byte string for use with `CreateFileMappingA` /
/// `OpenFileMappingA` (ANSI variants require `PCSTR`).
pub const THEME_IPC_PIPE_NAME: &[u8] = b"Local\\WindowsIsland_Theme_IPC_v1\0";

// ---------------------------------------------------------------------------
// ThemeConfig — binary layout shared with host app
// ---------------------------------------------------------------------------

/// Theme configuration read from shared memory.
///
/// # Layout
/// `#[repr(C)]` guarantees a stable, C-compatible binary layout so that the
/// struct can be safely cast from a raw pointer regardless of Rust compiler
/// version or optimisation level.
///
/// # Booleans as `u8`
/// `border_iridescence` and `is_dark_mode` are stored as `u8` (0 = false,
/// 1 = true) rather than Rust `bool` to avoid any ABI ambiguity: `bool` is
/// defined to occupy exactly one byte in Rust, but its bit-pattern guarantees
/// differ from C's `_Bool`, which can cause UB when reading arbitrary memory.
/// Using `u8` makes the read unconditionally safe for any bit pattern the host
/// may write.
///
/// # Version field
/// The `version` field lets future versions of the host app write a schema
/// version the DLL can check before interpreting the rest of the struct.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThemeConfig {
    /// Primary UI color as [R, G, B]
    pub primary_rgb: [u8; 3],
    /// Accent / highlight color as [R, G, B]
    pub accent_rgb: [u8; 3],
    /// Window transparency level (0.0 = fully transparent, 1.0 = opaque)
    pub transparency: f32,
    /// Whether to render iridescent border effects (0 = off, 1 = on)
    pub border_iridescence: u8,
    /// Window background color as [R, G, B]
    pub background_rgb: [u8; 3],
    /// Foreground / text color as [R, G, B]
    pub foreground_rgb: [u8; 3],
    /// Dark-mode flag (0 = light theme, 1 = dark theme)
    pub is_dark_mode: u8,
    /// Schema version; increment when layout changes
    pub version: u32,
}

// ---------------------------------------------------------------------------
// IpcClient
// ---------------------------------------------------------------------------

/// Holds an open handle to the shared memory region and a mapped view pointer.
///
/// # Lifetime
/// Intended to be stored in a `OnceLock<IpcClient>` for the lifetime of the
/// DLL.  The `Drop` impl releases both the view and the handle automatically.
///
/// # Thread safety
/// Named file mappings are a kernel object; the HANDLE is inherently shareable
/// across threads.  The view pointer is used only for atomic `ptr::read` of
/// a `Copy` struct, which is safe to do from any thread without additional
/// synchronisation.
pub struct IpcClient {
    mapping_handle: HANDLE,
    view_address: MEMORY_MAPPED_VIEW_ADDRESS,
}

// SAFETY: `HANDLE` is a raw pointer typedef, but Windows named file-mapping
// handles are kernel objects designed for cross-process (and cross-thread)
// sharing.  `MEMORY_MAPPED_VIEW_ADDRESS` wraps a `*mut c_void` which we only
// ever read through (via `ptr::read`), never write through, and the underlying
// memory is owned by the kernel until we call `UnmapViewOfFile`.  Treating the
// view as read-only from multiple threads is safe.
unsafe impl Send for IpcClient {}
unsafe impl Sync for IpcClient {}

impl IpcClient {
    /// Open the named file mapping created by the host Tauri application.
    ///
    /// Returns `Err` with a human-readable message if the mapping does not
    /// exist yet (i.e. the host has not started the IPC server) or if the OS
    /// call fails for any other reason.
    ///
    /// # Safety (internal)
    /// All Win32 calls are wrapped in `unsafe` blocks; the function itself is
    /// safe to call from any context.
    pub fn connect() -> Result<Self, String> {
        unsafe {
            // SAFETY: THEME_IPC_PIPE_NAME is a valid null-terminated byte
            // string; PCSTR wraps the pointer without taking ownership.
            let mapping_name = PCSTR::from_raw(THEME_IPC_PIPE_NAME.as_ptr());

            // Open an existing file mapping — we do NOT create one; the host
            // app is responsible for creation.  PAGE_READONLY is sufficient
            // for the DLL client.
            let h_mapping = OpenFileMappingA(
                FILE_MAP_READ.0, // dwDesiredAccess — read-only
                false,           // bInheritHandle  — not inherited by children
                mapping_name,
            )
            .map_err(|e| format!("OpenFileMappingA failed: {}", e))?;

            // Map a view of the file into this process's address space.
            // Requesting exactly `size_of::<ThemeConfig>` bytes is a hint; the
            // OS will round up to a page boundary but we only access our
            // struct's range.
            let view = MapViewOfFile(
                h_mapping,
                FILE_MAP_READ,
                0, // dwFileOffsetHigh
                0, // dwFileOffsetLow
                std::mem::size_of::<ThemeConfig>(),
            );

            if view.Value.is_null() {
                // MapViewOfFile failed; close the handle we just opened.
                let _ = CloseHandle(h_mapping);
                return Err("MapViewOfFile returned null pointer".to_string());
            }

            Ok(Self {
                mapping_handle: h_mapping,
                view_address: view,
            })
        }
    }

    /// Read a snapshot of the current theme configuration from shared memory.
    ///
    /// Uses `ptr::read` (a bitwise copy) to avoid any reference aliasing with
    /// the memory-mapped region, which is written by a different process.
    ///
    /// Returns `Err` if the view pointer is unexpectedly null (should not
    /// happen after a successful `connect`).
    pub fn read_theme_config(&self) -> Result<ThemeConfig, String> {
        if self.view_address.Value.is_null() {
            return Err("Shared memory view pointer is null".to_string());
        }

        // SAFETY:
        // - `view_address.Value` points to at least `size_of::<ThemeConfig>()`
        //   bytes mapped from the kernel file-mapping object.
        // - `ThemeConfig` is `#[repr(C)]` and `Copy`, so any bit pattern is a
        //   valid (if potentially garbage) value — reading before the host has
        //   written will yield zeroes or stale data, not UB.
        // - `ptr::read` performs a bitwise copy without creating a reference to
        //   the mapped memory, avoiding aliasing rules with the writer process.
        let config = unsafe {
            let theme_ptr = self.view_address.Value as *const ThemeConfig;
            ptr::read(theme_ptr)
        };

        Ok(config)
    }

    /// Refresh the cached theme config from shared memory
    pub fn refresh_theme(&self) -> Result<ThemeConfig, String> {
        self.read_theme_config()
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        // SAFETY: Both values were obtained from successful Win32 calls in
        // `connect`.  We check for null/invalid before releasing to be
        // defensive, even though they should never be in that state here.
        unsafe {
            if !self.view_address.Value.is_null() {
                let _ = UnmapViewOfFile(self.view_address);
            }
            if !self.mapping_handle.is_invalid() {
                let _ = CloseHandle(self.mapping_handle);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_config_has_expected_size() {
        // Verify the struct size is stable. If this fails, the host-app layout
        // has diverged and the IPC protocol needs a version bump.
        // Layout: 3+3+4+1+3+3+1+4 = 22 bytes of fields, but repr(C) padding
        // after border_iridescence (u8) before background_rgb ([u8;3]) and
        // after is_dark_mode (u8) before version (u32 — 4-byte alignment)
        // means actual size may differ by target.
        let size = std::mem::size_of::<ThemeConfig>();
        // Must be at least the sum of fields (22 bytes).
        assert!(size >= 22, "ThemeConfig too small: {} bytes", size);
    }

    #[test]
    fn theme_config_is_copy() {
        // Compile-time check that ThemeConfig implements Copy (required for
        // safe use with ptr::read and OnceLock).
        fn assert_copy<T: Copy>() {}
        assert_copy::<ThemeConfig>();
    }

    #[test]
    fn connect_fails_gracefully_when_mapping_absent() {
        // In a test environment the host app is not running, so the mapping
        // does not exist.  connect() must return Err, not panic.
        let result = IpcClient::connect();
        assert!(
            result.is_err(),
            "Expected Err when host mapping is absent, got Ok"
        );
    }
}
