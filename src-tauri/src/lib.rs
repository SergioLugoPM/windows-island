pub mod cpu_temp;
pub mod i18n;
pub mod injector;
pub mod media;
pub mod stats;
pub mod weather;

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{Manager, State};

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
        /// Sets a DWM window attribute.  attr 38 = DWMWA_SYSTEMBACKDROP_TYPE.
        /// hwnd passed as isize — both our 0.58 HWND and Tauri's 0.61 HWND are
        /// repr(transparent) around isize, so transmute_copy is safe.
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
        unsafe { GetCursorPos(&mut pt); }
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

#[derive(Default)]
pub struct AppState {
    weather_cache: Arc<Mutex<Option<(weather::WeatherInfo, Instant)>>>,
    stats: Arc<stats::StatsState>,
    pub i18n: Arc<Mutex<i18n::I18n>>,
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

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

// ─── Entry point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Single-instance guard (release only — dev allows multiple for testing) ──
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if !win_sys::claim_single_instance() {
        return; // another instance is running — exit silently
    }

    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
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
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _result = tauri_plugin_updater::Builder::new()
                        .build()
                        .check_update()
                        .await;
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
