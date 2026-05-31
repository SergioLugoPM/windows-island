# Windows Island v0.3.0: DLL Injection Architecture

## Overview

Windows Island v0.3.0 introduces DLL injection capabilities to extend theme customization beyond our Tauri window to native Windows components, specifically the taskbar and Start Menu. This document outlines the technical architecture for implementing secure, reversible DLL injection that maintains system stability while enabling deep UI customization.

The primary goal is to inject custom theming logic into `explorer.exe` (which hosts the taskbar) and potentially other system processes to apply our dark themes consistently across the Windows shell.

## Design Goals

1. **System Safety**: All injections must be reversible and non-destructive
2. **Theme Consistency**: Extend our dark theme to taskbar, Start Menu, and system dialogs
3. **Performance**: Minimal impact on system performance and stability
4. **User Control**: Easy enable/disable through our Tauri interface
5. **Windows Compatibility**: Support Windows 10 (1903+) and Windows 11

## Architecture

### 1. Injector Module (`src/injector/`)

**File: `src/injector/dll_injector.rs`**
```rust
pub struct DllInjector {
    target_processes: Vec<String>,
    dll_path: PathBuf,
    injection_method: InjectionMethod,
}

pub enum InjectionMethod {
    SetWindowsHookEx,
    ManualDllLoad,
    ProcessHollowing, // For advanced cases
}

impl DllInjector {
    pub fn inject_into_process(&self, process_name: &str) -> Result<(), InjectionError>;
    pub fn remove_injection(&self, process_name: &str) -> Result<(), InjectionError>;
    pub fn verify_injection(&self, process_name: &str) -> bool;
}
```

**File: `src/injector/process_manager.rs`**
```rust
pub struct ProcessManager;

impl ProcessManager {
    pub fn find_target_processes() -> Vec<ProcessInfo>;
    pub fn is_process_safe_to_inject(pid: u32) -> bool;
    pub fn get_process_architecture(pid: u32) -> Architecture; // x86 vs x64
    pub fn monitor_process_lifecycle() -> ProcessMonitor;
}
```

### 2. Hook Manager (`src/hooks/`)

**File: `src/hooks/window_hooks.rs`**
```rust
pub struct WindowHookManager {
    active_hooks: HashMap<String, HookHandle>,
}

pub enum HookType {
    WndProc,       // Window procedure hooks
    Paint,         // Drawing/paint hooks
    Theme,         // Theme change notifications
    Accessibility, // For Start Menu access
}

impl WindowHookManager {
    pub fn install_hook(&mut self, hook_type: HookType, target: &str) -> Result<HookHandle>;
    pub fn uninstall_hook(&mut self, handle: HookHandle) -> Result<()>;
    pub fn list_active_hooks(&self) -> Vec<HookInfo>;
}
```

**File: `src/hooks/theme_hooks.rs`**
```rust
pub struct ThemeHook {
    original_handlers: HashMap<String, usize>, // Store original function pointers
}

impl ThemeHook {
    pub fn hook_draw_theme_background(&self) -> Result<()>;
    pub fn hook_draw_theme_text(&self) -> Result<()>;
    pub fn hook_get_sys_color(&self) -> Result<()>;
    pub fn restore_original_handlers(&self) -> Result<()>;
}
```

### 3. Theme Manager (`src/theme/`)

**File: `src/theme/injection_themes.rs`**
```rust
pub struct InjectionThemeManager {
    current_theme: Arc<RwLock<ThemeConfig>>,
    color_overrides: HashMap<i32, u32>, // SysColor index -> RGB override
}

impl InjectionThemeManager {
    pub fn apply_taskbar_theme(&self, theme: &TaskbarTheme) -> Result<()>;
    pub fn apply_start_menu_theme(&self, theme: &StartMenuTheme) -> Result<()>;
    pub fn override_system_colors(&self, overrides: &[(i32, u32)]) -> Result<()>;
    pub fn restore_original_theme(&self) -> Result<()>;
}

#[derive(Serialize, Deserialize)]
pub struct TaskbarTheme {
    background_color: String,
    text_color: String,
    accent_color: String,
    transparency: f32,
}

#[derive(Serialize, Deserialize)]
pub struct StartMenuTheme {
    background_color: String,
    tile_color: String,
    text_color: String,
    search_background: String,
}
```

