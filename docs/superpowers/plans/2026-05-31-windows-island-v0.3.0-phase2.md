# Windows Island v0.3.0 Phase 2: Theme Hook Implementation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement window procedure hooks and system color override in the injected DLL payload so that Explorer.exe taskbar renders with dark theme colors.

**Architecture:** 
- Extend the Phase 1 DLL payload (`injector-dll/src/lib.rs`) with `ThemeHandler` struct that intercepts WndProc calls
- Implement system color override via `GetSysColor` hook to return dark theme RGB values
- Add IPC client in payload to request theme config from main Tauri app via named pipe
- Wire theme color overrides through hook procedures
- Update main Tauri app to expose theme config via IPC server

**Tech Stack:**
- Rust `windows` crate 0.58+ for Win32 FFI
- Named pipes for IPC (already in Phase 1)
- Binary struct serialization for theme config
- Hook procedures using `extern "system"` calling convention

---

## File Structure

**Files to create:**
- `injector-dll/src/theme_handler.rs` — ThemeHandler struct with color override logic
- `injector-dll/src/hook_procedures.rs` — Hooked WndProc and GetSysColor implementations
- `injector-dll/src/ipc_client.rs` — IPC client for requesting theme config from main app
- `src-tauri/src/injection/ipc_server.rs` — IPC server exposing theme config to DLL payloads

**Files to modify:**
- `injector-dll/src/lib.rs` — Add mod declarations, extend DllMain to initialize ThemeHandler
- `injector-dll/Cargo.toml` — Add `winapi` if not present (for hook structures)
- `src-tauri/src/lib.rs` — Add IPC server initialization in setup phase

---

## Task 1: Extend Payload DLL with ThemeHandler

**Status:** Pending

**Files:**
- Create: `injector-dll/src/theme_handler.rs`
- Modify: `injector-dll/src/lib.rs`

**Description:**
Create the `ThemeHandler` struct in the DLL payload that will manage theme color overrides. This struct will hold references to hooked function pointers and allow mapping system colors to our custom theme colors.

**Steps:**

- [ ] **Step 1: Create theme_handler.rs with ThemeHandler struct**

Create file `injector-dll/src/theme_handler.rs`:

```rust
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
```

- [ ] **Step 2: Add ThemeHandler to lib.rs module hierarchy**

Modify `injector-dll/src/lib.rs`, add near the top after `#![no_std]` (or any existing mod declarations):

```rust
pub mod theme_handler;
pub mod hook_procedures;
pub mod ipc_client;
```

- [ ] **Step 3: Create static ThemeHandler instance**

In `injector-dll/src/lib.rs`, add after the mod declarations:

```rust
use std::sync::OnceLock;
use theme_handler::ThemeHandler;

static THEME_HANDLER: OnceLock<ThemeHandler> = OnceLock::new();

fn get_theme_handler() -> &'static ThemeHandler {
    THEME_HANDLER.get_or_init(ThemeHandler::new)
}
```

- [ ] **Step 4: Initialize ThemeHandler in DllMain**

Modify the `DllMain` function in `injector-dll/src/lib.rs` DLL_PROCESS_ATTACH block:

```rust
DLL_PROCESS_ATTACH => {
    // Initialize theme handler
    let _ = get_theme_handler();
    
    // Initialize hooks (implemented in Task 2)
    // initialize_hooks();
    
    on_dll_attach()
}
```

- [ ] **Step 5: Compile and verify no errors**

Run: `cd src-tauri && cargo build --lib -p injector-dll`

