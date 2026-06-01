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

            TRUE
        }
        0 => { // DLL_PROCESS_DETACH
            // Restore original GetSysColor before we unload
            let _ = hook_procedures::uninstall_hooks();

            TRUE
        }
        _ => FALSE,
    }
}

