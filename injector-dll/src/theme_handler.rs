//! Theme handler for intercepting system color calls in injected DLL

use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::HWND;

/// Maps system color indices to dark theme RGB values
pub static DARK_THEME_COLORS: &[(i32, u32)] = &[
    (0, 0x1a1a1a),  // COLOR_WINDOWTEXT
    (3, 0x1a1a1a),  // COLOR_WINDOW
    (4, 0x2d2d2d),  // COLOR_WINDOWFRAME
    (5, 0x0000ff),  // COLOR_MENUTEXT
    (8, 0x2d2d2d),  // COLOR_MENUHILIGHT
    (10, 0x646464), // COLOR_HIGHLIGHT
    (11, 0xffffff), // COLOR_HIGHLIGHTTEXT
    (12, 0x2d2d2d), // COLOR_BTNFACE
    (13, 0x808080), // COLOR_BTNSHADOW
    (14, 0xcccccc), // COLOR_BTNTEXT
];

/// Manages theme overrides for system colors
pub struct ThemeHandler {
    color_overrides: Mutex<HashMap<i32, u32>>,
}

impl ThemeHandler {
    /// Create a new theme handler with dark theme defaults
    pub fn new() -> Self {
        let mut overrides = HashMap::new();
        for &(index, color) in DARK_THEME_COLORS {
            overrides.insert(index, color);
        }

        Self {
            color_overrides: Mutex::new(overrides),
        }
    }

    /// Get the override color for a system color index, or None if not overridden
    pub fn get_override(&self, color_index: i32) -> Option<u32> {
        self.color_overrides
            .lock()
            .ok()
            .and_then(|map| map.get(&color_index).copied())
    }

    /// Set a color override
    pub fn set_override(&self, color_index: i32, color: u32) -> Result<(), String> {
        self.color_overrides
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))?
            .insert(color_index, color);
        Ok(())
    }

    /// Clear all overrides and revert to system defaults
    pub fn clear_overrides(&self) -> Result<(), String> {
        self.color_overrides
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))?
            .clear();
        Ok(())
    }

    /// Apply theme colors to a window (currently stored for later use)
    pub fn apply_to_window(&self, _hwnd: HWND) -> Result<(), String> {
        // In Phase 2, this stores the hwnd for later hook interception
        // Actual hooking happens in hook_procedures.rs
        Ok(())
    }
}

impl Default for ThemeHandler {
    fn default() -> Self {
        Self::new()
    }
}
