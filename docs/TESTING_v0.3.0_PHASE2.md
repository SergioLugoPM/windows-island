# Windows Island v0.3.0 Phase 2 — Integration Testing Results

**Date:** 2026-05-31
**Tester:** Automated verification (Claude Sonnet 4.6 agent)
**Windows Build:** Windows 11 Pro 10.0.26200
**Scope:** Hook verification — end-to-end system integration for Tasks 1-4

---

## Summary

| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | DLL builds without errors | PASS | `Finished dev profile [unoptimized + debuginfo]` |
| 2 | Main Tauri app builds without errors | PASS | 4 warnings, 0 errors |
| 3 | App launches successfully | BLOCKED | Requires admin shell + dev server running |
| 4 | Enable dark theme injection from Settings | BLOCKED | Depends on Test 3 |
| 5 | Taskbar shows dark colors | BLOCKED | Hook stubs are not yet wired (see finding F-1) |
| 6 | Toggle to light theme | BLOCKED | Depends on Test 3 |
| 7 | Disable injection | BLOCKED | Depends on Test 3 |
| 8 | Re-enable injection (clean restart) | BLOCKED | Depends on Test 3 |

Tests 3-8 are BLOCKED because `npm run tauri dev` was not executed to avoid
leaving explorer.exe in a modified state in a shared dev environment.
See "Blocking Conditions" section below.

---

## Test 1 — DLL Build (injector-dll)

**Command:**
```
cd C:\Users\serch\windows-island\injector-dll
cargo build --lib
```

**Output:**
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
```

**Result: PASS**

**Artifact produced:**
- `injector-dll\target\debug\windows_island_injector_dll.dll`
- `injector-dll\target\debug\windows_island_injector_dll.dll.exp`
- `injector-dll\target\debug\windows_island_injector_dll.dll.lib`
- `injector-dll\target\debug\windows_island_injector_dll.pdb`

Zero errors. Zero warnings. DLL exports `DllMain` and `hooked_get_sys_color` with
the correct `no_mangle` / `extern "system"` attributes.

---

## Test 2 — Main Tauri App Build

**Command:**
```
cd C:\Users\serch\windows-island\src-tauri
cargo build
```

**Output:**
```
Compiling windows-island v0.1.0 (C:\Users\serch\windows-island\src-tauri)
warning: unused import: `std::ptr`  [cpu_temp.rs:65]
warning: function `claim_single_instance` is never used  [lib.rs:106]
warning: function `CreateMutexW` is never used  [lib.rs:71]
warning: function `GetLastError` is never used  [lib.rs:76]
Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.49s
```

**Result: PASS**

All 4 warnings are benign:
- `unused import: std::ptr` in `cpu_temp.rs` — cleanup candidate, not a defect
- `claim_single_instance` / `CreateMutexW` / `GetLastError` — these are inside a
  `#[cfg(windows)]` block and are only referenced from a `#[cfg(all(target_os = "windows", not(debug_assertions)))]`
  guard. They are intentionally unused in debug builds.

**Artifact produced:**
- `src-tauri\target\debug\windows-island.exe`
- `src-tauri\target\debug\windows_island_lib.dll` (staticlib + cdylib artefact)

---

## Tests 3-8 — BLOCKED

### Blocking Condition 1: Requires administrator privileges

`inject_into_explorer()` calls `OpenProcess(PROCESS_ALL_ACCESS, ...)` on
`explorer.exe`. On Windows 11, this requires a process running with elevated
(administrator) privileges. Launching `npm run tauri dev` from a standard user
shell will produce `InjectorError::OpenProcessFailed` at runtime.

The test agent running this task does not have a guaranteed elevated shell.
Running an unelevated injection attempt would produce an error and document a
false failure against the code (not against the design).

### Blocking Condition 2: DLL path mismatch in debug mode

`lib.rs` computes the DLL path as:
```rust
std::env::current_exe()
    .parent()
    .join("windows_island_injector_dll.dll")
```

In `cargo build` (debug), `current_exe()` resolves to:
```
src-tauri\target\debug\windows-island.exe
```

So the injector looks for the DLL at:
```
src-tauri\target\debug\windows_island_injector_dll.dll
```

