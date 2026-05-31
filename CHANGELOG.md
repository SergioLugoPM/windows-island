# Changelog

All notable changes to Windows Island are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-31

### Added
- **Network metrics** — real-time download/upload speed in System Stats (via sysinfo
  networks feature). Updated every 1.5s with running average.
- **i18n support** — English and Spanish localizations. Strings cover Settings, Weather,
  Stats labels (CPU/RAM/Battery), and system tray menu.
- **Auto-updater** — integrated tauri-plugin-updater. App checks for updates on startup;
  user prompted to download v0.2.1+. Requires valid update endpoint and public key.
- **StatsView network display** — shows net↓ and net↑ in MiB/s alongside CPU/RAM/Battery.

### Technical
- Enabled sysinfo `networks` feature (was disabled to reduce binary size; network overhead
  negligible in practice).
- i18n backend in Rust (src-tauri/src/i18n.rs) + React hook (src/hooks/useI18n.ts).
- Updater check fires async after Tauri setup in release builds (disabled in debug to avoid
  prompts during development).

## [0.1.0] - 2026-05-30

Initial release.

### Added
- Three view modes: peek, media, full (cycle by click)
- Three themes: Dark, Light, Vidrio (liquid glass with iridescent border,
  chromatic aberration, oval specular, animated caustics via SVG turbulence)
- Three clock sizes (S / M / L)
- Position modes: top-edge snap (cursor passthrough until edge) and floating
- Long-press → settings panel
- System tray with right-click menu (Show / Hide / Quit)
- Single-instance lock via named mutex
- Windows Media Session integration (play / pause / skip)
- Weather via wttr.in (no API key)
- MSI + NSIS installers, ~3 MB binary