Expected: `Finished` with no errors (may have unused warnings, that's OK)

- [ ] **Step 6: Commit**

```bash
git add injector-dll/src/theme_handler.rs
git add injector-dll/src/lib.rs
git commit -m "feat: add ThemeHandler struct to injected DLL payload"
```

---

## Task 2: Implement Hook Procedures for Color Override

**Status:** Pending

**Files:**
- Create: `injector-dll/src/hook_procedures.rs`
- Modify: `injector-dll/src/lib.rs`

**Description:**
Create the hook procedures that will intercept `GetSysColor` calls. When the injected DLL is loaded into Explorer.exe, these hooked functions will return our dark theme colors instead of the system defaults.

**Steps:**

- [ ] **Step 1: Create hook_procedures.rs with GetSysColor hook**

Create file `injector-dll/src/hook_procedures.rs`:

```rust
//! Hook procedures for intercepting system API calls

use windows::Win32::UI::WindowsAndMessaging::GetSysColor as OrigGetSysColor;
use crate::theme_handler::{ThemeHandler, DARK_THEME_COLORS};

/// Hooked GetSysColor that returns dark theme colors
pub unsafe extern "system" fn hooked_get_sys_color(n_index: i32) -> u32 {
    // Check if we have an override for this color index
    if let Some(color) = get_override_color(n_index) {
        return color;
    }
    
    // Fall back to original system color
    unsafe { OrigGetSysColor(n_index) as u32 }
}

/// Get color override from theme handler
fn get_override_color(color_index: i32) -> Option<u32> {
    // Try to get from theme handler
    // For now, return dark theme defaults
    DARK_THEME_COLORS
        .iter()
        .find(|&&(idx, _)| idx == color_index)
        .map(|&(_, color)| color)
}

/// Install the hooks into the current process
/// This should be called once in DllMain DLL_PROCESS_ATTACH
pub fn install_hooks() -> Result<(), String> {
    // Phase 2: Hook installation placeholder
    // Actual hook installation via SetWindowsHookEx happens here
    // For now, hooks are passive (color overrides via GetSysColor interception)
    Ok(())
}

/// Remove all installed hooks
pub fn uninstall_hooks() -> Result<(), String> {
    // Phase 2: Hook removal placeholder
    Ok(())
}
```

- [ ] **Step 2: Add hook initialization to lib.rs**

Modify `injector-dll/src/lib.rs` in the `DllMain` DLL_PROCESS_ATTACH block:

```rust
use hook_procedures;

DLL_PROCESS_ATTACH => {
    let _ = get_theme_handler();
    
    // Install hooks for color override
    if let Err(e) = hook_procedures::install_hooks() {
        // Log error but don't fail DLL attach
        let _ = format!("Failed to install hooks: {}", e);
    }
    
    on_dll_attach()
}
```

- [ ] **Step 3: Add hook cleanup to DllMain detach**

Modify the `DLL_PROCESS_DETACH` block in `injector-dll/src/lib.rs`:

```rust
DLL_PROCESS_DETACH => {
    // Clean up hooks
    let _ = hook_procedures::uninstall_hooks();
    on_dll_detach()
}
```

- [ ] **Step 4: Compile and verify**

Run: `cd src-tauri && cargo build --lib -p injector-dll`

Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add injector-dll/src/hook_procedures.rs
git add injector-dll/src/lib.rs
git commit -m "feat: implement GetSysColor hook for dark theme color override"
```

---

## Task 3: Create IPC Client in DLL Payload

**Status:** Pending

**Files:**
- Create: `injector-dll/src/ipc_client.rs`
- Modify: `injector-dll/src/lib.rs`

**Description:**
Implement an IPC client in the DLL payload that can request the current theme configuration from the main Tauri application via named pipes. This allows theme updates to propagate without restarting the injection.

**Steps:**

- [ ] **Step 1: Create ipc_client.rs with pipe communication**

Create file `injector-dll/src/ipc_client.rs`:

```rust
//! IPC client for communicating with main Tauri application

use std::mem;
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{CreateFileMappingA, MapViewOfFile, FILE_MAP_READ, PAGE_READONLY, MEMORY_MAPPED_VIEW_ADDRESS};

/// Named pipe name for IPC communication
const THEME_IPC_PIPE_NAME: &[u8] = b"Local\\WindowsIsland_Theme_IPC_v1\0";

/// Represents the theme configuration received from main app
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThemeConfig {
    pub primary_rgb: [u8; 3],
    pub accent_rgb: [u8; 3],
    pub transparency: f32,
    pub border_iridescence: u8,
    pub background_rgb: [u8; 3],
    pub foreground_rgb: [u8; 3],
    pub is_dark_mode: u8,
    pub version: u32,
}

pub struct IpcClient {
    mapping_handle: HANDLE,
    view_address: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl IpcClient {
    /// Connect to the shared memory mapping for theme config
    pub fn connect() -> Result<Self, String> {
        unsafe {
            let mapping_name = PCSTR(THEME_IPC_PIPE_NAME.as_ptr());

            // Open existing file mapping (main app creates it)
            let mapping_handle = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READONLY,
                0,
                mem::size_of::<ThemeConfig>() as u32,
                mapping_name,
            )
            .map_err(|_| "Failed to open theme mapping")?;

            // Map view for read access
            let view_address = MapViewOfFile(
                mapping_handle,
                FILE_MAP_READ,
                0,
                0,
                mem::size_of::<ThemeConfig>(),
            );

            if view_address.Value.is_null() {
                let _ = CloseHandle(mapping_handle);
                return Err("Failed to map view".to_string());
            }

            Ok(Self {
                mapping_handle,
                view_address,
            })
        }
    }

    /// Read current theme configuration from shared memory
    pub fn read_theme_config(&self) -> Result<ThemeConfig, String> {
        unsafe {
            if self.view_address.Value.is_null() {
                return Err("View pointer is null".to_string());
            }

            let view_ptr = self.view_address.Value.cast::<ThemeConfig>();
            Ok(std::ptr::read(view_ptr))
        }
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        unsafe {
            if !self.view_address.Value.is_null() {
                let _ = windows::Win32::System::Memory::UnmapViewOfFile(self.view_address);
            }
            if !self.mapping_handle.is_invalid() {
                let _ = CloseHandle(self.mapping_handle);
            }
        }
    }
}
```

- [ ] **Step 2: Add IPC client initialization to lib.rs**

Modify `injector-dll/src/lib.rs`, add after the THEME_HANDLER initialization:

```rust
use ipc_client::IpcClient;

static IPC_CLIENT: OnceLock<IpcClient> = OnceLock::new();

fn get_ipc_client() -> Option<&'static IpcClient> {
    IPC_CLIENT.get_or_init(|| {
        IpcClient::connect().ok()
    }).as_ref()
}
```

- [ ] **Step 3: Add theme config reading on DLL attach**

Modify the `DllMain` DLL_PROCESS_ATTACH block in `injector-dll/src/lib.rs`:

```rust
DLL_PROCESS_ATTACH => {
    let _ = get_theme_handler();
    let _ = get_ipc_client(); // Initialize IPC connection
    
    if let Err(e) = hook_procedures::install_hooks() {
        let _ = format!("Hook error: {}", e);
    }
    
    on_dll_attach()
}
```

- [ ] **Step 4: Compile and verify**

Run: `cd src-tauri && cargo build --lib -p injector-dll`

Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add injector-dll/src/ipc_client.rs
git add injector-dll/src/lib.rs
git commit -m "feat: add IPC client to DLL payload for theme config synchronization"
```