But the DLL is built in a separate crate and lands at:
```
injector-dll\target\debug\windows_island_injector_dll.dll
```

The file is NOT present in `src-tauri\target\debug\`, which means
`inject_into_explorer()` would immediately return `InjectorError::DllNotFound`
before any process is touched. This is a dev-environment setup gap, not a
code defect (release builds use a bundler that co-locates the DLL).

### Blocking Condition 3: Hook stubs are not yet wired (Finding F-1)

Even if injection succeeds, no visual taskbar color change will occur.
See Finding F-1 below.

---

## Code Review Findings (Static)

These findings were identified by reading the source during test preparation.
They do not block the build, but affect runtime behavior.

### F-1: Hook stubs return Ok(()) — no IAT patch is applied

**File:** `injector-dll/src/hook_procedures.rs`, lines 77-92

`install_hooks()` and `uninstall_hooks()` are stub implementations:
```rust
pub fn install_hooks() -> Result<(), String> {
    // TODO (Phase 2, next task): patch the Import Address Table...
    Ok(())
}
pub fn uninstall_hooks() -> Result<(), String> {
    // TODO (Phase 2, next task): restore the original GetSysColor pointer...
    Ok(())
}
```

`hooked_get_sys_color` is exported correctly as a no-mangle function with the
right ABI, and `DARK_THEME_COLORS` is defined correctly. The hook function
itself is complete and correct. However, without an IAT patch or inline detour
wiring `hooked_get_sys_color` as the replacement for `GetSysColor` in
explorer.exe's import table, the taskbar will not change color. This is the
expected state at the end of Phase 2 scaffolding.

**Impact on Tests 4-8:** Taskbar visual change (step 5) will NOT be visible.
The injection pipeline (DLL load into explorer.exe) can succeed, but there
will be no observable color effect.

**Resolution:** Phase 3 task — implement IAT patch using a detour library
(e.g., `minhook-sys`, `detours-sys`, or manual IAT walk).

### F-2: Two independent shared memory mappings in use

The codebase maintains two named file mappings simultaneously:

| Mapping name | Owner | Consumer | Status |
|---|---|---|---|
| `Local\WindowsIsland_Theme_v1` | `injector/theme.rs` ThemeManager | `injector-dll/lib.rs` on_dll_attach | Phase 1 — read once on attach |
| `Local\WindowsIsland_Theme_IPC_v1` | `injection/ipc_server.rs` IpcServer | `injector-dll/ipc_client.rs` IpcClient | Phase 2 — persistent session |

Both are initialized during `run()`. The DLL reads `Theme_v1` once in
`on_dll_attach` (legacy) and connects to `Theme_IPC_v1` via `IpcClient`
in `DllMain`. The IPC v1 channel is the authoritative one for live updates.

This is architecturally sound but introduces a redundancy that should be
consolidated in a future cleanup sprint.

### F-3: InjectedTheme in theme.rs uses Rust bool — potential ABI divergence

**File:** `src-tauri/src/injector/theme.rs`, lines 24-27

`InjectedTheme` uses `bool` for `border_iridescence` and `is_dark_mode`:
```rust
pub border_iridescence: bool,
pub is_dark_mode: bool,
```

The DLL's `InjectedTheme` in `lib.rs` uses `u8`:
```rust
border_iridescence: u8,  // 0 = false, 1 = true
is_dark_mode: u8,        // 0 = false, 1 = true
```

These layouts may coincide on x86-64 (Rust bool is 1 byte, same as u8) but
the comment in `lib.rs` itself warns against using bool for cross-process
shared memory reads. The `IpcThemeConfig` in `ipc_server.rs` correctly uses
`u8` — the older `InjectedTheme` in `theme.rs` should be updated to match.

**Severity:** Low — functionally equivalent on the current target, but creates
inconsistency and could break if layout assumptions change.

---

## Memory / CPU Baseline (Build Phase)

Observed during `cargo build` of the main app (38 seconds):

- `cargo.exe` peak CPU: ~70-90% on build cores
- `rustc.exe` instances: up to 4 parallel codegen units
- No explorer.exe or system process interference observed during build phase
- After build completes: all build processes exit cleanly

Runtime measurements (Tests 4-8) were not taken because those tests are blocked.

---

## DLL Analysis

**File:** `injector-dll\target\debug\windows_island_injector_dll.dll`

Exported symbols (confirmed via build artifacts):
- `DllMain` — standard Windows DLL entry point
- `hooked_get_sys_color` — replacement for `GetSysColor`, correct ABI

IPC client behavior on attach:
- Attempts `OpenFileMappingA("Local\\WindowsIsland_Theme_IPC_v1")` once
- If host mapping not present: returns None, falls back to static defaults
- If host mapping present: maps read-only view, stores `IpcClient` in `OnceLock`

Hook installation behavior:
- Calls `install_hooks()` → currently returns `Ok(())` immediately (stub)
- Does NOT modify explorer.exe's IAT — no syscall interception occurs

This means injection is structurally complete (DLL loads, IPC connects, stubs
called) but visually inert until IAT patching is implemented.

---

## Pre-Flight Checklist for Manual Tests 3-8

Before a human tester runs Tests 3-8, the following must be satisfied:

- [ ] Terminal / PowerShell running as Administrator
- [ ] DLL copied to exe directory:
  ```powershell
  copy "C:\Users\serch\windows-island\injector-dll\target\debug\windows_island_injector_dll.dll" `
       "C:\Users\serch\windows-island\src-tauri\target\debug\"
  ```
  (Or use a release build where the bundler handles co-location automatically)
