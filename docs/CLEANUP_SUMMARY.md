# Cleanup Summary — Phase 1 Legacy Dead Code

## Overview

This document records the removal of Phase 1 legacy code from
`injector-dll/src/lib.rs` in preparation for Phase 3 finalization.
The cleanup was performed across 3 surgical commits after both build
targets confirmed green.

---

## Sections Removed

### 1. `InjectedTheme` struct

- **Location:** `injector-dll/src/lib.rs`
- **Lines removed:** ~13
- **Reason:** Phase 1 shared-memory layout using the old mapping name
  `Local\WindowsIsland_Theme_v1`. Replaced by `ThemeConfig` in
  `injector-dll/src/ipc_client.rs`, which targets the Phase 2 mapping
  `Local\WindowsIsland_Theme_IPC_v1`.

### 2. `on_dll_attach()` function

- **Location:** `injector-dll/src/lib.rs`
- **Lines removed:** ~17
- **Reason:** Opened the old `Local\WindowsIsland_Theme_v1` mapping and
  read `InjectedTheme` from it. This path is superseded by
  `IpcClient::connect()` which is called directly from DllMain
  (via `get_ipc_client()`).

### 3. `on_dll_detach()` function

- **Location:** `injector-dll/src/lib.rs`
- **Lines removed:** ~3
- **Reason:** Was an empty stub with a Phase 2 TODO comment. Hook
  teardown is handled by `hook_procedures::uninstall_hooks()` which
  is called directly in the `DLL_PROCESS_DETACH` branch.

### 4. DllMain call sites

- **Lines removed:** ~6
- **Reason:** The `unsafe { on_dll_attach(); }` and
  `unsafe { on_dll_detach(); }` blocks inside DllMain's match arms
  were removed. DllMain now returns `TRUE` directly after
  hook install / uninstall.

### 5. Unused imports

- **Removed from `injector-dll/src/lib.rs`:**
  - `CloseHandle` (windows::Win32::Foundation)
  - `OpenFileMappingA`, `MapViewOfFile`, `UnmapViewOfFile`, `FILE_MAP_READ`
    (windows::Win32::System::Memory)
  - `PCSTR` (windows::core)
- **Retained:** `HINSTANCE`, `BOOL`, `TRUE`, `FALSE` — still required
  by DllMain's signature and return values.

---

## Total Lines Deleted

Approximately **~53 lines** across the 3 commits (struct + functions +
call sites + imports).

---

## ThemeManager Status (src-tauri/src/injector/theme.rs)

`ThemeManager` is **actively used** and was NOT removed:

- `AppState.theme_manager: Arc<Mutex<ThemeManager>>` holds it
- `enable_theme_injection` Tauri command writes a theme to it via
  `write_theme()`
- `InjectedTheme` factory methods (`dark_theme`, `light_theme`,
  `vidrio_theme`) drive that command

The `Local\WindowsIsland_Theme_v1` mapping it creates is the original
Phase 1 host-side path. This can be audited separately in Phase 3 once
the decision is made whether to keep or deprecate the legacy injection
path in favour of the new IPC server (`Local\WindowsIsland_Theme_IPC_v1`).

---

## Build Status

| Target | Command | Result |
|--------|---------|--------|
| Injector DLL | `cargo build --lib` (in `injector-dll/`) | PASS |
| Tauri app | `cargo build` (in `src-tauri/`) | PASS |

Pre-existing warnings (cpu_temp.rs unused ptr, cfg-gated single-instance
functions) remain but were not introduced by this cleanup.

---

## Commit History

| # | SHA | Message |
|---|-----|---------|
| 1 | 7839760 | refactor(dll): remove InjectedTheme struct (Phase 1 legacy) |
| 2 | 386a0ad | refactor(dll): remove on_dll_attach and on_dll_detach (Phase 1 legacy) |
| 3 | 6cc1afc | refactor(dll): remove unused imports left by Phase 1 dead code |

---

## Ready for Phase 3

`injector-dll/src/lib.rs` is now clean:
- DllMain initializes `ThemeHandler`, connects `IpcClient`, installs hooks
- No references to the old `Local\WindowsIsland_Theme_v1` mapping
- No dead structs or stub functions
- `get_theme_handler()` and `get_ipc_client()` are both intact and operational
