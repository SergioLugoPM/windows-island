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
            pv_param:  *mut core::ffi::c_void, f_win_ini: u32,
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
        fn DwmEnableBlurBehindWindow(hwnd: isize, p_blur_behind: *const DwmBlurBehind) -> i32;
    }

    #[repr(C)]
    pub struct DwmBlurBehind {
        pub dw_flags: u32,
        pub f_enable: i32,
        pub h_rgn_blur: isize,
        pub f_transition_on_maximized: i32,
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(
            x1: i32, y1: i32, x2: i32, y2: i32,
            cx_corner: i32, cy_corner: i32,
        ) -> isize;
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

    /// Enable blur-behind clipped to a rounded-rectangle region matching the
    /// window's current pill shape. `w`/`h`/`radius` are PHYSICAL pixels.
    ///
    /// SAFETY / OWNERSHIP: once passed to DwmEnableBlurBehindWindow, the
    /// system takes ownership of the HRGN — do NOT call DeleteObject on it
    /// afterward (same rule as SetWindowRgn).
    pub fn enable_blur_behind(hwnd_isize: isize, w: i32, h: i32, radius: i32) -> bool {
        const DWM_BB_ENABLE: u32 = 0x1;
        const DWM_BB_BLURREGION: u32 = 0x2;
        let hrgn = unsafe { CreateRoundRectRgn(0, 0, w, h, radius * 2, radius * 2) };
        if hrgn == 0 { return false; }
        let bb = DwmBlurBehind {
            dw_flags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            f_enable: 1,
            h_rgn_blur: hrgn,
            f_transition_on_maximized: 0,
        };
        let hr = unsafe { DwmEnableBlurBehindWindow(hwnd_isize, &bb) };
        hr == 0 // S_OK
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
        unsafe { SystemParametersInfoW(0x0030, 0, &mut rect as *mut RECT as *mut core::ffi::c_void, 0); }
        rect.bottom
    }

    /// Read the current desktop wallpaper's file path (SPI_GETDESKWALLPAPER = 0x0073).
    /// Returns None if there is no wallpaper file (solid color background) or the
    /// call fails.
    pub fn wallpaper_path() -> Option<String> {
        const SPI_GETDESKWALLPAPER: u32 = 0x0073;
        const MAX_PATH: usize = 260;
        let mut buf: [u16; MAX_PATH] = [0; MAX_PATH];
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETDESKWALLPAPER, MAX_PATH as u32,
                buf.as_mut_ptr() as *mut core::ffi::c_void, 0,
            )
        };
        if ok == 0 { return None; }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        if len == 0 { return None; }
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    /// Returns false if another instance is already running (named mutex already exists).
    /// Call once at startup; intentionally leaks the handle so it lives for the process.
    pub fn claim_single_instance() -> bool {
        let name: Vec<u16> = "Local\\HaloW_SingleInstance_v1"
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

/// Returns the current Windows accent color and dark-mode setting so the
/// island can mirror the system theme.
///
/// Returns `{ is_dark: bool, accent_r: u8, accent_g: u8, accent_b: u8 }`.
#[tauri::command]
fn get_windows_theme() -> serde_json::Value {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        // ── Read dark/light mode ──────────────────────────────────────────────
        let is_dark = read_reg_dword(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "SystemUsesLightTheme",
        ).map(|v| v == 0).unwrap_or(true);

        // ── Read accent color (DWM stores it as 0xAABBGGRR) ─────────────────
        let accent_bgr = read_reg_dword(
            "Software\\Microsoft\\Windows\\DWM",
            "AccentColor",
        ).unwrap_or(0xFF_CC_99_33); // fallback: warm amber

        let r = ((accent_bgr >>  0) & 0xFF) as u8;
        let g = ((accent_bgr >>  8) & 0xFF) as u8;
        let b = ((accent_bgr >> 16) & 0xFF) as u8;

        return serde_json::json!({
            "is_dark": is_dark,
            "accent_r": r,
            "accent_g": g,
            "accent_b": b,
        });
    }
    #[cfg(not(target_os = "windows"))]
    serde_json::json!({ "is_dark": true, "accent_r": 100, "accent_g": 180, "accent_b": 255 })
}

