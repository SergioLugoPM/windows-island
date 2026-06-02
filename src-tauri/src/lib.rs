pub mod cpu_temp;
pub mod i18n;
pub mod injection;
pub mod injector;
pub mod media;
pub mod stats;
pub mod weather;

use std::{
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    time::Instant,
};
use tauri::{Manager, State};
use crate::injector::{Injector, theme::ThemeManager};
use crate::injection::{IpcServer, IpcThemeConfig};

// ─── IPC Server (theme config shared memory) ─────────────────────────────────
//
// `OnceLock::get_or_try_init` is not yet stable (tracking issue #109737).
// Instead we keep the server in a `Mutex<Option<IpcServer>>` and initialise
// it once, the first time any code calls `with_ipc_server`.

static IPC_SERVER: Mutex<Option<IpcServer>> = Mutex::new(None);

/// Initialise the IPC server on first call and call `f` with a reference.
/// Subsequent calls reuse the already-initialised server.
fn with_ipc_server<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&IpcServer) -> Result<T, String>,
{
    let mut guard = IPC_SERVER
        .lock()
        .map_err(|e| format!("IPC server mutex poisoned: {}", e))?;
    if guard.is_none() {
        *guard = Some(
            IpcServer::new().map_err(|e| format!("IPC server init failed: {}", e))?,
        );
    }
    f(guard.as_ref().unwrap())
}

/// Attempt to initialise the IPC server without returning the guard.
/// Used in the setup phase where we only want the side-effect of creation.
fn init_ipc_server() -> Result<(), String> {
    with_ipc_server(|_| Ok(()))
}

