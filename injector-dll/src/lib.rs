use std::sync::OnceLock;
use windows::Win32::Foundation::{HINSTANCE, BOOL, TRUE, FALSE};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub mod theme_handler;
pub mod hook_procedures;
pub mod ipc_client;
pub mod iat_patcher;
pub mod message_handler;
pub mod pe_parser;

use theme_handler::ThemeHandler;
use ipc_client::IpcClient;

static THEME_HANDLER: OnceLock<ThemeHandler> = OnceLock::new();
static IPC_CLIENT: OnceLock<IpcClient> = OnceLock::new();

/// Controls the background theme polling thread lifetime.
/// Set to `true` on DLL attach, `false` on DLL detach.
static REFRESH_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

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

/// Read the current theme config from the host IPC server and populate the
/// hook's cached config.
///
/// Called once on DLL load so the taskbar receives the correct colors on the
/// first injection.  If the IPC client is unavailable or the read fails, this
/// is a no-op and the hook falls back to static defaults.
fn initialize_theme_from_ipc() {
    if let Some(ipc_client) = get_ipc_client() {
        if let Ok(config) = ipc_client.read_theme_config() {
            // Update the cached theme in hook procedures
            hook_procedures::update_cached_theme(config);
        }
    }
}

/// Spawn a background thread that polls IPC shared memory every 500 ms.
///
/// When `config.version` changes, the cached theme is updated and
/// `redraw_taskbar_windows()` is called to repaint Shell_TrayWnd.
///
/// `last_version` starts at `u32::MAX` so the first poll always triggers
/// an update, ensuring the taskbar gets the correct colors on injection.
///
/// # Safety note on spawning from DllMain
/// Spawning a thread from `DLL_PROCESS_ATTACH` while the loader lock is held
/// is safe here: the thread body only calls Win32 APIs and our own statics —
/// it never calls `LoadLibrary` or anything that re-enters the loader.
fn start_theme_refresh_thread() {
    REFRESH_THREAD_RUNNING.store(true, Ordering::SeqCst);

    std::thread::spawn(|| {
        let mut last_version: u32 = u32::MAX; // force first-poll update

        while REFRESH_THREAD_RUNNING.load(Ordering::Acquire) {
            if let Some(client) = get_ipc_client() {
                if let Ok(config) = client.read_theme_config() {
                    if config.version != last_version {
                        last_version = config.version;
                        hook_procedures::update_cached_theme(config);
                        let _ = message_handler::redraw_taskbar_windows();
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

/// Signal the background thread to exit on the next 500 ms tick.
/// Does NOT join — `DllMain` must not block.
fn stop_theme_refresh_thread() {
    REFRESH_THREAD_RUNNING.store(false, Ordering::SeqCst);
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

            // Initialize theme from IPC on load so the taskbar gets the
            // correct colors on the first injection.
            initialize_theme_from_ipc();

            // Install GetSysColor hook
            if let Err(e) = hook_procedures::install_hooks() {
                // Hook installation failed; log and continue — the DLL is still
                // functional without the hook (colors just won't be overridden).
                let _ = e; // TODO: surface via IPC once ipc_client is implemented
            }

            // Start background polling thread after hooks are installed.
            start_theme_refresh_thread();

            TRUE
        }
        0 => { // DLL_PROCESS_DETACH
            // Signal thread to stop before restoring the IAT.
            stop_theme_refresh_thread();

            // Restore original GetSysColor before we unload
            let _ = hook_procedures::uninstall_hooks();

            TRUE
        }
        _ => TRUE, // DLL_THREAD_ATTACH / DLL_THREAD_DETACH — not an error
    }
}

