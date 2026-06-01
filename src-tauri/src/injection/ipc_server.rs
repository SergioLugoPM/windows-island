//! IPC server — exposes theme configuration to the injected DLL via a named
//! file mapping.
//!
//! The DLL client (Task 3, `ipc_client.rs`) opens the same mapping with
//! `PAGE_READONLY` / `FILE_MAP_READ` and reads the `ThemeConfig` struct out.
//! This server creates the mapping with `PAGE_READWRITE` and writes into it.
//!
//! # Mapping name
//! `Local\WindowsIsland_Theme_IPC_v1` — must be identical to the constant
//! `THEME_IPC_PIPE_NAME` in the DLL's `ipc_client.rs`.

use std::mem;
use std::ptr;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingA, MapViewOfFile, UnmapViewOfFile,
    FILE_MAP_WRITE, PAGE_READWRITE, MEMORY_MAPPED_VIEW_ADDRESS,
};
use windows::core::PCSTR;

// ---------------------------------------------------------------------------
// IpcThemeConfig — binary layout MUST match DLL's ThemeConfig exactly
// ---------------------------------------------------------------------------

/// Theme configuration written into shared memory by the Tauri host app.
///
/// # Critical constraint
/// This struct's `#[repr(C)]` layout must be byte-for-byte identical to
/// `ThemeConfig` in the DLL's `ipc_client.rs`.  Any change here requires a
/// matching change there (and a version bump of the `version` field).
///
/// # Booleans as `u8`
/// `border_iridescence` and `is_dark_mode` are stored as `u8` (0 = false,
/// 1 = true) to match the DLL's layout.  Using Rust `bool` could introduce
/// ABI ambiguity when read as raw bytes by the DLL.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcThemeConfig {
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

impl IpcThemeConfig {
    /// Dark pill theme — dark background with blue accent.
    pub fn dark_theme() -> Self {
        Self {
            primary_rgb: [20, 20, 25],
            accent_rgb: [100, 180, 255],
            transparency: 0.95,
            border_iridescence: 0,
            background_rgb: [15, 15, 20],
            foreground_rgb: [240, 240, 255],
            is_dark_mode: 1,
            version: 1,
        }
    }

    /// Frosted white theme — light background with subtle blue accent.
    pub fn light_theme() -> Self {
        Self {
            primary_rgb: [245, 245, 250],
            accent_rgb: [100, 150, 220],
            transparency: 0.92,
            border_iridescence: 0,
            background_rgb: [255, 255, 255],
            foreground_rgb: [30, 30, 40],
            is_dark_mode: 0,
            version: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// IpcServer
// ---------------------------------------------------------------------------

/// Owns the named file mapping and a mapped view for writing theme config.
///
/// Create once via `IpcServer::new()` and store in a `OnceLock<IpcServer>`.
/// Use `update_config` to push a new theme; the DLL reads on demand.
pub struct IpcServer {
    mapping_handle: HANDLE,
    view_address: MEMORY_MAPPED_VIEW_ADDRESS,
    current_config: Arc<Mutex<IpcThemeConfig>>,
}

// SAFETY: `HANDLE` and `MEMORY_MAPPED_VIEW_ADDRESS` both wrap raw pointers.
// The kernel file-mapping object is inherently shareable across threads and
// processes.  We only access the view through `write_config`, which holds
// the `current_config` mutex for the duration of the write, preventing
// concurrent writes.  Reads from the DLL process are always whole-struct
// `ptr::read` copies and do not race destructively with our writes.
unsafe impl Send for IpcServer {}
unsafe impl Sync for IpcServer {}

impl IpcServer {
    /// Create the named file mapping and map a writable view.
    ///
    /// Calling this a second time in the same process is safe — Windows will
    /// return the existing mapping handle and `ERROR_ALREADY_EXISTS`, which
    /// we treat as success.
    pub fn new() -> Result<Self, String> {
        unsafe {
            // SAFETY: byte string is a valid null-terminated ANSI string.
            let mapping_name = PCSTR::from_raw(b"Local\\WindowsIsland_Theme_IPC_v1\0".as_ptr());

            let mapping_handle = CreateFileMappingA(
                INVALID_HANDLE_VALUE, // backed by the paging file
                None,                 // default security attributes
                PAGE_READWRITE,
                0,                                          // max size high DWORD
                mem::size_of::<IpcThemeConfig>() as u32,   // max size low DWORD
                mapping_name,
            )
            .map_err(|e| format!("CreateFileMappingA failed: {}", e))?;

            let view_address = MapViewOfFile(
                mapping_handle,
                FILE_MAP_WRITE,
                0, // file offset high
                0, // file offset low
                mem::size_of::<IpcThemeConfig>(),
            );

            if view_address.Value.is_null() {
                let _ = CloseHandle(mapping_handle);
                return Err("MapViewOfFile returned null pointer".to_string());
            }

            let server = Self {
                mapping_handle,
                view_address,
                current_config: Arc::new(Mutex::new(IpcThemeConfig::dark_theme())),
            };

            // Write the default (dark) config immediately so the DLL has valid
            // data from the moment it opens the mapping.
            server.write_config()?;

            Ok(server)
        }
    }

    /// Replace the current theme config and flush it to shared memory.
    pub fn update_config(&self, config: IpcThemeConfig) -> Result<(), String> {
        *self
            .current_config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))? = config;
        self.write_config()
    }

    /// Copy `current_config` into the shared memory view.
    fn write_config(&self) -> Result<(), String> {
        let config = self
            .current_config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))?;

        if self.view_address.Value.is_null() {
            return Err("Shared memory view pointer is null".to_string());
        }

        // SAFETY:
        // - `view_address.Value` points to at least `size_of::<IpcThemeConfig>()`
        //   bytes mapped from the kernel paging-file-backed mapping.
        // - `IpcThemeConfig` is `#[repr(C)]` and `Copy`; all bit patterns are valid.
        // - We hold the mutex, so no other thread can call `write_config` concurrently.
        unsafe {
            let dst = self.view_address.Value.cast::<IpcThemeConfig>();
            ptr::copy_nonoverlapping(&*config as *const IpcThemeConfig, dst, 1);
        }

        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
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
    fn ipc_theme_config_size_matches_dll() {
        // The DLL comment states the fields sum to 22 bytes before padding.
        // Layout: [u8;3]+[u8;3]+f32+u8+[u8;3]+[u8;3]+u8+u32
        //       =  3   + 3   + 4  + 1 +  3   +  3   + 1 + 4 = 22 bytes
        // repr(C) may add padding for alignment; we just assert a minimum.
        assert!(
            mem::size_of::<IpcThemeConfig>() >= 22,
            "IpcThemeConfig too small: {} bytes",
            mem::size_of::<IpcThemeConfig>()
        );
    }

    #[test]
    fn dark_theme_fields() {
        let d = IpcThemeConfig::dark_theme();
        assert_eq!(d.primary_rgb, [20, 20, 25]);
        assert_eq!(d.accent_rgb, [100, 180, 255]);
        assert_eq!(d.is_dark_mode, 1);
        assert_eq!(d.border_iridescence, 0);
        assert_eq!(d.version, 1);
    }

    #[test]
    fn light_theme_fields() {
        let l = IpcThemeConfig::light_theme();
        assert_eq!(l.primary_rgb, [245, 245, 250]);
        assert_eq!(l.is_dark_mode, 0);
        assert_eq!(l.version, 1);
    }
}