// ─── Raw Win32 FFI ─────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win_sys {
    use std::ffi::c_int;

    #[repr(C)] pub struct POINT { pub x: c_int, pub y: c_int }
    #[repr(C)] pub struct RECT  { pub left: c_int, pub top: c_int,
                                   pub right: c_int, pub bottom: c_int }

    extern "system" {
        pub fn GetCursorPos(lp_point: *mut POINT) -> i32;
        pub fn SystemParametersInfoW(
            ui_action: u32, ui_param: u32,
            pv_param:  *mut RECT, f_win_ini: u32,
        ) -> i32;
        pub fn DwmSetWindowAttribute(
            hwnd: isize, dw_attribute: u32,
            pv_attribute: *const core::ffi::c_void, cb_attribute: u32,
        ) -> i32;
        fn CreateMutexW(
            lp_attrs: *mut core::ffi::c_void,
            b_initial_owner: i32,
            lp_name:  *const u16,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
        // Changes the system color table entries and forces a WM_SYSCOLORCHANGE broadcast
        fn SetSysColors(
            n_changes:    c_int,
            lp_sys_color: *const c_int,
            lp_color_values: *const u32,
        ) -> i32;
        // Send a message to all top-level windows (HWND_BROADCAST = 0xFFFF)
        fn SendNotifyMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    }

    /// Apply a dark or light theme by writing directly to the Windows system
    /// color table via `SetSysColors`.  This is the approach that actually
    /// affects the taskbar on Windows 11.
    ///
    /// `SetSysColors` internally broadcasts `WM_SYSCOLORCHANGE` to all windows,
    /// so we don't need a separate broadcast for the color-table change.
    /// We send `WM_SETTINGCHANGE` as well so apps that watch settings are notified.
    pub fn apply_sys_colors(dark: bool) {
        // COLOR_* indices we want to override
        //  3 = COLOR_WINDOW        (window background)
        //  8 = COLOR_WINDOWTEXT    (window text)
        // 15 = COLOR_3DFACE        (button/dialog/taskbar face)
        // 16 = COLOR_3DSHADOW      (shadow edges)
        // 17 = COLOR_GRAYTEXT      (disabled text)
        // 18 = COLOR_HIGHLIGHT     (selected item background)
        // 19 = COLOR_HIGHLIGHTTEXT (selected item text)
        // 20 = COLOR_BTNFACE       (button face — alias for 3DFACE on Win11)
        let indices: [c_int; 8] = [3, 8, 15, 16, 17, 18, 19, 20];

        let (bg, text, face, shadow, gray, highlight, hl_text, btn): (u32,u32,u32,u32,u32,u32,u32,u32) =
            if dark {
                // Dark theme — charcoal tones
                (0x1a1a1a, 0xf0f0f0, 0x2d2d2d, 0x141414, 0x808080, 0x4a90d9, 0xffffff, 0x2d2d2d)
            } else {
                // Light theme — restore Windows defaults
                (0xffffff, 0x000000, 0xf0f0f0, 0xa0a0a0, 0x6d6d6d, 0x0078d7, 0xffffff, 0xf0f0f0)
            };

        // Win32 color values are 0x00BBGGRR (not 0x00RRGGBB)
        let to_bgr = |rgb: u32| -> u32 {
            let r = (rgb >> 16) & 0xFF;
            let g = (rgb >>  8) & 0xFF;
            let b =  rgb        & 0xFF;
            (b << 16) | (g << 8) | r
        };

        let values: [u32; 8] = [
            to_bgr(bg), to_bgr(text), to_bgr(face), to_bgr(shadow),
            to_bgr(gray), to_bgr(highlight), to_bgr(hl_text), to_bgr(btn),
        ];

        unsafe {
            SetSysColors(indices.len() as c_int, indices.as_ptr(), values.as_ptr());
            // WM_SETTINGCHANGE (0x001A) with "ImmersiveColorSet" notifies modern apps
            let param = "ImmersiveColorSet\0"
                .encode_utf16()
                .collect::<Vec<u16>>();
            SendNotifyMessageW(0xFFFF, 0x001A, 0, param.as_ptr() as isize);
        }
    }

    /// Restore default Windows system colors (light theme).
    pub fn restore_sys_colors() {
        apply_sys_colors(false);
    }

    /// Enable or disable the Mica backdrop effect on a window handle.
    /// backdrop_type: 1 = auto/off, 2 = Mica, 3 = Acrylic
    pub fn set_backdrop(hwnd_isize: isize, backdrop_type: u32) {
        unsafe {
            DwmSetWindowAttribute(
                hwnd_isize, 38u32,
                &backdrop_type as *const u32 as *const core::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    pub fn cursor_pos() -> (i32, i32) {
        let mut pt = POINT { x: 0, y: 0 };
        // GetCursorPos returns 0 on failure (e.g. no desktop access).
        // Return a sentinel cy = i32::MAX so the edge-detection threshold
        // (cy < 8–12 px) is never falsely triggered.
        let ok = unsafe { GetCursorPos(&mut pt) };
        if ok == 0 { return (0, i32::MAX); }
        (pt.x, pt.y)
    }

    /// Bottom of the work area (above taskbar) — physical pixels on DPI-aware process.
    pub fn work_area_bottom() -> i32 {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe { SystemParametersInfoW(0x0030, 0, &mut rect, 0); }
        rect.bottom
    }

    /// Returns false if another instance is already running (named mutex already exists).
    /// Call once at startup; intentionally leaks the handle so it lives for the process.
    pub fn claim_single_instance() -> bool {
        let name: Vec<u16> = "Local\\WindowsIsland_SingleInstance_v1"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let h = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) };
        if h.is_null() { return true; } // CreateMutex failed — allow startup anyway
        let err = unsafe { GetLastError() };
        err != 183 // ERROR_ALREADY_EXISTS = 183
    }
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct AppState {
    weather_cache: Arc<Mutex<Option<(weather::WeatherInfo, Instant)>>>,
    stats: Arc<stats::StatsState>,
    pub i18n: Arc<Mutex<i18n::I18n>>,
    pub injector: Arc<Injector>,
    pub theme_manager: Arc<Mutex<ThemeManager>>,
    pub injection_active: Arc<AtomicBool>,
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn enable_theme_injection(
    state: tauri::State<'_, AppState>,
    theme_name: String,
) -> Result<(), String> {
    if state.injection_active.load(Ordering::Relaxed) {
        return Ok(()); // Already active
    }

    // Select theme
    let theme = match theme_name.as_str() {
        "dark" => injector::theme::InjectedTheme::dark_theme(),
        "light" => injector::theme::InjectedTheme::light_theme(),
        "vidrio" => injector::theme::InjectedTheme::vidrio_theme(),
        _ => injector::theme::InjectedTheme::dark_theme(),
    };

    // Write theme to shared memory
    state.theme_manager
        .lock()
        .unwrap()
        .write_theme(&theme)
        .map_err(|e| format!("Failed to write theme: {}", e))?;

    // Inject into Explorer
    state.injector
        .inject_into_explorer()
        .map_err(|e| match e {
            injector::InjectorError::OpenProcessFailed(_) =>
                "Administrator required — right-click Windows Island and select 'Run as administrator'".to_string(),
            injector::InjectorError::DllNotFound =>
                "Injector DLL not found. Try rebuilding the app.".to_string(),
            other => format!("Injection failed: {:?}", other),
        })?;

    // Inject into StartMenuExperienceHost (Win11)
    let _ = state.injector.inject_into_startmenu();

    // Apply system colors directly — this is what actually changes the taskbar
    // on Windows 11, which uses DWM/Mica rather than GetSysColor for rendering.
    #[cfg(target_os = "windows")]
    win_sys::apply_sys_colors(theme_name != "light");

    state.injection_active.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn disable_theme_injection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.injection_active.store(false, Ordering::Relaxed);
    // Restore default Windows system colors
    #[cfg(target_os = "windows")]
    win_sys::restore_sys_colors();
    Ok(())
}

#[tauri::command]
fn is_injection_active(state: tauri::State<'_, AppState>) -> bool {
    state.injection_active.load(Ordering::Relaxed)
}

#[tauri::command]
fn get_cursor_screen_pos() -> (i32, i32) {
    #[cfg(target_os = "windows")]
    return win_sys::cursor_pos();
    #[cfg(not(target_os = "windows"))]
    (0, 0)
}

#[tauri::command]
fn get_work_area_bottom() -> i32 {
    #[cfg(target_os = "windows")]
    return win_sys::work_area_bottom();
    #[cfg(not(target_os = "windows"))]
    0
}

#[tauri::command]
async fn set_cursor_passthrough(app: tauri::AppHandle, enabled: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_ignore_cursor_events(enabled);
    }
}

/// Toggle Mica backdrop (DWMWA_SYSTEMBACKDROP_TYPE = 38).
/// enabled → 2 (Mica, Win11 22H2+)  |  disabled → 1 (auto/off)
/// Called from JS when switching to/from the "glass" theme.
#[tauri::command]
async fn set_mica_effect(app: tauri::AppHandle, enabled: bool) {
    if let Some(win) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        if let Ok(hwnd) = win.hwnd() {
            // SAFETY: HWND is repr(transparent) around isize in all recent
            // windows crate versions; transmute_copy extracts the raw value.
            let hwnd_raw: isize = unsafe { std::mem::transmute_copy(&hwnd) };
            win_sys::set_backdrop(hwnd_raw, if enabled { 2 } else { 1 });
        }
    }
}

/// Resize keeping bottom edge anchored to work area (above taskbar).
#[tauri::command]
async fn resize_anchor_bottom(app: tauri::AppHandle, w: f64, h: f64) {
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            let sw = monitor.size().width as f64 / scale;
            #[cfg(target_os = "windows")]
            let work_bottom = win_sys::work_area_bottom() as f64 / scale;
            #[cfg(not(target_os = "windows"))]
            let work_bottom = monitor.size().height as f64 / scale;
            let _ = win.set_size(tauri::LogicalSize::new(w, h));
            let _ = win.set_position(tauri::LogicalPosition::new((sw - w) / 2.0, work_bottom - h));
        }
    }
}

