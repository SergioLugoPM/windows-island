# Windows Island v0.3.0 Phase 1: Injector Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working Rust-based DLL injector that can inject a stub DLL into Explorer.exe and StartMenuExperienceHost.exe, receive theme updates via shared memory, and be toggled on/off from Settings.

**Architecture:** 
- **Injector module** uses CreateRemoteThread + VirtualAllocEx Win32 FFI to enumerate processes and inject DLL
- **Theme Manager** serializes `InjectedTheme` struct to named file mapping (`Local\WindowsIsland_Theme_v1`)
- **C++ DLL stub** is a minimal payload (logs theme reads, hooks placeholder)
- **Tauri integration** exposes three commands: enable/disable injection, check status
- **Settings toggle** in Island.tsx controls injection state

**Tech Stack:** 
- Rust 1.70+ (`windows` crate 0.58 for Win32 FFI)
- C++ (minimal; Visual Studio MSVC 2022)
- Shared memory via CreateFileMappingW + MapViewOfFile
- Process enumeration via CreateToolhelp32Snapshot

---

## Task Checklist

- [ ] Task 1: Create Injector Module (Win32 FFI)
- [ ] Task 2: Create Theme Manager (Shared Memory)
- [ ] Task 3: Create C++ DLL Stub (Minimal Payload)
- [ ] Task 4: Wire Injector + Theme Manager into AppState
- [ ] Task 5: Create Tauri Commands (enable/disable injection)
- [ ] Task 6: Add Settings UI Toggle
- [ ] Task 7: Manual Test on Windows 11

(Detailed task specs in plan document — see following sections)

---

**Plan Status:** Ready for Subagent-Driven Development execution
**Created:** 2026-05-31
