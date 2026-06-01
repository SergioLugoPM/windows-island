use windows::Win32::Foundation::{HINSTANCE, BOOL, TRUE, FALSE, CloseHandle};
use windows::Win32::System::Memory::{OpenFileMappingA, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ};
use windows::core::PCSTR;

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
            unsafe {
                on_dll_attach();
            }
            TRUE
        }
        0 => { // DLL_PROCESS_DETACH
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