#[tauri::command]
async fn resize_window(app: tauri::AppHandle, w: f64, h: f64) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            let sw = monitor.size().width as f64 / scale;
            let cur_y = win.outer_position()
                .map(|p| p.y as f64 / scale)
                .unwrap_or(4.0);
            let _ = win.set_position(tauri::LogicalPosition::new((sw - w) / 2.0, cur_y));
        }
    }
}

#[tauri::command]
async fn snap_to_edge(app: tauri::AppHandle, edge: String, w: f64, h: f64) {
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            let sw = monitor.size().width as f64 / scale;
            let sh = monitor.size().height as f64 / scale;
            let (nx, ny) = match edge.as_str() {
                "top" => ((sw - w) / 2.0, 0.0),
                "bottom" => {
                    #[cfg(target_os = "windows")]
                    let wb = win_sys::work_area_bottom() as f64 / scale;
                    #[cfg(not(target_os = "windows"))]
                    let wb = sh;
                    ((sw - w) / 2.0, wb - h)
                }
                "left"  => (0.0, (sh - h) / 2.0),
                "right" => (sw - w, (sh - h) / 2.0),
                _       => ((sw - w) / 2.0, 0.0),
            };
            let _ = win.set_size(tauri::LogicalSize::new(w, h));
            let _ = win.set_position(tauri::LogicalPosition::new(nx, ny));
        }
    }
}