**File: `src/theme/color_engine.rs`**
```rust
pub struct ColorEngine;

impl ColorEngine {
    pub fn rgb_to_colorref(rgb: &str) -> u32;
    pub fn apply_color_transform(original: u32, transform: &ColorTransform) -> u32;
    pub fn generate_accent_colors(base: u32) -> AccentColorSet;
    pub fn is_color_contrast_sufficient(fg: u32, bg: u32) -> bool;
}

pub struct ColorTransform {
    hue_shift: f32,
    saturation_multiply: f32,
    lightness_adjust: f32,
}
```

### 4. DLL Payload (`payload/`)

**Directory Structure:**
```
payload/
├── Cargo.toml
├── build.rs
└── src/
    ├── lib.rs
    ├── theme_handler.rs
    ├── hook_procedures.rs
    └── ipc_client.rs
```

**File: `payload/src/lib.rs`**
```rust
use std::os::windows::ffi::OsStringExt;
use winapi::um::winuser::*;

static mut ORIGINAL_WNDPROC: Option<unsafe extern "system" fn(HWND, u32, usize, isize) -> isize> = None;
static mut THEME_CONFIG: Option<ThemeConfig> = None;

#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void
) -> i32 {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            initialize_hooks();
        },
        DLL_PROCESS_DETACH => {
            cleanup_hooks();
        },
        _ => {}
    }
    1
}

fn initialize_hooks() {
    // Install window procedure hooks
    // Set up IPC connection to main Tauri app
    // Apply initial theme if available
}
```

**File: `payload/src/theme_handler.rs`**
```rust
pub struct ThemeHandler {
    ipc_client: IpcClient,
}

impl ThemeHandler {
    pub fn handle_paint_message(&self, hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize;
    pub fn override_system_color(&self, color_index: i32) -> u32;
    pub fn apply_dark_theme_to_window(&self, hwnd: HWND);
}
```

**File: `payload/src/ipc_client.rs`**
```rust
pub struct IpcClient {
    pipe_name: String,
}

impl IpcClient {
    pub fn connect_to_main_app(&self) -> Result<NamedPipe>;
    pub fn request_theme_config(&self) -> Result<ThemeConfig>;
    pub fn report_injection_status(&self, status: InjectionStatus) -> Result<()>;
}
```

### 5. Tauri Integration (`src-tauri/src/`)

**File: `src-tauri/src/injection_controller.rs`**
```rust
#[tauri::command]
pub async fn enable_system_theming(app_handle: AppHandle) -> Result<(), String> {
    let injector = DllInjector::new(get_payload_dll_path());
    
    // Inject into explorer.exe for taskbar
    injector.inject_into_process("explorer.exe")
        .map_err(|e| format!("Failed to inject into explorer.exe: {}", e))?;
    
    // Notify frontend
    app_handle.emit_all("injection-status", InjectionStatusEvent {
        process: "explorer.exe".to_string(),
        status: "active".to_string(),
    }).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn disable_system_theming() -> Result<(), String> {
    let injector = DllInjector::new(get_payload_dll_path());
    injector.remove_injection("explorer.exe")
        .map_err(|e| format!("Failed to remove injection: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_injection_status() -> Result<Vec<ProcessInjectionInfo>, String> {
    let process_manager = ProcessManager::new();
    Ok(process_manager.get_injection_status())
}
```

