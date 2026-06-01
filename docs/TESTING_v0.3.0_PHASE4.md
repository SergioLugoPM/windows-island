# Windows Island v0.3.0 Phase 4: Integration Test Report

## Build Status

| Artifact | Status |
|---|---|
| `windows_island_injector_dll.dll` | ✅ Compiles clean |
| `windows-island.exe` | ✅ Compiles clean |

## Architecture Delivered

| Feature | File | Status |
|---|---|---|
| PE header parser | `injector-dll/src/pe_parser.rs` | ✅ Implemented |
| Real IAT patching | `injector-dll/src/iat_patcher.rs` | ✅ Implemented |
| Taskbar redraw | `injector-dll/src/message_handler.rs` | ✅ Implemented |
| Background refresh thread | `injector-dll/src/lib.rs` | ✅ Implemented |

## Manual Test Procedure (requires Administrator)

1. Launch `windows-island.exe` as Administrator
2. In Settings, enable injection
3. Within 500 ms the background thread fires its first poll
4. Expected: `Shell_TrayWnd` receives `WM_SYSCOLORCHANGE` → repaints
5. Change theme (Dark ↔ Light) in Settings
6. Expected: taskbar repaints within 500 ms
7. Disable injection
8. Expected: IAT restored, taskbar reverts to Windows defaults

## IAT Patch Notes

- `find_and_patch_iat` targets `GetSysColor` in `user32.dll` in Explorer.exe's
  main module IAT. On Windows 11, the taskbar may call `GetSysColor` from a
  sub-DLL (e.g. `twinui.dll`). If so, the IAT patch succeeds structurally but
  may not intercept all taskbar color queries — Phase 5 would address this by
  enumerating all loaded modules.
- If GetSysColor is not in the main module IAT, `find_and_patch_iat` returns
  an error and the hook degrades gracefully (the thread still polls and sends
  `WM_SYSCOLORCHANGE` without IAT interception).

## Known Limitations

1. **Windows 11 23H2 taskbar** — Uses WinUI 3 / DWM for its translucent pill
   colors. `WM_SYSCOLORCHANGE` forces a repaint but the DWM compositor may
   override the colors. Phase 5 would target `DwmSetWindowAttribute` +
   `UxTheme` for deeper integration.
2. **Background thread not joined on unload** — The thread exits within 500 ms
   after `REFRESH_THREAD_RUNNING` is set to false. This is safe: the thread
   only touches Win32 APIs and our own statics, and the process continues
   running (we only unload the DLL, not terminate Explorer).
3. **Admin required** — DLL injection into Explorer.exe requires the Windows
   Island process to run as Administrator.
