# Windows Island v0.3.0 Phase 4: IAT Patching & Hook Implementation

## Overview

Phase 4 completes the rendering hooks architecture by implementing the two most complex pieces deferred from Phase 3:

1. **Actual IAT Patching** — Modify Explorer.exe's Import Address Table to redirect GetSysColor calls
2. **Message Hook Full Implementation** — Detect window creation and force taskbar redraws with new colors

Upon completion, changing the theme in Windows Island's Settings will immediately update Explorer.exe and Start Menu colors without restart.

## Phase 3 → Phase 4 Handoff

**What Phase 3 delivered:**
- ✅ IAT patcher module that stores original GetSysColor pointers (no patching yet)
- ✅ Message handler stubs ready for wiring
- ✅ Theme config caching infrastructure (lazy_static, Mutex, fallback to DARK_THEME_COLORS)
- ✅ IPC initialization on DLL load
- ✅ Frontend command wiring (UI → `refresh_injected_theme_config` → IPC → DLL)
- ✅ Builds clean, commits stable, architecture validated

**What Phase 4 must deliver:**
- 🔨 Actual IAT patching (PE header parsing, import table modification)
- 🔨 Message hook procedure implementation (window enumeration, redraw forcing)
- 🔨 Background theme refresh thread (polling, cache invalidation)
- 🔨 Integration tests with admin injection
- 🔨 Performance profiling and optimization

## Task Breakdown

### Task 1: PE Header Parser for IAT Modification

**Files to create:**
- `injector-dll/src/pe_parser.rs` — PE header utilities for finding GetSysColor import

**Goal:** Parse the PE header of the injected process to locate GetSysColor in the import table, then patch the entry.

**Technical approach:**
- Find the DOS header signature (MZ)
- Read PE offset from DOS header
- Iterate import directory entries
- Match module name ("user32.dll") and function name ("GetSysColor")
- Patch the IAT entry with pointer to hooked_get_sys_color
- Store original pointer for restoration

**Deliverable:** `patch_iat_real()` function that modifies the IAT, with corresponding unpatch.

---

### Task 2: CBT Hook Procedure Implementation

**Files to modify:**
- `injector-dll/src/message_handler.rs` — Implement cbt_hook_proc body

**Goal:** Detect window creation events and force taskbar redraws.

**Technical approach:**
- In cbt_hook_proc, check if code == HC_ACTION and wparam is a window handle
- Use FindWindowA("Shell_TrayWnd") to locate the taskbar window
- Call InvalidateRect to mark it for redraw
- Call UpdateWindow to force immediate redraw
- Call PostMessageA with WM_SETTINGCHANGE to notify of color updates

**Deliverable:** Full cbt_hook_proc implementation that redraws taskbar on window creation or system color change.

---

### Task 3: Background Theme Refresh Thread

**Files to modify:**
- `injector-dll/src/lib.rs` — Spawn worker thread on DLL load

**Goal:** Periodically check if theme config has changed and force UI updates.

**Technical approach:**
- In DLL_PROCESS_ATTACH, after hook install, spawn a worker thread
- Thread polls IPC every 500ms for theme config changes (hash the config)
- On change, call redraw_taskbar_windows() and post WM_SETTINGCHANGE
- Thread exits cleanly on DLL_PROCESS_DETACH

**Deliverable:** Background theme refresh loop with graceful thread lifecycle.

---

### Task 4: Integration Testing with Admin Injection

**Files to create:**
- `docs/TESTING_v0.3.0_PHASE4.md` — Integration test procedures and results

**Goal:** Verify that colors actually change in the taskbar when the theme changes in Windows Island.

**Test procedure:**
1. Build Windows Island release
2. Run as Administrator
3. Start Windows Island app
4. Open Settings → Rendering (or equivalent)
5. Toggle injection ON
6. Observe taskbar colors change to dark theme
7. Change theme to "Light" in Settings
8. Observe taskbar colors change to light
9. Toggle injection OFF
10. Observe taskbar colors revert to Windows default
11. Verify no crashes at any step

**Deliverable:** Test report with pass/fail status and any issues found.

---

### Task 5: Performance Profiling & Optimization

**Files to review/modify:**
- `injector-dll/src/hook_procedures.rs` — Optimize color lookup
- `injector-dll/src/iat_patcher.rs` — Optimize IAT scan

**Goal:** Ensure hook latency is < 1ms and does not cause Explorer.exe to hang.

**Optimization targets:**
- Pre-compute color lookup tables (avoid runtime match on every GetSysColor call)
- Cache Windows API function pointers (GetModuleHandleA, GetProcAddress)
- Minimize allocations in hot paths (hook procedures)
- Profile with Windows Performance Toolkit to measure latency

**Deliverable:** Optimized hook procedures with latency measurements < 1ms per GetSysColor call.

---

## Timeline

**Estimated effort:** 3-4 development sessions

- Session 1: PE header parser + IAT patching (Tasks 1-2)
- Session 2: Background thread + integration tests (Tasks 3-4)
- Session 3: Performance optimization + final review (Task 5)

## Success Criteria

- ✅ Explorer.exe taskbar colors change immediately when theme changes in Windows Island
- ✅ No freezing or crashes during injection/uninjection
- ✅ Hook latency < 1ms
- ✅ Graceful fallback if IPC fails
- ✅ Full integration test pass

## Notes

- Phase 4 is the final phase before v0.3.0 release
- After Phase 4: Update version to 0.3.0, publish release build, tag git
- Future versions (0.4.0+) can add more rendering targets (Start Menu, Taskbar buttons, Windows chrome)