**File: `src-tauri/src/ipc_server.rs`**
```rust
pub struct IpcServer {
    pipe_server: NamedPipeServer,
    theme_config: Arc<RwLock<ThemeConfig>>,
}

impl IpcServer {
    pub fn start(&self) -> Result<()>;
    pub fn handle_client_request(&self, request: IpcRequest) -> IpcResponse;
    pub fn broadcast_theme_update(&self, config: &ThemeConfig) -> Result<()>;
}

pub enum IpcRequest {
    GetThemeConfig,
    ReportStatus(InjectionStatus),
    RequestColorOverride(i32), // SysColor index
}

pub enum IpcResponse {
    ThemeConfig(ThemeConfig),
    ColorValue(u32),
    Ack,
    Error(String),
}
```

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)
- Implement basic DLL injection using `SetWindowsHookEx`
- Create simple payload DLL that logs successful injection
- Set up IPC communication between main app and payload
- Test injection/removal cycle for stability

**Deliverables:**
- `DllInjector` basic implementation
- Minimal payload DLL that can be injected/removed
- IPC pipe communication working
- Unit tests for injection safety

### Phase 2: Theme Hook Implementation (Week 3-4)
- Implement window procedure hooks in payload DLL
- Add theme color override capabilities
- Create theme configuration management
- Integrate with existing Tauri theme system

**Deliverables:**
- `ThemeHandler` with paint message interception
- System color override functionality
- Theme configuration IPC
- Basic taskbar theming working

### Phase 3: UI Integration & Polish (Week 5-6)
- Add injection controls to Tauri frontend
- Implement monitoring and status reporting
- Add safety checks and error handling
- Performance optimization and testing

**Deliverables:**
- Frontend UI for enabling/disabling injection
- Real-time injection status monitoring
- Comprehensive error handling
- Performance benchmarks and optimization

## Security Considerations

### 1. Code Signing
- All DLL payloads must be code-signed to avoid Windows Defender issues
- Use authenticode certificates for production builds
- Implement certificate pinning for payload verification

### 2. Privilege Management
- Run injection with minimal required privileges
- Use process token checks before injection attempts
- Implement UAC elevation only when necessary

### 3. Anti-Virus Compatibility
- Whitelist payload DLL with major AV vendors during development
- Use well-known, documented injection techniques
- Avoid suspicious API calls that trigger heuristic detection

### 4. System Integrity
- Implement comprehensive rollback mechanisms
- Store original function pointers for all hooks
- Use structured exception handling in all hook procedures
- Monitor system stability and auto-disable on crashes

## Testing Strategy

### 1. Automated Testing
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_injection_cycle() {
        // Test inject -> verify -> remove cycle
    }
    
    #[test]
    fn test_theme_application() {
        // Test theme changes are applied correctly
    }
    
    #[test]
    fn test_system_stability() {
        // Stress test injection under load
    }
}
```

### 2. Integration Testing
- Test on fresh Windows VMs (10 & 11)
- Verify compatibility with Windows Updates
- Test interaction with third-party themes/customization tools
- Performance testing under various system loads

### 3. Security Testing
- Static analysis with Windows SDK tools
- Dynamic analysis for memory leaks/crashes
- Test rollback scenarios under failure conditions
- Verify clean uninstall leaves no system modifications

## Rollback Plan

In case of critical issues:

1. **Immediate Rollback**: Emergency disable command that removes all injections
2. **Safe Mode Detection**: Auto-disable injection if Windows boots in Safe Mode
3. **Crash Recovery**: Monitor for explorer.exe crashes and auto-disable on repeated crashes
4. **Registry Backup**: Store original theme registry values for complete restoration
5. **Uninstall Cleanup**: Remove all hooks, restore original function pointers, clean registry

## Open Questions / Future Work

1. **Windows 12 Compatibility**: How will this architecture adapt to future Windows versions?
2. **Performance Metrics**: What's the acceptable performance impact threshold?
3. **Third-party Integration**: How do we handle conflicts with other theming tools?
4. **Start Menu Complexity**: Windows 11 Start Menu may require different injection approach
5. **Dark Mode API**: Should we integrate with Windows native dark mode APIs where available?

---
**Document Created**: 2026-05-31  
**Status**: Design Phase - Ready for Implementation  
**Next Review**: After Phase 1 completion