use std::sync::OnceLock;
use windows::Win32::Foundation::{HINSTANCE, BOOL, TRUE, FALSE, CloseHandle};
use windows::Win32::System::Memory::{OpenFileMappingA, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ};
use windows::core::PCSTR;

pub mod theme_handler;
pub mod hook_procedures;
pub mod ipc_client;
pub mod iat_patcher;

use theme_handler::ThemeHandler;
use ipc_client::IpcClient;

static THEME_HANDLER: OnceLock<ThemeHandler> = OnceLock::new();
static IPC_CLIENT: OnceLock<IpcClient> = OnceLock::new();

fn get_theme_handler() -> &'static ThemeHandler {
    THEME_HANDLER.get_or_init(ThemeHandler::new)
}

/// Attempt a one-time connection to the host IPC shared memory region.
///
/// Returns a reference to the `IpcClient` on success, or `None` if the host
/// app has not yet created the file mapping (e.g. the DLL was injected before
/// the Tauri process started, or connection failed for another reason).
///
/// The connection attempt is made at most once per DLL load.  Subsequent calls
/// return the cached result without any Win32 syscall.
fn get_ipc_client() -> Option<&'static IpcClient> {
    // `OnceLock::set` succeeds only on the first call; subsequent calls are
    // no-ops.  If connect() fails we simply leave the lock empty and return
    // None, which the caller must handle gracefully.
    if IPC_CLIENT.get().is_none() {
        if let Ok(client) = IpcClient::connect() {
            // Ignore error from set() — it means another thread raced us and
            // already stored a client, which is fine.
            let _ = IPC_CLIENT.set(client);
        }
    }
    IPC_CLIENT.get()
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

            // Attempt to connect to the host app's IPC shared memory.
            // This may return None if the host mapping is not yet available;
            // that is non-fatal — the hook will fall back to static defaults.
            let _ = get_ipc_client();

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
