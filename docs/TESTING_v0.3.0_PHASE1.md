# Windows Island v0.3.0 Phase 1 — Manual Testing Guide

**Objective:** Verify DLL injection system works correctly on Windows 11 23H2

## Pre-Test Checklist

- [ ] Windows 11 23H2 (build 22621+)
- [ ] Explorer.exe running (should be always)
- [ ] No antivirus interference expected (DLL is self-signed, will trigger SmartScreen if unsigned)
- [ ] USB drive or dev folder with test DLL accessible

## Build and Deploy

```powershell
cd C:\Users\serch\windows-island

# Build frontend
npm run build

# Build Tauri app + injector DLL
cargo tauri build
```

Expected output:
- `src-tauri/target/release/windows-island-injector-dll.dll` (≈80 KB)
- `src-tauri/target/release/bundle/msi/...msi` or `.exe` (installer)

## Test Flow

### Test 1: Launch Application

- [ ] Run Windows Island from Start Menu or double-click installer
- [ ] App launches, pill widget appears at top of screen
- [ ] Settings opens on long-press (600 ms)
- [ ] No crash on startup

### Test 2: Locate Settings Panel

- [ ] Long-press the island for ≥600 ms
- [ ] Settings panel slides up from bottom
- [ ] Scroll down in settings panel
- [ ] Find "Theme Injection (Experimental)" section with radio buttons
- [ ] Three options visible: Dark, Light, Vidrio
- [ ] Toggle button visible (color: green for "Enable Injection")

### Test 3: Enable Dark Theme Injection

- [ ] Select "Dark" radio button
- [ ] Click "Enable Injection" button
- [ ] Button text changes to "Disable Injection" (red color)
- [ ] Status shows "✓ Active"
- [ ] **Expected behavior:** DLL loads into Explorer.exe silently (no visible crash)

### Test 4: Verify Explorer Still Responsive

- [ ] Click on taskbar (should still work)
- [ ] Open File Explorer from taskbar
- [ ] Navigate folders normally
- [ ] No lag or freezing observed
- [ ] Taskbar theme **may** change if rendering hooks are active (v0.3.0 phase 2)

### Test 5: Toggle to Light Theme

- [ ] In Settings, select "Light" radio button (keep injection enabled)
- [ ] No restart needed — theme update via shared memory
- [ ] Status still shows "✓ Active"
- [ ] Explorer remains responsive

### Test 6: Test Vidrio Theme

- [ ] In Settings, select "Vidrio" radio button
- [ ] Verify injection still active
- [ ] All three themes can be cycled without crashing

### Test 7: Disable Injection

- [ ] Click "Disable Injection" button
- [ ] Button text changes to "Enable Injection" (green)
- [ ] Status shows "○ Inactive"
- [ ] Explorer remains responsive (no cleanup needed — DLL stays loaded)

### Test 8: Re-enable Injection

- [ ] Click "Enable Injection" again
- [ ] Select Dark theme
- [ ] Should work without any issues

### Test 9: Explorer Lifecycle

- [ ] With injection **active**, open a new Explorer window
- [ ] New window also receives the theme (DLL is system-wide)
- [ ] Close and reopen Explorer
- [ ] Injection persists while toggle is "active"

### Test 10: Restart Application

- [ ] Close Windows Island completely
- [ ] Reopen from Start Menu
- [ ] Settings should show "○ Inactive" (state doesn't persist by design)
- [ ] Re-enable injection
- [ ] Should work normally

## Expected Results

| Test | Pass Criteria |
|------|---|
| 1. Launch | App opens, no crash, pill visible |
| 2. Settings | Panel opens, injection section visible, radio + button present |
| 3. Enable Dark | Button toggles, status shows active, Explorer doesn't crash |
| 4. Explorer OK | Taskbar responsive, File Explorer works, no lag |
| 5. Light Theme | Theme toggles, Explorer responsive |
| 6. Vidrio Theme | All three themes work, no crashes |
| 7. Disable | Button toggles, status shows inactive |
| 8. Re-enable | Can toggle on again, no state conflicts |
| 9. Explorer lifecycle | New Explorer windows work, theme persists |
| 10. Restart | State resets correctly, re-enable works |

## Troubleshooting

| Issue | Diagnosis |
|---|---|
| "DLL not found" error | Verify `windows-island-injector-dll.dll` exists in `%APPDATA%\windows-island\` |
| Explorer crashes immediately | Check DLL is compiled for x64 (Release build) |
| Button does nothing | Check Tauri dev console for errors; verify commands are registered |
| Theme doesn't change visually | Expected for v0.3.0 phase 1 (no rendering hooks yet); DLL loads silently |
| Settings panel won't open | Long-press may need ≥600 ms; try 1 second hold |

## Success Criteria for Phase 1

✅ All 10 tests pass → **Phase 1 complete**

Phase 1 focuses on **injection pipeline only**. No visual theme rendering in Explorer yet (that's phase 2).

---

**Test Date:** [fill in]  
**Tester:** [fill in]  
**Windows Build:** [fill in]  
**Results:** [PASS / FAIL]