#[tauri::command]
async fn get_media_info() -> media::MediaInfo { media::get_media_info().await }

#[tauri::command]
async fn toggle_play_pause() -> Result<(), String> { media::toggle_play_pause().await }

#[tauri::command]
async fn skip_next() -> Result<(), String> { media::skip_next().await }

#[tauri::command]
async fn skip_previous() -> Result<(), String> { media::skip_previous().await }

#[tauri::command]
fn get_system_stats(state: State<'_, AppState>) -> stats::SystemStats {
    stats::collect(&state.stats)
}

#[tauri::command]
async fn get_weather(city: String, state: State<'_, AppState>) -> Result<weather::WeatherInfo, String> {
    {
        let cache = state.weather_cache.lock().unwrap();
        if let Some((ref info, ref ts)) = *cache {
            if ts.elapsed().as_secs() < 1800 { return Ok(info.clone()); }
        }
    }
    let info = weather::get_weather(&city).await?;
    *state.weather_cache.lock().unwrap() = Some((info.clone(), Instant::now()));
    Ok(info)
}

#[tauri::command]
fn get_translation(state: State<'_, AppState>, key: String) -> String {
    state.i18n.lock().unwrap().t(&key)
}

#[tauri::command]
fn set_locale(state: State<'_, AppState>, locale: String) {
    state.i18n.lock().unwrap().set_locale(&locale);
}

/// Push a theme configuration to the injected DLL via the IPC shared memory.
///
/// Accepted `config_name` values: `"dark"`, `"light"`.
/// Returns `Err` for unknown names or if the IPC server fails to initialise.
#[tauri::command]
fn update_injected_theme(config_name: String) -> Result<(), String> {
    let config = match config_name.as_str() {
        "dark" => IpcThemeConfig::dark_theme(),
        "light" => IpcThemeConfig::light_theme(),
        _ => return Err(format!("Unknown theme '{}'; expected 'dark' or 'light'", config_name)),
    };
    with_ipc_server(|server| server.update_config(config))
}

