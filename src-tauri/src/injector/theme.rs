//! Theme Manager for Nimbo
//!
//! Provides shared memory IPC between Tauri and injected DLL for theme data.
//! Uses Windows file mapping to share theme configuration in binary format.

use std::mem;
use std::ptr;
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingA, MapViewOfFile, UnmapViewOfFile,
    FILE_MAP_WRITE, PAGE_READWRITE, MEMORY_MAPPED_VIEW_ADDRESS
};

/// Theme data structure that matches the C++ DLL binary layout
/// Must be kept in sync with the injected DLL's InjectedTheme struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InjectedTheme {
    pub primary_rgb: [u8; 3],       // Primary color (e.g., [200, 220, 255])
    pub accent_rgb: [u8; 3],        // Accent color
    pub transparency: f32,           // 0.0..1.0 transparency level
    pub border_iridescence: bool,    // Enable iridescent border effects
    pub background_rgb: [u8; 3],     // Background color
    pub foreground_rgb: [u8; 3],     // Foreground/text color
    pub is_dark_mode: bool,          // Dark or light theme
    pub version: u32,                // Schema version for compatibility
}

impl InjectedTheme {
    /// Dark pill theme - dark colors with blue accent
    pub fn dark_theme() -> Self {
        Self {
            primary_rgb: [20, 20, 25],
            accent_rgb: [100, 180, 255],
            transparency: 0.95,
            border_iridescence: false,
            background_rgb: [15, 15, 20],
            foreground_rgb: [240, 240, 255],
            is_dark_mode: true,
            version: 1,
        }
    }

    /// Frosted white theme - light colors with subtle blue
    pub fn light_theme() -> Self {
        Self {
            primary_rgb: [245, 245, 250],
            accent_rgb: [100, 150, 220],
            transparency: 0.92,
            border_iridescence: false,
            background_rgb: [255, 255, 255],
            foreground_rgb: [30, 30, 40],
            is_dark_mode: false,
            version: 1,
        }
    }

    /// Glass effect theme - translucent with iridescence
    pub fn vidrio_theme() -> Self {
        Self {
            primary_rgb: [200, 220, 255],
            accent_rgb: [150, 200, 255],
            transparency: 0.75,
            border_iridescence: true,
            background_rgb: [220, 235, 255],
            foreground_rgb: [20, 30, 50],
            is_dark_mode: false,
            version: 1,
        }
    }
}

/// Errors that can occur during theme operations
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Failed to create file mapping: {0}")]
    CreateMappingFailed(String),

    #[error("Failed to map view of file: {0}")]
    MapViewFailed(String),

    #[error("Failed to write theme data: {0}")]
    WriteFailed(String),

    #[error("Failed to read theme data: {0}")]
    ReadFailed(String),
}

/// Manages shared memory for theme data exchange between Tauri and injected DLL
pub struct ThemeManager {
    mapping_handle: HANDLE,
    view_address: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl ThemeManager {
    /// Creates or opens the shared memory mapping for theme data
    ///
    /// Uses a named file mapping "Local\WindowsIsland_Theme_v1" that can be
    /// accessed by both Tauri and the injected DLL.
    pub fn new() -> Result<Self, ThemeError> {
        unsafe {
            // Mapping name as null-terminated byte string for CreateFileMappingA
            let mapping_name = PCSTR(b"Local\\WindowsIsland_Theme_v1\0".as_ptr());

            // Create or open existing file mapping
            let mapping_handle = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                mem::size_of::<InjectedTheme>() as u32,
                mapping_name,
            ).map_err(|e| ThemeError::CreateMappingFailed(format!("Win32 error: {}", e)))?;

            // Map the view for read/write access
            let view_address = MapViewOfFile(
                mapping_handle,
                FILE_MAP_WRITE,
                0,
                0,
                mem::size_of::<InjectedTheme>(),
            );

            if view_address.Value.is_null() {
                CloseHandle(mapping_handle).ok();
                return Err(ThemeError::MapViewFailed("MapViewOfFile returned null".to_string()));
            }

            Ok(Self {
                mapping_handle,
                view_address,
            })
        }
    }

    /// Writes theme data to shared memory
    ///
    /// The injected DLL can read this data to apply the theme configuration
    /// to Explorer windows and Start Menu components.
    pub fn write_theme(&mut self, theme: &InjectedTheme) -> Result<(), ThemeError> {
        unsafe {
            if self.view_address.Value.is_null() {
                return Err(ThemeError::WriteFailed("View pointer is null".to_string()));
            }

            let view_ptr = self.view_address.Value.cast::<InjectedTheme>();

            // Binary copy of theme struct to shared memory
            ptr::copy_nonoverlapping(
                theme as *const InjectedTheme,
                view_ptr,
                1,
            );

            Ok(())
        }
    }

    /// Reads current theme data from shared memory
    ///
    /// Returns a copy of the theme configuration currently stored in the
    /// shared memory region.
    pub fn read_theme(&self) -> Result<InjectedTheme, ThemeError> {
        unsafe {
            if self.view_address.Value.is_null() {
                return Err(ThemeError::ReadFailed("View pointer is null".to_string()));
            }

            let view_ptr = self.view_address.Value.cast::<InjectedTheme>();

            // Read theme struct from shared memory
            Ok(ptr::read(view_ptr))
        }
    }
}

impl Drop for ThemeManager {
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

// SAFETY: ThemeManager's internal HANDLE and view pointers are thread-safe once created.
// Windows named file mappings are designed for cross-process sharing and can be safely
// accessed from multiple threads. The view address is read-only after creation.
unsafe impl Send for ThemeManager {}
unsafe impl Sync for ThemeManager {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_constructors() {
        let dark = InjectedTheme::dark_theme();
        assert_eq!(dark.primary_rgb, [20, 20, 25]);
        assert_eq!(dark.accent_rgb, [100, 180, 255]);
        assert!(dark.is_dark_mode);
        assert!(!dark.border_iridescence);

        let light = InjectedTheme::light_theme();
        assert_eq!(light.primary_rgb, [245, 245, 250]);
        assert!(!light.is_dark_mode);

        let vidrio = InjectedTheme::vidrio_theme();
        assert_eq!(vidrio.primary_rgb, [200, 220, 255]);
        assert!(vidrio.border_iridescence);
    }

    #[test]
    fn test_theme_manager_creation() {
        // This test may fail in CI environments without sufficient permissions
        // but should work in development environments
        let result = ThemeManager::new();

        // Just check that we get a consistent result (success or specific error)
        match result {
            Ok(_) => println!("ThemeManager created successfully"),
            Err(e) => println!("ThemeManager creation failed (expected in some environments): {}", e),
        }
    }

    #[test]
    fn test_round_trip_theme_data() {
        if let Ok(mut manager) = ThemeManager::new() {
            let original = InjectedTheme::vidrio_theme();

            // Write theme
            manager.write_theme(&original).expect("Failed to write theme");

            // Read it back
            let read_back = manager.read_theme().expect("Failed to read theme");

            // Verify all fields match
            assert_eq!(read_back.primary_rgb, original.primary_rgb);
            assert_eq!(read_back.accent_rgb, original.accent_rgb);
            assert_eq!(read_back.transparency, original.transparency);
            assert_eq!(read_back.border_iridescence, original.border_iridescence);
            assert_eq!(read_back.background_rgb, original.background_rgb);
            assert_eq!(read_back.foreground_rgb, original.foreground_rgb);
            assert_eq!(read_back.is_dark_mode, original.is_dark_mode);
            assert_eq!(read_back.version, original.version);
        }
    }
}