/// Read a DWORD value from HKCU via raw Win32 (no winreg crate needed).
#[cfg(target_os = "windows")]
fn read_reg_dword(subkey: &str, value: &str) -> Option<u32> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn RegOpenKeyExW(
            h_key: isize, lp_sub_key: *const u16,
            ul_options: u32, sam_desired: u32, phk_result: *mut isize,
        ) -> i32;
        fn RegQueryValueExW(
            h_key: isize, lp_value_name: *const u16, lp_reserved: *const u32,
            lp_type: *mut u32, lp_data: *mut u8, lpcb_data: *mut u32,
        ) -> i32;
        fn RegCloseKey(h_key: isize) -> i32;
    }

    const HKCU: isize = -2147483647; // 0x80000001 as isize
    const KEY_READ: u32 = 0x20019;

    let sub_w: Vec<u16> = OsStr::new(subkey).encode_wide().chain(Some(0)).collect();
    let val_w: Vec<u16> = OsStr::new(value).encode_wide().chain(Some(0)).collect();

    unsafe {
        let mut hk: isize = 0;
        if RegOpenKeyExW(HKCU, sub_w.as_ptr(), 0, KEY_READ, &mut hk) != 0 {
            return None;
        }
        let mut data: u32 = 0;
        let mut size: u32 = 4;
        let mut typ: u32 = 0;
        let ok = RegQueryValueExW(
            hk, val_w.as_ptr(), std::ptr::null(),
            &mut typ, &mut data as *mut u32 as *mut u8, &mut size,
        );
        RegCloseKey(hk);
        if ok == 0 { Some(data) } else { None }
    }
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

/// Helper — resolve a window by label, falling back to "main".
fn island_win(app: &tauri::AppHandle, label: Option<&str>) -> Option<tauri::WebviewWindow> {
    app.get_webview_window(label.unwrap_or("main"))
}

#[tauri::command]
async fn set_cursor_passthrough(app: tauri::AppHandle, label: Option<String>, enabled: bool) {
    if let Some(win) = island_win(&app, label.as_deref()) {
        let _ = win.set_ignore_cursor_events(enabled);
    }
}

/// Toggle Mica backdrop (DWMWA_SYSTEMBACKDROP_TYPE = 38).
/// enabled → 2 (Mica, Win11 22H2+)  |  disabled → 1 (auto/off)
/// Called from JS when switching to/from the "glass" theme.
#[tauri::command]
async fn set_mica_effect(app: tauri::AppHandle, label: Option<String>, enabled: bool) {
    if let Some(win) = island_win(&app, label.as_deref()) {
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
async fn resize_anchor_bottom(app: tauri::AppHandle, label: Option<String>, w: f64, h: f64) {
    if let Some(win) = island_win(&app, label.as_deref()) {
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            let mx = monitor.position().x as f64 / scale;
            let my = monitor.position().y as f64 / scale;
            let sw = monitor.size().width as f64 / scale;
            let sh = monitor.size().height as f64 / scale;
            #[cfg(target_os = "windows")]
            let work_bottom = if monitor.position().x == 0 && monitor.position().y == 0 {
                win_sys::work_area_bottom() as f64 / scale
            } else {
                my + sh
            };
            #[cfg(not(target_os = "windows"))]
            let work_bottom = my + sh;
            let _ = win.set_size(tauri::LogicalSize::new(w, h));
            let _ = win.set_position(tauri::LogicalPosition::new(mx + (sw - w) / 2.0, work_bottom - h));
        }
    }
}

#[tauri::command]
async fn resize_window(app: tauri::AppHandle, label: Option<String>, w: f64, h: f64) {
    if let Some(win) = island_win(&app, label.as_deref()) {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            // Logical origin of this monitor in the virtual desktop.
            // For the primary monitor this is 0,0; for secondary monitors it is
            // their physical offset divided by scale.
            let mx = monitor.position().x as f64 / scale;
            let sw = monitor.size().width as f64 / scale;
            // Keep current Y (preserves top / floating position).
            let cur_y = win.outer_position()
                .map(|p| p.y as f64 / scale)
                .unwrap_or(monitor.position().y as f64 / scale);
            let _ = win.set_position(tauri::LogicalPosition::new(mx + (sw - w) / 2.0, cur_y));
        }
    }
}

