# Windows Island

Apple-style Dynamic Island for Windows — an always-on-top, edge-snapping pill widget that shows the clock, weather, and the currently playing media. Built with **Tauri 2 + React + TypeScript + Rust**.

![Windows Island](https://img.shields.io/badge/platform-Windows%2010%2B-blue) ![Tauri](https://img.shields.io/badge/Tauri-2.x-yellow) ![License: MIT](https://img.shields.io/badge/License-MIT-green)

## Features

- **Always-on-top pill** that snaps to the top edge of the screen
- **Cursor passthrough in idle** — clicks through to apps below until the cursor touches the screen edge
- **Three view modes** (cycle by click): peek (clock + weather/media), media (vinyl + visualizer), full (everything)
- **Three themes**: Dark, Light, **Vidrio** (liquid-glass with iridescent border, chromatic aberration, oval specular, caustic light)
- **Clock sizes** (S / M / L) — resizes the idle pill
- **Configurable hover content**: weather or media
- **Floating mode** — drag anywhere on screen
- **Long-press** the island to open settings
- **System tray** with right-click menu (show/hide, quit)
- **Single-instance lock** — only one runs at a time
- **Windows Media Session integration** — play/pause, skip via Windows global media controls
- **wttr.in weather** (no API key required)

## Install

Download the latest installer from [Releases](../../releases):

- `Windows.Island_x.y.z_x64_en-US.msi` — MSI installer
- `Windows.Island_x.y.z_x64-setup.exe` — NSIS installer

## Build from source

```bash
git clone https://github.com/SergioLugoPM/windows-island
cd windows-island
npm install
cargo tauri dev      # dev mode
cargo tauri build    # release installer
```

Installers land in `src-tauri/target/release/bundle/`.

## Tech stack

| Layer | Tool |
|---|---|
| Window / OS integration | Tauri 2 (Rust) |
| Frontend | React 18 + TypeScript + Vite |
| Animation | Framer Motion |
| Media | Windows.Media.Control (WinRT) |
| Weather | wttr.in |
| Glass effect | CSS + SVG feTurbulence/feDisplacementMap |

## Controls

| Action | Effect |
|---|---|
| Hover screen edge (idle) | Expand to peek |
| Click | Cycle through modes |
| Long-press | Open settings |
| Drag | Move (floating mode) or re-snap to nearest edge |
| Escape | Collapse to idle |
| Tray left-click | Toggle visibility |
| Tray right-click | Context menu |

## License

MIT