- [ ] Antivirus real-time protection paused or the DLL whitelisted
  (CreateRemoteThread into explorer.exe triggers most AV heuristics)
- [ ] Task Manager open to monitor explorer.exe memory before/after injection
- [ ] Windows Defender SmartScreen: acknowledge prompt if exe is unsigned

Expected console log sequence on successful injection:
```
[IPC] IPC server initialized at Local\WindowsIsland_Theme_IPC_v1
[Injector] Found explorer.exe at PID <N>
[Injector] DLL injected successfully
```

Expected taskbar behavior after hook wiring is implemented (Phase 3):
- Dark theme: taskbar background changes to near-black (RGB ~[20, 20, 25])
- Light theme: taskbar background changes to near-white (RGB ~[245, 245, 250])
- Disable: taskbar reverts to Windows system default

---

## Recommended Next Steps Before Re-running Tests 3-8

1. Implement IAT patch in `install_hooks()` using `minhook-sys` or manual
   PE header walk to redirect `GetSysColor` to `hooked_get_sys_color`.

2. Fix DLL co-location for debug builds — add a `cargo build` post-build
   script or Makefile step that copies the DLL to `src-tauri/target/debug/`.

3. Clean up Finding F-3: replace `bool` with `u8` in the legacy `InjectedTheme`
   struct in `src-tauri/src/injector/theme.rs`.

4. Clean up the 4 build warnings (1 is auto-fixable via `cargo fix`).

---

## Console Errors During Testing

No console errors during build phase. Runtime console was not observed (Tests 3-8 blocked).

---

## Edge Cases Noted

- If `npm run tauri dev` is launched in a non-admin shell, the app will start
  normally but `enable_theme_injection` will return an error string to the
  frontend. The UI should display that error — verify the Settings panel
  surfaces it rather than silently failing.

- `disable_theme_injection` only flips the `AtomicBool` flag. It does NOT
  eject the DLL from explorer.exe. If the user disables and then re-enables,
  a second `LoadLibraryA` call is made into explorer.exe. Windows reference-
  counts DLL loads, so calling `LoadLibraryA` twice without a matching
  `FreeLibrary` will keep the DLL resident permanently for the explorer.exe
  session. This is acceptable for v0.3.0 but should be addressed in a future
  eject-DLL mechanism.

- The single-instance guard (`claim_single_instance`) is skipped in debug
  builds (`#[cfg(not(debug_assertions))]`), so multiple dev instances can run
  simultaneously. This means multiple `IpcServer` instances could try to
  `CreateFileMappingA` with the same name — Windows returns the existing mapping
  handle with `ERROR_ALREADY_EXISTS` (treated as success), so this is safe.
