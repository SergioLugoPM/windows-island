# Changelog

All notable changes to Windows Island are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Real media progress bar — reads `Position()`, `StartTime()`, `EndTime()` from
  `GlobalSystemMediaTransportControlsSession.TimelineProperties`. Client-side
  interpolation at 4 Hz between 3 s polls so the bar moves smoothly.
- Live `M:SS / M:SS` time display next to the progress bar.

### Fixed
- **Cycle no longer auto-collapses while the cursor is over the island.**
  Previously `handleClick` scheduled a 5 s collapse that fired even with hover;
  now only `handleMouseLeave` schedules a collapse (2.8 s after the cursor
  leaves).
- **Audio visualizer is now always animated when media is playing.**
  Removed the `getUserMedia` path: it prompted the OS for microphone permission
  (bad UX), and two hook instances on the same page (idle + media views) raced
  for the same stream so one failed silently. Visualizer is now a deterministic
  3-harmonic sine bank with golden-ratio phase spread.

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
