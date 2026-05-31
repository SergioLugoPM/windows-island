# Windows Island

> Apple-style Dynamic Island for Windows — an always-on-top, edge-snapping pill widget with clock, weather, media controls and three themes (Dark, Light, Liquid Glass). Built with **Tauri 2 + React + TypeScript + Rust**.

[![Platform](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white)](https://github.com/SergioLugoPM/windows-island/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-CE412B?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Release](https://img.shields.io/github/v/release/SergioLugoPM/windows-island)](../../releases/latest)

---

<!--
  TODO — replace with real screenshots / GIFs:

  ![Hero](docs/screenshots/hero.png)
  ![Themes](docs/screenshots/themes.png)
  ![Demo](docs/screenshots/demo.gif)
-->

## Why?

Windows' built-in clock gets hidden by fullscreen apps. The taskbar widgets are heavy. Existing Dynamic Island clones for Windows are either abandoned or feel like browser extensions. **Windows Island** is a 3.3 MB native widget that:

- Floats above everything (`set_always_on_top`)
- Ignores your mouse until you reach the screen edge (`SetIgnoreCursorEvents` + `GetCursorPos` polling)
- Snaps to a pill shape on the top edge or floats anywhere you drop it
- Renders three full visual themes with real CSS/SVG glass effects
- Reads the actual Windows Media Session (`Windows.Media.Control` WinRT API) — works with Spotify, YouTube, Edge, anything

## Features

### Three view modes (cycle by click)
| Mode | Shows |
|---|---|
| **Idle** | Just the clock, transparent at 72% opacity. Mouse passes through |
| **Peek** | Clock + weather *or* media (configurable) |
| **Media** | Vinyl disc animation + real audio visualizer + track info + transport buttons |
| **Full** | Clock + weather + media compact, all at once |
| **Settings** | Long-press to open |

### Three themes
| Theme | Look |
|---|---|
| **Dark** | Solid dark pill with blue accent glow — default |
| **Light** | Frosted white pill with soft shadow |
| **Glass** | Semi-transparent pill with iridescent conic border, chromatic aberration, oval specular highlight, animated caustics (SVG `feTurbulence` + `feDisplacementMap`) |

### Clock sizes
- **S** (140×52) / **M** (160×64) / **L** (184×80) — resizes the OS window in real time

### Position modes
- **↑ Edge** — Snapped to top edge, cursor passes through until it touches the edge zone (8 px), then expands
- **Libre** — Drag-anywhere; always reacts to hover

### System integration
- **Tray icon** with right-click menu (Show / Hide / Quit)
- **Single-instance lock** via named mutex (`CreateMutexW`)
- **Drag to move** with `startDragging` Tauri API; auto-snaps on release
- **Esc** to collapse from any mode

## Install

Grab the latest installer from [**Releases**](../../releases/latest):

| File | Size | Notes |
|---|---|---|
| `Windows Island_0.1.0_x64_en-US.msi` | 3.3 MB | Standard Windows installer |
| `Windows Island_0.1.0_x64-setup.exe` | 2.4 MB | NSIS (smaller, faster) |

> **Note:** the binary is not code-signed yet, so Windows SmartScreen will show a warning the first time. Click "More info" → "Run anyway". If you'd rather verify the source, build from scratch (see below).

### System requirements
- Windows 10 (build 19041+) or Windows 11
- WebView2 Runtime — preinstalled on Win11, auto-installed on Win10

## Build from source

```bash
git clone https://github.com/SergioLugoPM/windows-island
cd windows-island
npm install
cargo tauri dev     # hot-reload dev mode
cargo tauri build   # release installers in src-tauri/target/release/bundle/
```

Prerequisites:
- Rust stable (`rustup default stable`)
- Node 18+
- Windows 10 SDK + MSVC build tools

## Controls reference

| Action | Effect |
|---|---|
| Cursor reaches screen top (idle, edge mode) | Expand to peek |
| Click on island | Cycle modes (peek → media → full → idle) |
| Long-press (600 ms) | Open settings panel |
| Drag | Move window (floating) or re-snap to edge |
| Esc | Instantly collapse to idle |
| Tray left-click | Toggle visibility |
| Tray right-click | Context menu (Show / Hide / Quit) |

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│  WebView2  (transparent, no decorations, always-on-top, 164×68)│
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  React 18 + Framer Motion                                │  │
│  │  ┌──── Island.tsx (state machine) ────────────────────┐  │  │
│  │  │   LiquidBackground   LiquidGlassChrome             │  │  │
│  │  │   Clock  WeatherView  MediaView  AudioVisualizer   │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ▲  invoke('cmd', …)                                           │
└──┼─────────────────────────────────────────────────────────────┘
   │
┌──┴─────────────────────────────────────────────────────────────┐
│  Tauri 2 / Rust backend                                        │
│  ┌──── lib.rs ──────────────────────────────────────────────┐  │
│  │  resize_window / resize_anchor_bottom / snap_to_edge     │  │
│  │  set_cursor_passthrough  get_cursor_screen_pos           │  │
│  │  get_work_area_bottom    set_mica_effect                 │  │
│  │  get_media_info / toggle_play_pause / skip_*             │  │
│  │  get_weather (30-min cache)                              │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──── win_sys module (raw Win32 FFI) ──────────────────────┐  │
│  │  GetCursorPos  SystemParametersInfoW                     │  │
│  │  DwmSetWindowAttribute  CreateMutexW                     │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──── media.rs ────────────────────────────────────────────┐  │
│  │  Windows.Media.Control via `windows` crate 0.58          │  │
│  │  spawn_blocking + sync polling (IAsyncOperation is !Send)│  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Tech stack

| Layer | Tech |
|---|---|
| Native shell | Tauri 2 (Rust) |
| Frontend | React 18, TypeScript, Vite |
| Animation | Framer Motion (spring physics) |
| Media (Win) | `windows` crate 0.58 — `Windows.Media.Control` namespace |
| Weather | wttr.in REST (no API key) |
| Glass effect | CSS gradients + SVG `feTurbulence` + `feDisplacementMap` |
| Cursor / DWM | Raw Win32 FFI (`GetCursorPos`, `DwmSetWindowAttribute`, `SystemParametersInfoW`, `CreateMutexW`) |

## Interesting problems solved

A handful of things that took more thought than expected:

### 1. Cursor passthrough only at the screen edge
The island sits at the top of the screen but must let clicks pass through when idle. Solution: `set_ignore_cursor_events(true)` permanently, plus a 60ms `GetCursorPos` poll in JS that re-enables events the instant the cursor enters the top 8 px of the screen.

### 2. `windows` crate version conflicts
Tauri internally depends on `windows` 0.61. Our crate uses 0.58 (for `Windows.Media.Control`). Using `HWND` from one in an API call from the other fails at compile time. Fix: raw `extern "system"` FFI for DWM / cursor / mutex calls. HWND is `repr(transparent)` over `isize`, so `std::mem::transmute_copy` extracts the value safely across crate versions.

### 3. Windows Media Session blocks Tokio
`IAsyncOperation::Status()` in the `windows` crate is not `Send`, so awaiting it in a `tokio::spawn` fails. Wrapped the WinRT calls in `tokio::task::spawn_blocking` and switched to synchronous status polling (`Status()` + `GetResults()`).

### 4. Pill-shaped window with antialiased corners
`SetWindowRgn` gives 1-bit alpha (jagged edges). DWM Mica fills the rectangular HWND, ignoring CSS `border-radius` — corners leak in transparent windows. Solution: keep the WebView2 window transparent, animate `border-radius` on the React root via Framer Motion. WebView2's compositor handles antialiasing per-frame.

### 5. `backdrop-filter` artifact in WebView2
Setting `backdrop-filter: blur()` on a transparent WebView2 window paints a hard rectangle that ignores `border-radius`. We use SVG `feTurbulence` + `feDisplacementMap` instead — the "liquid" displacement is purely visual on top of the semi-transparent pill, never touching the OS compositor.

### 6. Resize that stays anchored to the taskbar
The OS resize API preserves the top-left corner — change the size, the window grows downward. For a "snap to bottom" mode that lives above the taskbar, we read `SystemParametersInfoW(SPI_GETWORKAREA)` and reposition so the **bottom** edge stays glued. *(Bottom mode removed in 0.1.0 per UX simplification, but the code is still there.)*

### 7. Single-instance lock without leaking the handle
Named mutex (`CreateMutexW`) with `Local\\WindowsIsland_SingleInstance_v1`. If `GetLastError() == ERROR_ALREADY_EXISTS`, exit silently. The handle is intentionally leaked — it lives for the process lifetime; the OS reclaims it on exit.

## Roadmap

This is v0.1.0. If there's enough interest, future ideas:
- More widgets (timer, pomodoro, calendar peek, app launcher)
- Direct Spotify / Discord / Slack integrations
- Custom theme editor
- Multi-monitor positioning memory
- Linux / macOS ports (Tauri-based but Win32 FFI needs platform-specific replacements)

## License

[MIT](LICENSE) — do whatever you want, attribution appreciated.

## Acknowledgments

- **Apple** for the original Dynamic Island design language on iPhone 14 Pro
- **Tauri team** for making sub-4MB native apps possible
- **wttr.in** for keyless weather

---

Made with 🦀 + ⚛ by [@SergioLugoPM](https://github.com/SergioLugoPM)