/// Signal the injected DLL to re-read the IPC config.
///
/// In a future phase, this could use a pipe / event to wake the DLL.
/// For now it is a placeholder that returns success so the frontend can
/// complete the "theme change → IPC update → DLL refresh" loop.
#[tauri::command]
fn refresh_injected_theme_config() -> Result<(), String> {
    Ok(())
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Single-instance guard (release only — dev allows multiple for testing) ──
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if !win_sys::claim_single_instance() {
        return; // another instance is running — exit silently
    }

    // Get or compute DLL path
    let dll_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("windows_island_injector_dll.dll"))
        .unwrap_or_else(|| "windows_island_injector_dll.dll".into());

    // Create injector
    let injector = Arc::new(Injector::new(dll_path));

    // Create theme manager
    let theme_manager = Arc::new(Mutex::new(
        ThemeManager::new()
            .expect("Failed to initialize theme manager")
    ));

    // Atomic bool for injection state
    let injection_active = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .manage(AppState {
            weather_cache: Arc::new(Mutex::new(None)),
            stats: Arc::new(stats::StatsState::default()),
            i18n: Arc::new(Mutex::new(i18n::I18n::default())),
            injector,
            theme_manager,
            injection_active,
        })
        .setup(|app| {
            // ── IPC server init (shared memory for DLL theme config) ──────────
            // Initialise early so the mapping exists before the DLL is injected.
            if let Err(e) = init_ipc_server() {
                eprintln!("[IPC] Warning: could not start IPC server: {}", e);
            }

            let win = app.get_webview_window("main").unwrap();
            win.set_always_on_top(true)?;

            // ── Force-disable Mica on startup ──────────────────────────────
            // DWM may have cached Mica from a previous run / debug session,
            // and Mica paints the full window rectangle ignoring border-radius
            // (visible as rectangular corner artifacts around the pill).
            #[cfg(target_os = "windows")]
            if let Ok(hwnd) = win.hwnd() {
                let hwnd_raw: isize = unsafe { std::mem::transmute_copy(&hwnd) };
                win_sys::set_backdrop(hwnd_raw, 1); // 1 = auto/none
            }

            // Initial position — top center
            if let Some(monitor) = win.current_monitor()? {
                let scale = monitor.scale_factor();
                let sw = monitor.size().width as f64 / scale;
                let win_w = 164.0_f64;
                win.set_position(tauri::LogicalPosition::new((sw - win_w) / 2.0, 0.0))?;
            }

            // ── Tray icon + context menu ──────────────────────────────────────
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let toggle = MenuItem::with_id(app, "toggle", "Mostrar / Ocultar", true, None::<&str>)?;
            let sep    = PredefinedMenuItem::separator(app)?;
            let quit   = MenuItem::with_id(app, "quit",   "Salir",             true, None::<&str>)?;
            let menu   = Menu::with_items(app, &[&toggle, &sep, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Windows Island")
                .menu(&menu)
                .show_menu_on_left_click(false)
                // ── Menu item selected ────────────────────────────────────────
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                // ── Tray icon click ───────────────────────────────────────────
                .on_tray_icon_event(|tray, event| {
                    // Left click toggles visibility
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up, ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                    // Right click is handled by the menu automatically
                })
                .build(app)?;

            // ── Auto-updater (release builds only) ──────────────────────────────
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_updater::UpdaterExt;
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = handle.updater_builder().build()
                        .and_then(|u| Ok(u))
                        .map(|_| ());
                    // check() requires an active endpoint — skip silently if unconfigured
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update_injected_theme,
            refresh_injected_theme_config,
            enable_theme_injection,
            disable_theme_injection,
            is_injection_active,
            resize_window,
            resize_anchor_bottom,
            snap_to_edge,
            set_cursor_passthrough,
            set_mica_effect,
            get_cursor_screen_pos,
            get_work_area_bottom,
            get_media_info,
            toggle_play_pause,
            skip_next,
            skip_previous,
            get_weather,
            get_system_stats,
            get_translation,
            set_locale,
        ])
        .run(tauri::generate_context!())
        .expect("error running windows-island");
}
