# Windows Island Cleanup: Remove Legacy Code (Phase 1-2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Remove dead/duplicated code from Phase 1-2 to reduce maintenance burden before finishing Phase 3.

**Context:**
- Phase 2 reviews identified duplicated IPC mapping logic
- Phase 1 has `on_dll_attach()` that opens old mapping name (Theme_v1, not Theme_IPC_v1)
- Phase 2 created the correct new IPC path (Theme_IPC_v1)
- Legacy code still runs but is superseded; should be removed

**Tech Stack:**
- Rust code cleanup
- Git commits

---

## File Structure

**Files to modify:**
- `injector-dll/src/lib.rs` — Remove legacy `on_dll_attach()` and `InjectedTheme` struct
- `src-tauri/src/injector/theme.rs` — Document that Legacy Theme Manager is now unused

---

## Task 1: Identify All Legacy Code

**Status:** Pending

**Files:**
- Read: `injector-dll/src/lib.rs`
- Read: `src-tauri/src/injector/theme.rs`

**Steps:**

- [ ] **Step 1: Read current lib.rs to identify legacy sections**

Read `injector-dll/src/lib.rs` and document:
- Lines of `InjectedTheme` struct definition
- Lines of `on_dll_attach()` function
- Mapping name used (should be "Local\\WindowsIsland_Theme_v1")

- [ ] **Step 2: Read theme.rs to understand ThemeManager**

Read `src-tauri/src/injector/theme.rs`:
- Is `ThemeManager` still referenced anywhere?
- Is the mapping name "Local\\WindowsIsland_Theme_v1" still needed?
- Are there any comments about DLL injection?

- [ ] **Step 3: Document findings**

Create `docs/CLEANUP_LEGACY_ANALYSIS.md` with:
- Lines/sections to remove
- Why they're superseded
- Any references that need updating

---

## Task 2: Remove InjectedTheme Struct from DLL

**Status:** Pending

**Files:**
- Modify: `injector-dll/src/lib.rs`

**Steps:**

- [ ] **Step 1: Locate InjectedTheme struct in lib.rs**

Find the struct definition (should be around lines 42-54 based on code review notes).

- [ ] **Step 2: Delete the InjectedTheme struct**

Remove the entire struct definition:
```rust
// DELETE THIS:
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InjectedTheme {
    // ... fields ...
}
```

- [ ] **Step 3: Compile to find any dangling references**

Run: `cd src-tauri && cargo build --lib -p windows-island-injector-dll`

Expected: If InjectedTheme had other references, compiler will report them. If no errors, we're good.

- [ ] **Step 4: Commit**

```bash
git add injector-dll/src/lib.rs
git commit -m "refactor: remove legacy InjectedTheme struct from DLL"
```

---

## Task 3: Remove on_dll_attach() Function

**Status:** Pending

**Files:**
- Modify: `injector-dll/src/lib.rs`

**Steps:**

- [ ] **Step 1: Locate on_dll_attach() function**

Find the function definition (should be around lines 97-113 based on code review notes).

- [ ] **Step 2: Delete the function entirely**

Remove:
```rust
// DELETE THIS:
fn on_dll_attach() -> i32 {
    // ... implementation using old mapping name ...
}
```

Also remove any other helper functions it calls that are now unused.

- [ ] **Step 3: Update DllMain to remove call**

Find `DllMain` and remove the call to `on_dll_attach()`. Current line should be something like:
```rust
DLL_PROCESS_ATTACH => {
    // ... other code ...
    on_dll_attach()  // <- REMOVE THIS LINE
}
```

Replace with just:
```rust
DLL_PROCESS_ATTACH => {
    // ... other code ...
    1  // Return success (unchanged)
}
```

- [ ] **Step 4: Also remove on_dll_detach() if it exists**

Search for `on_dll_detach` and remove if present (likely unused).

- [ ] **Step 5: Compile and verify**

Run: `cd src-tauri && cargo build --lib -p windows-island-injector-dll`

Expected: `Finished` with no errors, no new warnings

- [ ] **Step 6: Commit**

```bash
git add injector-dll/src/lib.rs
git commit -m "refactor: remove legacy on_dll_attach() function and Phase 1 IPC logic"
```

---

## Task 4: Clean Up Unused Imports in DLL

**Status:** Pending

**Files:**
- Modify: `injector-dll/src/lib.rs`

**Steps:**

- [ ] **Step 1: Identify unused imports**

After removing `on_dll_attach()`, some imports may no longer be needed. Run:

```bash
cd src-tauri && cargo build --lib -p windows-island-injector-dll 2>&1 | grep "unused"
```

Expected: May see warnings like "unused import: CreateFileMappingA" if they were only used in on_dll_attach.

- [ ] **Step 2: Remove unused imports**

Delete any imports that are no longer referenced:
- `CreateFileMappingA`
- `MapViewOfFile`
- `UnmapViewOfFile`
- `FILE_MAP_READ`
- `PAGE_READONLY`
- `PCSTR`
- Any others flagged as unused

Keep imports needed by:
- `ThemeManager` struct (if still present in this file)
- `get_theme_handler()` function
- `get_ipc_client()` function

- [ ] **Step 3: Compile to verify**

Run: `cd src-tauri && cargo build --lib -p windows-island-injector-dll`

Expected: `Finished` with no warnings about unused imports

- [ ] **Step 4: Commit**

```bash
git add injector-dll/src/lib.rs
git commit -m "refactor: remove unused imports from DLL lib.rs"
```

---

## Task 5: Verify Legacy ThemeManager is Still Working for Main App

**Status:** Pending

**Files:**
- Read: `src-tauri/src/injector/theme.rs`
- Modify: `src-tauri/src/injector/theme.rs` (add comment if needed)

**Steps:**

- [ ] **Step 1: Confirm ThemeManager is used in main app**

Check where `ThemeManager` is used:
- Is it used in any Tauri commands?
- Is it used in the main app's injection flow?

If yes, it's still needed for main app. If no, can be removed too (Task 6).

- [ ] **Step 2: Add deprecation comment if needed**

If `ThemeManager` is still there but only for legacy purposes, add a comment:
```rust
/// DEPRECATED in v0.3.0: Phase 2 introduces IpcServer which replaces this.
/// Kept for backward compatibility if needed, but new code should use injection::IpcServer.
pub struct ThemeManager {
    // ...
}
```

- [ ] **Step 3: Commit if changes made**

```bash
git add src-tauri/src/injector/theme.rs
git commit -m "docs: mark ThemeManager as deprecated in favor of IpcServer"
```

Or if no changes needed:
```bash
git commit --allow-empty -m "refactor: verified ThemeManager still used, no changes needed"
```

---

## Task 6: Final Verification

**Status:** Pending

**Steps:**

- [ ] **Step 1: Build both DLL and main app**

```bash
cd src-tauri
cargo build --lib -p windows-island-injector-dll
cargo build
```

Expected: Both succeed with no errors, no new warnings

- [ ] **Step 2: Check git log**

```bash
git log --oneline -5
```

Expected: See cleanup commits (Tasks 1-5)

- [ ] **Step 3: Create cleanup summary**

Create `docs/CLEANUP_SUMMARY.md`:
- Removed sections
- Lines deleted
- Build status
- Ready for Phase 3 completion

- [ ] **Step 4: Commit**

```bash
git add docs/CLEANUP_SUMMARY.md
git commit -m "docs: document cleanup of Phase 1-2 legacy code"
```

---

**Plan saved:** 2026-05-31
**Execution ready:** Yes
