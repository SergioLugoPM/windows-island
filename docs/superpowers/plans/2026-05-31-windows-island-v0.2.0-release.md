# Windows Island v0.2.0 Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Release v0.2.0 with network metrics, i18n (es/en), auto-updater support, and establish architecture for v0.3.0 DLL injection motor for taskbar/Start Menu theming.

**Architecture:** 
- **v0.2.0 (this release):** Enable `sysinfo:networks` feature to read network throughput; add basic i18n system with JSON locale files; integrate `tauri-plugin-updater` for OTA updates; tag release to trigger CI auto-build.
- **v0.3.0 (design phase):** Design Rust DLL injector architecture (Injector, HookManager, ThemeManager) for Explorer.exe and StartMenuExperienceHost.exe injection via CreateRemoteThread + MinHook FFI.

**Tech Stack:** 
- sysinfo 0.32 (networks feature)
- Tauri 2.0 (tauri-plugin-updater)
- React 18 + TypeScript (i18n hooks)
- Rust 1.70+ (Windows API FFI for future v0.3.0)

---

## File Structure

**Files to create:**
- `src-tauri/src/i18n.rs` — i18n initialization and locale loading
- `src/hooks/useI18n.ts` — React hook for translations
- `src/locales/en.json` — English strings
- `src/locales/es.json` — Spanish strings
- `docs/ARCHITECTURE_v0.3.0.md` — DLL injector design document

**Files to modify:**
- `src-tauri/Cargo.toml` — enable sysinfo `networks`, add `tauri-plugin-updater`
- `src-tauri/src/lib.rs` — add `mod i18n`, call updater check in setup
- `src-tauri/src/stats.rs` — read network data from sysinfo
- `src/components/StatsView.tsx` — display net_down_kbps, net_up_kbps
- `src/components/Island.tsx` — use i18n hook in settings/weather/battery labels
- `src/App.tsx` — i18n provider initialization
- `src-tauri/tauri.conf.json` — configure updater endpoint
- `CHANGELOG.md` — add v0.2.0 section
- `.github/workflows/release.yml` — already exists, no changes needed (auto-builds on tag)

---

## Task 1: Enable Network Metrics in sysinfo

**Status:** Pending

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/stats.rs:82-86`

Task steps listed in plan document.

---

## Task 2: Create i18n System (Rust Backend)

**Status:** Pending

**Files:**
- Create: `src-tauri/src/i18n.rs`
- Modify: `src-tauri/src/lib.rs`

Task steps listed in plan document.

---

## Task 3: Create i18n React Hook

**Status:** Pending

**Files:**
- Create: `src/hooks/useI18n.ts`
- Create: `src/locales/en.json`
- Create: `src/locales/es.json`

Task steps listed in plan document.

---

## Task 4: Integrate tauri-plugin-updater

**Status:** Pending

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`

Task steps listed in plan document.

---

## Task 5: Update CHANGELOG.md for v0.2.0

**Status:** Pending (depends on Tasks 1–4)

---

## Task 6: Tag v0.2.0 and Push

**Status:** Pending (depends on Task 5)

---

## Task 7: Design v0.3.0 DLL Injector Architecture

**Status:** Pending

**Files:**
- Create: `docs/ARCHITECTURE_v0.3.0.md`

---

**Plan saved:** 2026-05-31
**Execution ready:** Yes
