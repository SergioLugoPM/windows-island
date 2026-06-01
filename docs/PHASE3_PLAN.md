# Windows Island v0.3.0 Phase 3: Rendering Hooks Implementation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the hook stubs from Phase 2 so they actually intercept WndProc messages and apply theme colors to Explorer.exe taskbar in real-time.

**Architecture:** 
- Replace `install_hooks()` stub with actual IAT patching of `GetSysColor` in Explorer.exe's import table
- Implement message interception via `SetWindowsHookEx` to detect window creation and force color updates
- Connect `IpcClient` data (theme config from main app) into the hook procedures
- Store original function pointers for safe restoration in `uninstall_hooks()`
- Add message pump for handling WM_SETTINGCHANGE and WM_SYSCOLORCHANGE notifications

**Tech Stack:**
- Rust `windows` crate for Win32 FFI
- `SetWindowsHookEx` with `WH_CBT` hook type for window creation interception
- IAT patching via manual PE parsing or `detours`-style approach
- Atomic flags for thread-safe hook state

---

**Status:** 
- Task 1: ✅ DONE (IAT Patcher)
- Tasks 2-7: Queued

**Next steps when resuming:**
1. Continue with Task 2 (Message Handler)
2. Execute Tasks 3-7 sequentially
3. Final integration testing

**Files modified this session:**
- `injector-dll/src/iat_patcher.rs` (NEW)
- `injector-dll/src/hook_procedures.rs` (UPDATED)
- `injector-dll/src/lib.rs` (UPDATED)

**Build status:** ✅ Passing (0 new warnings)

---

See full plan in: `docs/superpowers/plans/2026-05-31-windows-island-v0.3.0-phase3.md`