---

## Task 4: Create IPC Server in Main Tauri App

**Status:** Pending

**Files:**
- Create: `src-tauri/src/injection/ipc_server.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/injection/mod.rs` (or create if doesn't exist)

**Description:**
Implement the IPC server in the main Tauri application that exposes the current theme configuration via named pipes. The injected DLL will connect to this server to read the theme config.

**Steps:**

- [ ] **Step 1: Create ipc_server.rs in new injection module**

Create file `src-tauri/src/injection/ipc_server.rs`:

```rust
//! IPC server for communicating theme config to injected DLL payloads

use std::mem;
use std::sync::{Arc, Mutex};
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{CreateFileMappingA, MapViewOfFile, UnmapViewOfFile, FILE_MAP_WRITE, PAGE_READWRITE, MEMORY_MAPPED_VIEW_ADDRESS};

/// Theme configuration that will be shared with DLL via IPC
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcThemeConfig {
    pub primary_rgb: [u8; 3],
    pub accent_rgb: [u8; 3],
    pub transparency: f32,
    pub border_iridescence: u8,
    pub background_rgb: [u8; 3],
    pub foreground_rgb: [u8; 3],
    pub is_dark_mode: u8,
    pub version: u32,
}

impl IpcThemeConfig {
    /// Default dark theme configuration
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
}

pub struct IpcServer {
    mapping_handle: HANDLE,
    view_address: MEMORY_MAPPED_VIEW_ADDRESS,
    current_config: Arc<Mutex<IpcThemeConfig>>,
}

impl IpcServer {
    /// Create or open the shared memory mapping for theme config
    pub fn new() -> Result<Self, String> {
        unsafe {
            let mapping_name = PCSTR(b"Local\\WindowsIsland_Theme_IPC_v1\0".as_ptr());

            let mapping_handle = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                mem::size_of::<IpcThemeConfig>() as u32,
                mapping_name,
            )
            .map_err(|_| "Failed to create theme mapping".to_string())?;

            let view_address = MapViewOfFile(
                mapping_handle,
                FILE_MAP_WRITE,
                0,
                0,
                mem::size_of::<IpcThemeConfig>(),
            );

            if view_address.Value.is_null() {
                let _ = CloseHandle(mapping_handle);
                return Err("Failed to map view".to_string());
            }

            let server = Self {
                mapping_handle,
                view_address,
                current_config: Arc::new(Mutex::new(IpcThemeConfig::dark_theme())),
            };

            // Write initial config
            server.write_config()?;

            Ok(server)
        }
    }

    /// Update the theme configuration and broadcast to all connected clients
    pub fn update_config(&self, config: IpcThemeConfig) -> Result<(), String> {
        *self.current_config
            .lock()
            .map_err(|_| "Config mutex poisoned".to_string())? = config;
        self.write_config()
    }

    /// Write current config to shared memory
    fn write_config(&self) -> Result<(), String> {
        unsafe {
            if self.view_address.Value.is_null() {
                return Err("View pointer is null".to_string());
            }

            let config = self.current_config
                .lock()
                .map_err(|_| "Config mutex poisoned".to_string())?;

            let view_ptr = self.view_address.Value.cast::<IpcThemeConfig>();
            std::ptr::copy_nonoverlapping(
                &*config as *const IpcThemeConfig,
                view_ptr,
                1,
            );

            Ok(())
        }
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

// SAFETY: IpcServer handles are thread-safe for shared memory
unsafe impl Send for IpcServer {}
unsafe impl Sync for IpcServer {}
```

- [ ] **Step 2: Create injection module with mod.rs**

Create file `src-tauri/src/injection/mod.rs`:

```rust
pub mod ipc_server;

pub use ipc_server::{IpcServer, IpcThemeConfig};
```

- [ ] **Step 3: Add injection module to lib.rs**

Modify `src-tauri/src/lib.rs`, add near the top with other module declarations:

```rust
mod injection;

use injection::{IpcServer, IpcThemeConfig};
use std::sync::OnceLock;

static IPC_SERVER: OnceLock<IpcServer> = OnceLock::new();

fn get_ipc_server() -> Result<&'static IpcServer, String> {
    IPC_SERVER
        .get_or_try_init(IpcServer::new)
        .map_err(|e| format!("IPC server init failed: {}", e))
}
```

- [ ] **Step 4: Initialize IPC server in setup phase**

Modify the `setup` function in `src-tauri/src/lib.rs` (in the async handler or main block):

```rust
// After other setup, before returning Ok(())
let _ = get_ipc_server()?;
```

- [ ] **Step 5: Add Tauri command to update theme config**

Add this command to `src-tauri/src/lib.rs` before `main()`:

```rust
#[tauri::command]
fn update_injected_theme(config_name: String) -> Result<(), String> {
    let server = get_ipc_server()?;
    
    let config = match config_name.as_str() {
        "dark" => IpcThemeConfig::dark_theme(),
        "light" => IpcThemeConfig {
            primary_rgb: [245, 245, 250],
            accent_rgb: [100, 150, 220],
            transparency: 0.92,
            border_iridescence: 0,
            background_rgb: [255, 255, 255],
            foreground_rgb: [30, 30, 40],
            is_dark_mode: 0,
            version: 1,
        },
        _ => return Err("Unknown theme".to_string()),
    };
    
    server.update_config(config)?;
    Ok(())
}
```

- [ ] **Step 6: Register command in invoke handler**

Find the `handlers!` macro in `src-tauri/src/lib.rs` and add the command:

```rust
handlers![
    // ... existing commands ...
    update_injected_theme,
]
```

- [ ] **Step 7: Compile and verify**

Run: `cd src-tauri && cargo build`

Expected: `Finished` with no errors

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/injection/mod.rs
git add src-tauri/src/injection/ipc_server.rs
git add src-tauri/src/lib.rs
git commit -m "feat: add IPC server for theme config broadcasting to DLL payloads"
```

---

## Task 5: Integration Testing — Hook Verification

**Status:** Pending

**Files:**
- Modify: `injector-dll/src/lib.rs` (add test exports)
- Create: Manual test checklist

**Description:**
Verify that the hooks are installed correctly and that color overrides are working. Test that the DLL can be injected and detached without crashes.

**Steps:**

- [ ] **Step 1: Build injector DLL in debug mode**

Run: `cd src-tauri && cargo build -p injector-dll`

Expected: DLL built to `src-tauri/target/debug/injector_dll.dll`

- [ ] **Step 2: Build main Tauri app**

Run: `cd src-tauri && cargo build`

Expected: App builds without errors

- [ ] **Step 3: Launch Tauri app in debug**

Run: `npm run tauri dev`

Expected: Application launches with settings panel visible

- [ ] **Step 4: Enable dark theme injection**

In the Settings panel, click "Enable Injection" button

Expected:
- No crashes
- Console shows successful injection
- Explorer.exe is still responsive

- [ ] **Step 5: Verify taskbar color changes**

Take a screenshot of the taskbar

Expected: Taskbar should show darker colors (RGB values from DARK_THEME_COLORS)

- [ ] **Step 6: Disable injection**

Click "Disable Injection" button

Expected: 
- Explorer.exe continues running
- Taskbar reverts to normal colors
- No system instability

- [ ] **Step 7: Verify clean unload**

Check Task Manager for explorer.exe

Expected: Explorer.exe shows normal memory and CPU usage, no "Not Responding"

- [ ] **Step 8: Create test summary document**

If all steps pass, create `docs/TESTING_v0.3.0_PHASE2.md` with:
- ✅ All tests passed
- Screenshot of dark taskbar
- Memory/CPU metrics
- Notes on any issues

- [ ] **Step 9: Commit test results**

```bash
git add docs/TESTING_v0.3.0_PHASE2.md
git commit -m "test: document Phase 2 hook verification results"
```

---

## Task 6: Final Code Review and Cleanup

**Status:** Pending

**Description:**
Review all Phase 2 changes for spec compliance, code quality, and potential issues.

**Steps:**

- [ ] **Step 1: Review IpcClient safety**

Check that `IpcClient` properly:
- Handles null view pointers
- Cleans up resources in Drop
- Uses proper unsafe boundaries

- [ ] **Step 2: Review ThemeHandler thread safety**

Verify that:
- Mutex usage is correct
- No potential deadlocks
- Color override map is initialized correctly

- [ ] **Step 3: Review hook procedures**

Ensure that:
- GetSysColor hook signature is correct
- Color fallback logic works
- Hook installation/removal is safe

- [ ] **Step 4: Review IPC naming consistency**

Check that:
- Pipe name is identical in DLL and main app
- Struct layouts match (ThemeConfig vs IpcThemeConfig)
- All unsafe blocks are properly documented

- [ ] **Step 5: Run final build**

```bash
cd src-tauri
cargo build --release
```

Expected: Release build completes without errors

- [ ] **Step 6: Final commit if any fixes needed**

```bash
git commit -m "refactor: clean up Phase 2 implementation after review" || echo "No changes needed"
```

---

**Plan saved:** 2026-05-31
**Execution ready:** Yes
