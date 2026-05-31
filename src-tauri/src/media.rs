use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub is_playing: bool,
    pub has_session: bool,
    /// Current playback position in seconds.
    pub position_s: f64,
    /// Total track duration in seconds (0 if unknown / livestream).
    pub duration_s: f64,
}

// ─── Windows implementation ───────────────────────────────────────────────────
// Use spawn_blocking + synchronous IAsyncOperation polling.
// Avoids the Send/Future issues of WinRT objects in Tokio's multi-thread runtime.

#[cfg(windows)]
pub async fn get_media_info() -> MediaInfo {
    tokio::task::spawn_blocking(get_media_blocking)
        .await
        .unwrap_or_default()
}

#[cfg(windows)]
fn get_media_blocking() -> MediaInfo {
    use windows::{
        Foundation::AsyncStatus,
        Media::Control::{
            GlobalSystemMediaTransportControlsSessionManager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus,
        },
        Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    };

    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }

    // Request manager
    let async_op = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => op,
        Err(_) => return MediaInfo::default(),
    };

    // Poll until complete
    loop {
        match async_op.Status() {
            Ok(AsyncStatus::Completed) => break,
            Ok(AsyncStatus::Error) | Ok(AsyncStatus::Canceled) => return MediaInfo::default(),
            Err(_) => return MediaInfo::default(),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }

    let mgr = match async_op.GetResults() {
        Ok(m) => m,
        Err(_) => return MediaInfo::default(),
    };

    let session = match mgr.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return MediaInfo::default(),
    };

    // Properties
    let props_op = match session.TryGetMediaPropertiesAsync() {
        Ok(op) => op,
        Err(_) => return MediaInfo::default(),
    };
    loop {
        match props_op.Status() {
            Ok(AsyncStatus::Completed) => break,
            Ok(AsyncStatus::Error) | Ok(AsyncStatus::Canceled) => return MediaInfo::default(),
            Err(_) => return MediaInfo::default(),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    let props = match props_op.GetResults() {
        Ok(p) => p,
        Err(_) => return MediaInfo::default(),
    };

    let title  = props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();

    let is_playing = session
        .GetPlaybackInfo()
        .ok()
        .and_then(|pb| pb.PlaybackStatus().ok())
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false);

    // Timeline — current position + total duration.
    // TimeSpan::Duration is in 100-ns ticks; divide by 1e7 for seconds.
    let (position_s, duration_s) = session
        .GetTimelineProperties()
        .ok()
        .map(|tl| {
            let pos = tl.Position().map(|d| d.Duration as f64 / 1e7).unwrap_or(0.0);
            let end = tl.EndTime().map(|d| d.Duration as f64 / 1e7).unwrap_or(0.0);
            let start = tl.StartTime().map(|d| d.Duration as f64 / 1e7).unwrap_or(0.0);
            // Some apps report Position relative to StartTime (Spotify), others absolute.
            // Normalize by subtracting start so position is always relative to 0.
            let p = (pos - start).max(0.0);
            let d = (end - start).max(0.0);
            (p, d)
        })
        .unwrap_or((0.0, 0.0));

    MediaInfo {
        title, artist, is_playing, has_session: true,
        position_s, duration_s,
    }
}

#[cfg(windows)]
fn send_command(cmd: fn(&windows::Media::Control::GlobalSystemMediaTransportControlsSession)
    -> windows::core::Result<windows::Foundation::IAsyncOperation<bool>>) -> Result<(), String>
{
    use windows::{
        Foundation::AsyncStatus,
        Media::Control::GlobalSystemMediaTransportControlsSessionManager,
        Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    };

    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }

    let async_op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?;
    loop {
        match async_op.Status().map_err(|e| e.to_string())? {
            AsyncStatus::Completed => break,
            AsyncStatus::Error | AsyncStatus::Canceled => return Err("RequestAsync failed".into()),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    let session = async_op.GetResults().map_err(|e| e.to_string())?
        .GetCurrentSession().map_err(|e| e.to_string())?;

    let op = cmd(&session).map_err(|e| e.to_string())?;
    loop {
        match op.Status().map_err(|e| e.to_string())? {
            AsyncStatus::Completed => break,
            AsyncStatus::Error | AsyncStatus::Canceled => return Err("command failed".into()),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    Ok(())
}

#[cfg(windows)]
pub async fn toggle_play_pause() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        send_command(|s| s.TryTogglePlayPauseAsync())
    }).await.map_err(|e| e.to_string())?
}

#[cfg(windows)]
pub async fn skip_next() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        send_command(|s| s.TrySkipNextAsync())
    }).await.map_err(|e| e.to_string())?
}

#[cfg(windows)]
pub async fn skip_previous() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        send_command(|s| s.TrySkipPreviousAsync())
    }).await.map_err(|e| e.to_string())?
}

// ─── Non-Windows stubs ────────────────────────────────────────────────────────

#[cfg(not(windows))]
pub async fn get_media_info() -> MediaInfo { MediaInfo::default() }
#[cfg(not(windows))]
pub async fn toggle_play_pause() -> Result<(), String> { Ok(()) }
#[cfg(not(windows))]
pub async fn skip_next() -> Result<(), String> { Ok(()) }
#[cfg(not(windows))]
pub async fn skip_previous() -> Result<(), String> { Ok(()) }