#[tauri::command]
async fn snap_to_edge(app: tauri::AppHandle, label: Option<String>, edge: String, w: f64, h: f64) {
    if let Some(win) = island_win(&app, label.as_deref()) {
        if let Ok(Some(monitor)) = win.current_monitor() {
            let scale = monitor.scale_factor();
            // Logical origin + dimensions of this monitor.
            let mx = monitor.position().x as f64 / scale;
            let my = monitor.position().y as f64 / scale;
            let sw = monitor.size().width as f64 / scale;
            let sh = monitor.size().height as f64 / scale;
            let (nx, ny) = match edge.as_str() {
                "top" => (mx + (sw - w) / 2.0, my),
                "bottom" => {
                    // work_area_bottom is primary-monitor only; use monitor bottom for others.
                    #[cfg(target_os = "windows")]
                    let wb = if monitor.position().x == 0 && monitor.position().y == 0 {
                        win_sys::work_area_bottom() as f64 / scale
                    } else {
                        my + sh
                    };
                    #[cfg(not(target_os = "windows"))]
                    let wb = my + sh;
                    (mx + (sw - w) / 2.0, wb - h)
                }
                "left"  => (mx,           my + (sh - h) / 2.0),
                "right" => (mx + sw - w,  my + (sh - h) / 2.0),
                _       => (mx + (sw - w) / 2.0, my),
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

#[derive(serde::Serialize)]
struct AccentColor { r: u8, g: u8, b: u8 }

#[tauri::command]
async fn get_wallpaper_accent() -> Option<AccentColor> {
    #[cfg(target_os = "windows")]
    {
        let path = win_sys::wallpaper_path()?;
        let img = image::open(&path).ok()?;
        let small = img.resize_exact(16, 16, image::imageops::FilterType::Nearest).to_rgb8();
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for px in small.pixels() {
            r += px[0] as u64;
            g += px[1] as u64;
            b += px[2] as u64;
            n += 1;
        }
        if n == 0 { return None; }
        Some(AccentColor { r: (r / n) as u8, g: (g / n) as u8, b: (b / n) as u8 })
    }
    #[cfg(not(target_os = "windows"))]
    None
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

// ─── Multi-monitor support ─────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct MonitorInfo {
    /// Physical-pixel X origin of this monitor on the virtual desktop.
    x: i32,
    /// Physical-pixel Y origin of this monitor on the virtual desktop.
    y: i32,
    /// Physical-pixel width of this monitor.
    width: u32,
    /// Physical-pixel height of this monitor.
    height: u32,
    /// DPI scale factor (1.0 = 96 dpi, 1.5 = 144 dpi, 2.0 = 192 dpi, …).
    scale_factor: f64,
}

/// Returns the physical-pixel bounds and scale factor of the monitor that
/// contains the given island window (or "main" if label is None).
#[tauri::command]
async fn get_window_monitor(app: tauri::AppHandle, label: Option<String>) -> Option<MonitorInfo> {
    let win = island_win(&app, label.as_deref())?;
    let monitor = win.current_monitor().ok()??;
    Some(MonitorInfo {
        x:            monitor.position().x,
        y:            monitor.position().y,
        width:        monitor.size().width,
        height:       monitor.size().height,
        scale_factor: monitor.scale_factor(),
    })
}

// ─── Entry point ──────────────────────────────────────────────────────────────

/// Show or hide every island window in unison.
/// Decision is based on the "main" window: if visible → hide all; if hidden → show all.
fn toggle_all_islands(app: &tauri::AppHandle) {
    let main_visible = app.get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);

    for (label, win) in app.webview_windows() {
        if !label.starts_with("island") && label != "main" { continue; }
        if main_visible {
            let _ = win.hide();
        } else {
            let _ = win.show();
        }
    }
}

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

                // ── SPIKE: DwmEnableBlurBehindWindow go/no-go test ──────────
                // Unlike Mica, blur-behind is clipped via an HRGN, so it can
                // respect the pill's rounded shape. Purely additive/one-shot;
                // remove this block entirely if the spike is rejected.
                if let Ok(size) = win.inner_size() {
                    let ok = win_sys::enable_blur_behind(hwnd_raw, size.width as i32, size.height as i32, 32);
                    eprintln!("[SPIKE] blur-behind enable result: {}", ok);
                }
            }

            // ── Initial position — top center of primary monitor ──────────────
            if let Some(monitor) = win.current_monitor()? {
                let scale = monitor.scale_factor();
                let sw = monitor.size().width as f64 / scale;
                let win_w = 164.0_f64;
                win.set_position(tauri::LogicalPosition::new((sw - win_w) / 2.0, 0.0))?;
            }

            // ── Create one island window per secondary monitor ────────────────
            // primary_monitor() returns the monitor the OS considers "main".
            // We skip it because the "main" WebviewWindow (from tauri.conf.json)
            // is already placed there above.
            let primary_pos = app.primary_monitor()
                .ok()
                .flatten()
                .map(|m| (m.position().x, m.position().y));

            for (idx, monitor) in app.available_monitors()
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                // Skip the primary monitor — already covered by "main"
                let pos = (monitor.position().x, monitor.position().y);
                if Some(pos) == primary_pos { continue; }

                let label = format!("island_{}", idx);
                let scale = monitor.scale_factor();
                // Logical dimensions that match the default idle pill + MARGIN
                let win_w = 164.0_f64 + 4.0;
                let win_h = 64.0_f64 + 4.0;

                match tauri::WebviewWindowBuilder::new(
                    app,
                    &label,
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("")
                .inner_size(win_w, win_h)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .focused(false)
                .visible(true)
                .additional_browser_args(
                    "--default-background-color=00000000 \
                     --enable-features=msWebView2EnableDraggableRegions"
                )
                .build() {
                    Ok(sec_win) => {
                        // Position: top-center of this monitor (physical pixels)
                        let mx = monitor.position().x;
                        let my = monitor.position().y;
                        let mw = monitor.size().width as i32;
                        let pw = (win_w * scale) as i32;
                        let _ = sec_win.set_position(tauri::PhysicalPosition::new(
                            mx + (mw - pw) / 2,
                            my,
                        ));
                        // Disable Mica on secondary windows too
                        #[cfg(target_os = "windows")]
                        if let Ok(hwnd) = sec_win.hwnd() {
                            let hwnd_raw: isize = unsafe { std::mem::transmute_copy(&hwnd) };
                            win_sys::set_backdrop(hwnd_raw, 1);
                        }
                    }
                    Err(e) => eprintln!("[HaloW] Could not create window {}: {}", label, e),
                }
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
                    "toggle" => toggle_all_islands(app),
                    "quit"   => app.exit(0),
                    _        => {}
                })
                // ── Tray icon click ───────────────────────────────────────────
                .on_tray_icon_event(|tray, event| {
                    // Left click toggles visibility of all islands
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up, ..
                    } = event {
                        toggle_all_islands(tray.app_handle());
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
            get_windows_theme,
            get_window_monitor,
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
            get_wallpaper_accent,
            get_translation,
            set_locale,
        ])
        .run(tauri::generate_context!())
        .expect("error running windows-island");
}
