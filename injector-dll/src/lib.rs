use std::sync::OnceLock;
use windows::Win32::Foundation::{HINSTANCE, BOOL, TRUE, FALSE, CloseHandle};
use windows::Win32::System::Memory::{OpenFileMappingA, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ};
use windows::core::PCSTR;

pub mod theme_handler;
pub mod hook_procedures;
pub mod ipc_client;

use theme_handler::ThemeHandler;

static THEME_HANDLER: OnceLock<ThemeHandler> = OnceLock::new();

fn get_theme_handler() -> &'static ThemeHandler {
    THEME_HANDLER.get_or_init(ThemeHandler::new)
}

// Match the Rust InjectedTheme struct (from Task 2)
// Note: Using u8 for bools to ensure C-compatible binary layout
#[repr(C)]
struct InjectedTheme {
    primary_rgb: [u8; 3],
    accent_rgb: [u8; 3],
    transparency: f32,
    border_iridescence: u8,  // 0 = false, 1 = true
    background_rgb: [u8; 3],
    foreground_rgb: [u8; 3],
    is_dark_mode: u8,        // 0 = false, 1 = true
    version: u32,
}

#[no_mangle]
pub extern "system" fn DllMain(
    _module: HINSTANCE,
    call_reason: u32,
    _reserved: *const (),
) -> BOOL {
    match call_reason {
        1 => { // DLL_PROCESS_ATTACH
            // Initialize theme handler (populates DARK_THEME_COLORS into handler)
            let _ = get_theme_handler();

            // Install GetSysColor hook
            if let Err(e) = hook_procedures::install_hooks() {
                // Hook installation failed; log and continue — the DLL is still
                // functional without the hook (colors just won't be overridden).
                let _ = e; // TODO: surface via IPC once ipc_client is implemented
            }

            unsafe {
                on_dll_attach();
            }
            TRUE
        }
        0 => { // DLL_PROCESS_DETACH
            // Restore original GetSysColor before we unload
            let _ = hook_procedures::uninstall_hooks();

            unsafe {
                on_dll_detach();
            }
            TRUE
        }
        _ => FALSE,
    }
}

unsafe fn on_dll_attach() {
    // Try to read theme from shared memory
    let mapping_name = PCSTR::from_raw(b"Local\\WindowsIsland_Theme_v1\0".as_ptr());

    if let Ok(h_mapping) = OpenFileMappingA(FILE_MAP_READ.0, false, mapping_name) {
        let view = MapViewOfFile(h_mapping, FILE_MAP_READ, 0, 0, std::mem::size_of::<InjectedTheme>());
        if !view.Value.is_null() {
            let theme_ptr = view.Value as *const InjectedTheme;
            let theme = std::ptr::read(theme_ptr);
            // TODO: Log theme data or install hooks
            // For Phase 1, just verify we can read it
            let _ = theme.version;
            let _ = UnmapViewOfFile(view);
        }
        let _ = CloseHandle(h_mapping);
    }
}

unsafe fn on_dll_detach() {
    // Cleanup hooks (Phase 2)
}
