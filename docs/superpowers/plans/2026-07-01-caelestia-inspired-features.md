# Windows Island v0.4.0 — Caelestia-Inspired Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the feature gap identified against Caelestia Shell — render network stats that are already collected, add disk/GPU stats, redesign the Performance/Media/Weather panels to borrow Caelestia's card-based visual language (icon-headed cards, circular progress rings, sparkline network graph, a dedicated Weather view), apply a wallpaper-derived accent color, and (if a Windows 11 compositor spike proves it works) replace the CSS-only "liquid glass" look with real OS-level blur-behind.

**Architecture:** Six independent phases, each shippable and testable on its own — do not treat this as one monolithic feature. Phases are ordered by risk/effort so low-risk wins land first and gate the two research spikes (GPU counters, native blur) behind explicit go/no-go checkpoints instead of committing to unverified Win32 behavior up front.

**Tech Stack:** Rust/Tauri 2 backend (raw Win32 FFI, following the existing `win_sys` module pattern in `src-tauri/src/lib.rs` — no new `windows` crate features unless a task says so), React/TypeScript frontend, `sysinfo` 0.32 for disk stats, `image` crate (new dependency) for wallpaper color sampling.

**Testing approach — read this before starting:** This project has **no test runner** (`package.json` has no `vitest`/`jest`; `Cargo.toml` has no `#[test]` harness wired to CI). Every prior feature in this codebase was verified by `npm run build` (tsc type-check) + `cargo tauri build` + manually exercising the UI. This plan follows that same pattern instead of inventing a test framework mid-feature: each task's "verify" step is a type-check/build command plus an exact manual action and exact expected result. Do not add vitest/jest as part of this plan — that would be unrequested scope creep.

---

## Phase ordering (do this order unless you have a reason not to)

1. **Phase 1 — Network throughput display** (data already exists, zero backend risk)
2. **Phase 2 — Disk usage stats** (new `sysinfo` feature flag, well-documented API)
3. **Phase 3 — Caelestia-inspired panel redesign** (card-based visuals for Performance/Media/Weather, new dedicated Weather mode — depends on Phases 1-2 for data)
4. **Phase 4 — Wallpaper accent color ("Material Color Engine")** (new `image` dependency + one raw FFI call)
5. **Phase 5 — Native blur-behind spike** (research spike with go/no-go; only continue past Task 5.1 if the spike shows a real visual improvement)
6. **Phase 6 — GPU usage via PDH counters** (highest implementation risk, lowest value/effort ratio — do this last, treat as optional)

> **Descoped:** a "Quick Toggles" panel (WiFi/Bluetooth) was considered and dropped — opening Windows' own native Quick Settings from inside the island would take more steps than just using Windows' existing Win+A shortcut directly. Not worth building.

---

## Phase 1: Network throughput display

The backend already collects `net_down_kbps` / `net_up_kbps` in `SystemStats` (`src-tauri/src/stats.rs:30-31`) and the frontend mirror already has both fields (`src/components/StatsView.tsx:9-10`). They are simply never rendered. This phase is frontend-only.

**Files:**
- Modify: `src/components/StatsView.tsx`

- [ ] **Step 1: Add a kbps formatter and a full-view network row**

Insert after `colorForLoad` (after line 65 in `src/components/StatsView.tsx`):

```typescript
function formatKbps(kbps: number): string {
  if (kbps < 1024) return `${kbps.toFixed(0)} KB/s`;
  return `${(kbps / 1024).toFixed(1)} MB/s`;
}
```

Inside `StatsFull` (`src/components/StatsView.tsx:69-102`), add a network row after the battery block, right before the closing `</div>` of the component's return:

```tsx
      {hasBattery && (
        <StatRow label={t("battery")}
          value={`${s.battery_percent}%`}
          sub={s.battery_charging ? t("charging") : undefined}
          bar={s.battery_percent / 100}
          color={
            s.battery_percent < 20 ? "rgba(255,110,110,0.95)" :
            s.battery_percent < 50 ? "rgba(255,200,90,0.85)" :
            "rgba(120,220,140,0.85)"
          }
        />
      )}
      <div style={{
        display: "flex", justifyContent: "space-between", alignItems: "baseline",
        fontFamily: "-apple-system,'SF Pro Text','Segoe UI',system-ui,sans-serif",
      }}>
        <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: 0.5, color: "rgba(140,170,220,0.7)" }}>
          {t("network")}
        </span>
        <span style={{ fontSize: 11, fontWeight: 600, color: "rgba(220,230,255,0.95)", fontVariantNumeric: "tabular-nums" }}>
          ↓ {formatKbps(s.net_down_kbps)}
          <span style={{ marginLeft: 8 }}>↑ {formatKbps(s.net_up_kbps)}</span>
        </span>
      </div>
```

- [ ] **Step 2: Add a compact network line to the mini view**

Inside `StatsMini` (`src/components/StatsView.tsx:132-140`), add a line after the RAM `MiniLine`:

```tsx
export function StatsMini() {
  const s = useSystemStats();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 3, minWidth: 0 }}>
      <MiniLine label="CPU" pct={s.cpu_percent} color={colorForLoad(s.cpu_percent)} />
      <MiniLine label="RAM" pct={s.ram_percent} color={colorForLoad(s.ram_percent)} />
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 9, fontWeight: 600, color: "rgba(140,170,220,0.65)", letterSpacing: 0.4, width: 22 }}>
          NET
        </span>
        <span style={{ fontSize: 9, fontWeight: 600, color: "rgba(220,230,255,0.9)", fontVariantNumeric: "tabular-nums" }}>
          ↓{formatKbps(s.net_down_kbps)} ↑{formatKbps(s.net_up_kbps)}
        </span>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Add the `network` i18n key**

`src/locales/en.json` — add `"network": "NET",` after `"charging"`.
`src/locales/es.json` — add `"network": "RED",` after `"charging"`.

- [ ] **Step 4: Verify**

Run: `npm run build`
Expected: `✓ built in` with no TS errors.

Run: `cargo tauri dev` (from `src-tauri`, or via existing dev workflow), download/upload something in the background, open the island's Stats view (peek mode set to "stats", or the Full cycle mode).
Expected: a "NET" / "RED" row shows `↓ X KB/s ↑ Y KB/s` and the numbers move when there is network traffic.

- [ ] **Step 5: Commit**

```bash
git add src/components/StatsView.tsx src/locales/en.json src/locales/es.json
git commit -m "feat(stats): render network throughput in stats views"
```

---

## Phase 2: Disk usage stats

`sysinfo` 0.32 is already a dependency but built with `default-features = false, features = ["system", "network"]` (`src-tauri/Cargo.toml:24`) — the `disks` feature is explicitly excluded. Add it and surface `C:\` usage the same way RAM is surfaced.

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/stats.rs`
- Modify: `src/components/StatsView.tsx`

- [ ] **Step 1: Enable the sysinfo `disks` feature**

In `src-tauri/Cargo.toml:24`, change:

```toml
sysinfo     = { version = "0.32", default-features = false, features = ["system", "network"] }
```
to:
```toml
sysinfo     = { version = "0.32", default-features = false, features = ["system", "network", "disk"] }
```

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: compiles. If `sysinfo` 0.32 names the feature differently (e.g. `disks` instead of `disk`), the compiler error will say `unknown feature` — run `cargo doc --manifest-path src-tauri/Cargo.toml -p sysinfo --open` and check the `Disks` struct's location under `Cargo.toml`'s `[features]` to get the exact name, then retry.

- [ ] **Step 2: Add disk fields to `SystemStats` and collect them**

In `src-tauri/src/stats.rs`, add the import and struct fields. Change line 18:

```rust
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
```

Add fields to `SystemStats` (after `pub cpu_temp_c: Option<f32>,` at line 36):

```rust
    /// 0..100 — used / total space on the system drive (C:\)
    pub disk_percent: f32,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
```

Add a `disks: Mutex<Disks>` field to `StatsState` (after `pub networks: Mutex<Networks>,` at line 41):

```rust
    pub disks: Mutex<Disks>,
```

Update `StatsState::new()` (lines 46-58) to initialize it:

```rust
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(networks),
            disks: Mutex::new(disks),
            last_refresh: Mutex::new(Instant::now()),
        }
    }
```

Update `collect()` (`src-tauri/src/stats.rs:67-112`) to refresh disks and compute the C:\ figures. Add after `networks.refresh();` (line 78):

```rust
    let mut disks = state.disks.lock().unwrap();
    disks.refresh(true);
```

Add before the final `SystemStats { ... }` construction:

```rust
    let (disk_used_gb, disk_total_gb, disk_percent) = disks
        .list()
        .iter()
        .find(|d| d.mount_point().to_string_lossy().eq_ignore_ascii_case("c:\\"))
        .or_else(|| disks.list().first())
        .map(|d| {
            let total = d.total_space();
            let avail = d.available_space();
            let used = total.saturating_sub(avail);
            let total_gb = total as f64 / 1_073_741_824.0;
            let used_gb = used as f64 / 1_073_741_824.0;
            let pct = if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 };
            (used_gb, total_gb, pct)
        })
        .unwrap_or((0.0, 0.0, 0.0));
```

And add the three fields to the returned struct literal:

```rust
    SystemStats {
        cpu_percent,
        ram_percent,
        ram_used_mb,
        ram_total_mb,
        net_down_kbps,
        net_up_kbps,
        battery_percent,
        battery_charging,
        cpu_temp_c,
        disk_percent,
        disk_used_gb,
        disk_total_gb,
    }
```

- [ ] **Step 3: Verify the Rust side compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors. `state.disks.lock().unwrap()` must not deadlock with the existing `state.sys`/`state.networks` locks — they're separate `Mutex`es locked sequentially in the same function, not nested, so this is safe.

- [ ] **Step 4: Mirror the fields on the frontend and render them**

In `src/components/StatsView.tsx`, update the `SystemStats` interface (lines 4-14):

```typescript
export interface SystemStats {
  cpu_percent:       number;
  ram_percent:       number;
  ram_used_mb:       number;
  ram_total_mb:      number;
  net_down_kbps:     number;
  net_up_kbps:       number;
  battery_percent:   number;
  battery_charging:  boolean;
  cpu_temp_c:        number | null;
  disk_percent:      number;
  disk_used_gb:      number;
  disk_total_gb:     number;
}
```

Update the default state in `useSystemStats` (lines 18-22):

```typescript
  const [stats, setStats] = useState<SystemStats>({
    cpu_percent: 0, ram_percent: 0, ram_used_mb: 0, ram_total_mb: 0,
    net_down_kbps: 0, net_up_kbps: 0,
    battery_percent: -1, battery_charging: false, cpu_temp_c: null,
    disk_percent: 0, disk_used_gb: 0, disk_total_gb: 0,
  });
```

Add a disk row in `StatsFull`, after the network row added in Phase 1:

```tsx
      <StatRow label={t("disk")}
        value={`${s.disk_percent.toFixed(0)}%`}
        sub={`${s.disk_used_gb.toFixed(0)} / ${s.disk_total_gb.toFixed(0)} GB`}
        bar={s.disk_percent / 100}
        color={colorForLoad(s.disk_percent)}
      />
```

- [ ] **Step 5: Add the `disk` i18n key**

`src/locales/en.json` — add `"disk": "DISK",`.
`src/locales/es.json` — add `"disk": "DISCO",`.

- [ ] **Step 6: Verify**

Run: `npm run build`
Expected: no TS errors.

Run the app, open Full stats view.
Expected: a DISK row shows a plausible percentage and `used / total GB` for your `C:\` drive (cross-check against Windows' own File Explorer "This PC" free-space display — should match within a GB or two).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/stats.rs src/components/StatsView.tsx src/locales/en.json src/locales/es.json
git commit -m "feat(stats): add C: drive usage to system stats"
```

---

## Phase 3: Caelestia-inspired panel redesign

Reference: 4 screenshots of Caelestia Shell's actual dashboard (Dashboard/Media/Performance/Weather tabs) were reviewed directly for this phase. We are **not** copying the tabbed-dashboard architecture (HaloW's pill already cycles through discrete modes via click/scroll — adding an internal tab bar on top of that would be redundant). What we *are* borrowing is the visual language:

- **Performance tab:** each metric lives in its own rounded card with an icon+label header. CPU/GPU use a horizontal bar; RAM/Storage use a circular progress ring with the percentage centered inside it; Network shows a small history sparkline plus a Down/Up/Total line list; Battery is visually distinct (tall, colored by charge level).
- **Media tab:** friendly empty-state copy ("Play some music for stuff to show up here!" instead of a bare "no session" label) and the album-art slot rendered as a dashed-outline placeholder frame when idle.
- **Weather tab:** a hero readout (icon + big temp + description) plus a row of small stat cards for Humidity / Feels Like / Wind — HaloW's `WeatherView` already has an unused richer branch (`src/components/WeatherView.tsx:82-91`, only ever invoked with `compact` today) that this phase extends instead of replacing.

**Files:**
- Create: `src/components/RingStat.tsx`
- Modify: `src/components/StatsView.tsx`
- Modify: `src/components/MediaView.tsx`
- Modify: `src/components/WeatherView.tsx`
- Modify: `src-tauri/src/weather.rs`
- Modify: `src/components/Island.tsx`
- Modify: `src/App.css`
- Modify: `src/locales/en.json`, `src/locales/es.json`

### Task 3.1: Circular ring-stat component

- [ ] **Step 1: Create `RingStat`**

```tsx
// src/components/RingStat.tsx
interface RingStatProps {
  percent: number; // 0-100
  color: string;   // any valid CSS color, e.g. "rgba(100,180,255,0.9)"
  size?: number;    // px, default 56
  label: string;
  sub: string;
}

export function RingStat({ percent, color, size = 56, label, sub }: RingStatProps) {
  const stroke = 5;
  const r = (size - stroke) / 2;
  const circumference = 2 * Math.PI * r;
  const clamped = Math.max(0, Math.min(100, percent));
  const offset = circumference * (1 - clamped / 100);

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 4, minWidth: size }}>
      <div style={{ position: "relative", width: size, height: size }}>
        <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
          <circle cx={size / 2} cy={size / 2} r={r} fill="none"
            stroke="rgba(255,255,255,0.08)" strokeWidth={stroke} />
          <circle cx={size / 2} cy={size / 2} r={r} fill="none"
            stroke={color} strokeWidth={stroke} strokeLinecap="round"
            strokeDasharray={circumference} strokeDashoffset={offset}
            style={{ transition: "stroke-dashoffset 0.4s ease" }} />
        </svg>
        <div style={{
          position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: size * 0.24, fontWeight: 700, color: "rgba(230,235,255,0.95)",
          fontVariantNumeric: "tabular-nums",
        }}>
          {clamped.toFixed(0)}%
        </div>
      </div>
      <div style={{ fontSize: 9, fontWeight: 600, letterSpacing: 0.4, color: "rgba(140,170,220,0.7)", textTransform: "uppercase" }}>
        {label}
      </div>
      <div style={{ fontSize: 9, color: "rgba(200,210,235,0.55)", fontVariantNumeric: "tabular-nums" }}>
        {sub}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify**

Run: `npm run build`
Expected: no TS errors (this file isn't imported anywhere yet, so this just checks it compiles standalone).

- [ ] **Step 3: Commit**

```bash
git add src/components/RingStat.tsx
git commit -m "feat(ui): add circular RingStat component"
```

### Task 3.2: Performance panel card-grid redesign

This assumes Phase 1 (network) and Phase 2 (disk) are already done — `SystemStats` must already have `net_down_kbps`/`net_up_kbps`/`disk_percent`/`disk_used_gb`/`disk_total_gb`.

- [ ] **Step 1: Add card + header styles**

In `src/App.css`, add near `.island-glass` (after line 36):

```css
/* ─── Stat cards (Performance panel) ─── */
.stat-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.stat-card-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: rgba(140, 170, 220, 0.75);
  text-transform: uppercase;
  font-family: -apple-system, 'SF Pro Text', 'Segoe UI', system-ui, sans-serif;
}

.stat-card-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  width: 100%;
}

.stat-card-rings {
  display: flex;
  justify-content: space-around;
  width: 100%;
}
```

- [ ] **Step 2: Add a network sparkline hook**

In `src/components/StatsView.tsx`, add after `useSystemStats` (after line 39):

```typescript
/** Keeps a rolling window of recent net_down_kbps samples for the sparkline. */
function useNetHistory(current: number, size = 20) {
  const [history, setHistory] = useState<number[]>(() => new Array(size).fill(0));
  useEffect(() => {
    setHistory(h => [...h.slice(1), current]);
  }, [current]);
  return history;
}

function Sparkline({ values, color, width = 70, height = 24 }: {
  values: number[]; color: string; width?: number; height?: number;
}) {
  const max = Math.max(1, ...values);
  const points = values
    .map((v, i) => {
      const x = (i / (values.length - 1)) * width;
      const y = height - (v / max) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg width={width} height={height} style={{ overflow: "visible" }}>
      <polyline points={points} fill="none" stroke={color} strokeWidth={1.5}
        strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}
```

- [ ] **Step 3: Rebuild `StatsFull` as a card grid**

Replace the entire `StatsFull` function (`src/components/StatsView.tsx:69-102`) with:

```tsx
export function StatsFull() {
  const s = useSystemStats();
  const { t } = useI18n();
  const hasBattery = s.battery_percent >= 0;
  const netHistory = useNetHistory(s.net_down_kbps);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="stat-card">
        <div className="stat-card-header">{t("cpu")}</div>
        <StatBar value={s.cpu_percent / 100} color={colorForLoad(s.cpu_percent)} />
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "rgba(200,210,235,0.6)" }}>
          <span>{s.cpu_percent.toFixed(0)}%</span>
          {s.cpu_temp_c !== null && <span>{s.cpu_temp_c.toFixed(0)}°C</span>}
        </div>
      </div>

      <div className="stat-card">
        <div className="stat-card-header">{t("network")}</div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
          <Sparkline values={netHistory} color="rgba(120,200,140,0.85)" />
          <div style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 9, fontVariantNumeric: "tabular-nums", color: "rgba(220,230,255,0.9)" }}>
            <span>↓ {formatKbps(s.net_down_kbps)}</span>
            <span>↑ {formatKbps(s.net_up_kbps)}</span>
          </div>
        </div>
      </div>

      <div className="stat-card-rings">
        <RingStat percent={s.ram_percent} color="rgba(170,130,255,0.85)"
          label={t("ram")} sub={`${(s.ram_used_mb / 1024).toFixed(1)}/${(s.ram_total_mb / 1024).toFixed(1)} GB`} />
        <RingStat percent={s.disk_percent} color="rgba(100,200,220,0.85)"
          label={t("disk")} sub={`${s.disk_used_gb.toFixed(0)}/${s.disk_total_gb.toFixed(0)} GB`} />
      </div>

      {hasBattery && (
        <div className="stat-card">
          <div className="stat-card-header">{t("battery")}</div>
          <StatBar value={s.battery_percent / 100} color={
            s.battery_percent < 20 ? "rgba(255,110,110,0.95)" :
            s.battery_percent < 50 ? "rgba(255,200,90,0.85)" :
            "rgba(120,220,140,0.85)"
          } />
          <div style={{ fontSize: 9, color: "rgba(200,210,235,0.6)" }}>
            {s.battery_percent}%{s.battery_charging ? ` · ${t("charging")}` : ""}
          </div>
        </div>
      )}
    </div>
  );
}
```

Add the import at the top of `src/components/StatsView.tsx`:

```typescript
import { RingStat } from "./RingStat";
```

- [ ] **Step 4: Grow the `stats` mode dimensions to fit the grid**

In `src/components/Island.tsx`, update the `stats` entry in `DIMS` (around line 86):

```typescript
  stats:    { w: 300, h: 240, r: 22 },
```

(This height is a starting point — Step 6 has you tweak it visually if content clips or leaves excess empty space.)

- [ ] **Step 5: Verify types**

Run: `npm run build`
Expected: no TS errors. If `StatBar`/`colorForLoad`/`formatKbps` aren't in scope, confirm Phase 1's `formatKbps` and the pre-existing `StatBar`/`colorForLoad` (lines 44-65 of the original file) are still present above `StatsFull` — this task only replaces the `StatsFull` function body, not the whole file.

- [ ] **Step 6: Verify visually and tune `h`**

Run: `npm run build && cargo tauri build`, install, cycle to Stats mode (click through peek → media → stats, or use the existing mode-cycle interaction).
Expected: CPU bar card, Network sparkline card, RAM/Disk rings side by side, Battery card (if present) — all visible without clipping or a scrollbar, no excess empty space below the last card. Adjust `stats.h` in `DIMS` up or down until it fits tightly, then re-run `cargo tauri build` and re-check.

- [ ] **Step 7: Commit**

```bash
git add src/components/StatsView.tsx src/components/Island.tsx src/App.css
git commit -m "feat(ui): redesign Performance panel as icon-card grid with rings and sparkline"
```

### Task 3.3: Friendlier Media empty state

- [ ] **Step 1: Update copy and add a placeholder frame**

In `src/components/MediaView.tsx`, replace the empty-state branch inside `MediaFull` (lines 124-130):

```tsx
  if (!info.has_session) {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: 14, width: "100%" }}>
        <div style={{
          width: 64, height: 64, borderRadius: "50%", flexShrink: 0,
          border: "2px dashed rgba(255,255,255,0.18)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 22, color: "rgba(255,255,255,0.25)",
        }}>
          ♪
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <span className="media-title" style={{ color: "rgba(255,255,255,0.75)" }}>
            {t("noMedia")}
          </span>
          <span className="empty-label" style={{ fontSize: 10 }}>
            {t("noMediaHint")}
          </span>
        </div>
      </div>
    );
  }
```

Add `import { useI18n } from "../hooks/useI18n";` at the top of `src/components/MediaView.tsx`, and add `const { t } = useI18n();` as the first line inside `MediaFull` (before the `useMediaInfo()` call).

- [ ] **Step 2: Add the two new i18n keys**

`src/locales/en.json` — add:
```json
  "noMedia": "No media",
  "noMediaHint": "Play something for it to show up here",
```
`src/locales/es.json` — add:
```json
  "noMedia": "Sin reproducción",
  "noMediaHint": "Reproduce algo para verlo aquí",
```

- [ ] **Step 3: Verify**

Run: `npm run build`
Expected: no TS errors.

Run the app with no media session active (close any music/video players), open Media mode.
Expected: dashed circle placeholder + "No media" / "Sin reproducción" + hint text, instead of the old single flat "Sin reproducción activa" label.

- [ ] **Step 4: Commit**

```bash
git add src/components/MediaView.tsx src/locales/en.json src/locales/es.json
git commit -m "feat(ui): friendlier empty state for Media panel"
```

### Task 3.4: Dedicated Weather mode with Humidity/Feels Like/Wind

`wttr.in`'s `j1` JSON format was checked directly against a live response — `current_condition[0]` includes `FeelsLikeC`, `humidity`, and `windspeedKmph` as top-level string fields, same shape as the existing `temp_C` field this file already parses.

- [ ] **Step 1: Extend the backend `WeatherInfo`**

In `src-tauri/src/weather.rs`, update the struct (lines 3-9):

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeatherInfo {
    pub temp_c: i32,
    pub description: String,
    pub icon_code: String,
    pub city: String,
    pub humidity: i32,
    pub feels_like_c: i32,
    pub wind_kmph: i32,
}
```

Update `WttrCurrent` (lines 19-27) to capture the new fields:

```rust
#[derive(Deserialize)]
struct WttrCurrent {
    #[serde(rename = "temp_C")]
    temp_c: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    humidity: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WttrValue>,
    #[serde(rename = "weatherCode")]
    weather_code: String,
}
```

Update `get_weather` (lines 42-91) to parse and return them — after the existing `let temp_c: i32 = cur.temp_c.parse()...` line, add:

```rust
    let humidity: i32 = cur.humidity.parse().unwrap_or(0);
    let feels_like_c: i32 = cur.feels_like_c.parse().unwrap_or(temp_c);
    let wind_kmph: i32 = cur.windspeed_kmph.parse().unwrap_or(0);
```

And add the three fields to the final `Ok(WeatherInfo { ... })` literal:

```rust
    Ok(WeatherInfo {
        temp_c,
        description,
        icon_code: cur.weather_code,
        city: city_name,
        humidity,
        feels_like_c,
        wind_kmph,
    })
```

- [ ] **Step 2: Verify the Rust side**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors.

- [ ] **Step 3: Mirror the fields and enrich the full weather view**

In `src/components/WeatherView.tsx`, update the `WeatherInfo` interface (lines 3-8):

```typescript
export interface WeatherInfo {
  temp_c: number;
  description: string;
  icon_code: string;
  city: string;
  humidity: number;
  feels_like_c: number;
  wind_kmph: number;
}
```

Replace the non-compact return block (lines 82-91) — this branch already exists but is currently unused by any call site; Task 3.4 Step 6 is what starts using it:

```tsx
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, width: "100%" }}>
      <div className="weather-row">
        <div className="weather-icon">{getIcon(weather.icon_code)}</div>
        <div className="weather-info">
          <div className="weather-temp">{weather.temp_c}°C</div>
          <div className="weather-desc">{weather.description}</div>
          <div className="weather-city">{weather.city}</div>
        </div>
      </div>
      <div className="stat-card-grid">
        <div className="stat-card">
          <div className="stat-card-header">Humidity</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.humidity}%
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-header">Feels like</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.feels_like_c}°C
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-header">Wind</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.wind_kmph} km/h
          </div>
        </div>
      </div>
    </div>
  );
```

- [ ] **Step 4: Add a dedicated `weather` cycle mode**

In `src/components/Island.tsx`, update the `Mode` type (line 16):

```typescript
type Mode = "idle" | "peek" | "media" | "stats" | "weather" | "full" | "settings";
```

Add a `weather` entry to `DIMS` (near the `stats` entry updated in Task 3.2):

```typescript
  weather:  { w: 320, h: 210, r: 26 },
```

Update `CYCLE` (line 95):

```typescript
const CYCLE: Mode[] = ["peek", "media", "stats", "weather", "full"];
```

Add the JSX block right after the `{/* ── STATS ── */}` block (after the `)}` that closes it, before `{/* ── FULL ── */}`):

```tsx
            {/* ── WEATHER ── */}
            {mode === "weather" && (
              <motion.div key="weather"
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={springFast}
                style={{ width: "100%" }}
              >
                <WeatherView />
              </motion.div>
            )}
```

- [ ] **Step 5: Check for other places that assume the old `Mode` union or `CYCLE` contents**

Run: `grep -n "CYCLE\[" src/components/Island.tsx` and `grep -n ": Mode" src/components/Island.tsx` (or use the Grep tool) to confirm nothing else hardcodes the old 4-entry cycle length or exhaustively switches over `Mode` without a `weather` case. If a `switch(mode)` exists anywhere without a `weather` case and without a `default`, add one following the same pattern as the `stats` case.

- [ ] **Step 6: Verify**

Run: `npm run build && cargo tauri build`
Expected: both succeed.

Install, cycle through modes until you reach the new Weather mode.
Expected: hero temp/icon/description row, plus three small cards for Humidity/Feels like/Wind with real numbers from `wttr.in`. Resize/multi-monitor: drag to a secondary monitor and confirm the Weather mode still renders correctly sized and centered (same mechanism already validated for other modes).

- [ ] **Step 7: Add the `noMedia`/`noMediaHint` keys already covered in Task 3.3 — no new keys needed here** (Humidity/Feels like/Wind labels are left in English inline above rather than i18n keys, matching how `StatRow` labels like "CPU"/"RAM" are already partly hardcoded in `StatsView.tsx`; add `humidity`/`feelsLike`/`wind` i18n keys instead if you want full translation parity — optional, not required for this task to be done).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/weather.rs src/components/WeatherView.tsx src/components/Island.tsx
git commit -m "feat(weather): add dedicated Weather mode with humidity/feels-like/wind"
```

---

## Phase 4: Wallpaper accent color ("Material Color Engine")

Caelestia extracts a color from the wallpaper and tints the whole shell. We'll do a lighter version: read the current wallpaper file path via `SPI_GETDESKWALLPAPER`, decode it, compute an average color, and use that as the accent instead of the fixed Windows blue. Only one place in the codebase currently hardcodes that accent — `.media-progress-fill` in `src/App.css:347-348` — so the blast radius is small.

This intentionally skips the `IDesktopWallpaper` COM interface (which would give per-monitor wallpaper paths on multi-monitor Windows 11 setups) because it's unverified whether the `windows` 0.58 crate exposes it under a known feature flag from this environment. `SPI_GETDESKWALLPAPER` is a plain, stable Win32 call that needs no new crate feature and returns the primary wallpaper path, which is good enough for a single global accent color.

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/App.css`
- Modify: `src/components/Island.tsx`

- [ ] **Step 1: Add the `image` crate**

In `src-tauri/Cargo.toml`, under `[dependencies]` (after `sysinfo`, line 24):

```toml
image       = { version = "0.25", default-features = false, features = ["jpeg", "png", "bmp"] }
```

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: dependency resolves and compiles (adds a moderate number of transitive crates — that's expected for an image decoder).

- [ ] **Step 2: Relax `SystemParametersInfoW`'s signature to accept a string buffer**

The existing declaration (`src-tauri/src/lib.rs:60-63`) is typed for `SPI_GETWORKAREA`'s `RECT` output. `SPI_GETDESKWALLPAPER` instead writes a UTF-16 string into a caller-provided buffer, so the parameter needs to be a generic pointer. Change:

```rust
        pub fn SystemParametersInfoW(
            ui_action: u32, ui_param: u32,
            pv_param:  *mut RECT, f_win_ini: u32,
        ) -> i32;
```
to:
```rust
        pub fn SystemParametersInfoW(
            ui_action: u32, ui_param: u32,
            pv_param:  *mut core::ffi::c_void, f_win_ini: u32,
        ) -> i32;
```

Update the one existing call site, `work_area_bottom()` (`src-tauri/src/lib.rs:163-166`), to cast the `RECT` pointer:

```rust
    pub fn work_area_bottom() -> i32 {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe { SystemParametersInfoW(0x0030, 0, &mut rect as *mut RECT as *mut core::ffi::c_void, 0); }
        rect.bottom
    }
```

(Re-read the rest of that function below line 166 first — only the `SystemParametersInfoW` call itself changes; do not alter the return logic.)

- [ ] **Step 3: Add a wallpaper-path + accent-color function to `win_sys`**

Add after `work_area_bottom()`:

```rust
    /// Read the current desktop wallpaper's file path (SPI_GETDESKWALLPAPER = 0x0073).
    /// Returns None if there is no wallpaper file (solid color background) or the
    /// call fails.
    pub fn wallpaper_path() -> Option<String> {
        const SPI_GETDESKWALLPAPER: u32 = 0x0073;
        const MAX_PATH: usize = 260;
        let mut buf: [u16; MAX_PATH] = [0; MAX_PATH];
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETDESKWALLPAPER, MAX_PATH as u32,
                buf.as_mut_ptr() as *mut core::ffi::c_void, 0,
            )
        };
        if ok == 0 { return None; }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        if len == 0 { return None; }
        Some(String::from_utf16_lossy(&buf[..len]))
    }
```

- [ ] **Step 4: Add the accent-color Tauri command**

Add near `get_system_stats` (`src-tauri/src/lib.rs:470-473`):

```rust
#[derive(serde::Serialize)]
struct AccentColor { r: u8, g: u8, b: u8 }

#[tauri::command]
async fn get_wallpaper_accent() -> Option<AccentColor> {
    #[cfg(target_os = "windows")]
    {
        let path = win_sys::wallpaper_path()?;
        let img = image::open(&path).ok()?;
        let small = img.resize_exact(16, 16, image::imageops::FilterType::Nearest).to_rgb8();
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for px in small.pixels() {
            r += px[0] as u64;
            g += px[1] as u64;
            b += px[2] as u64;
            n += 1;
        }
        if n == 0 { return None; }
        Some(AccentColor { r: (r / n) as u8, g: (g / n) as u8, b: (b / n) as u8 })
    }
    #[cfg(not(target_os = "windows"))]
    None
}
```

Register it in `invoke_handler`, add `get_wallpaper_accent,` after `get_system_stats,`.

- [ ] **Step 5: Verify the Rust side**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors.

- [ ] **Step 6: Wire the accent color into CSS via a custom property**

In `src/App.css`, add a fallback default at the top of the file (right after the `*, *::before, *::after` reset block, before `.island-outer`):

```css
:root {
  --accent-rgb: 0, 120, 212; /* Windows Fluent blue — overridden by JS if wallpaper sampling succeeds */
}
```

Replace the hardcoded accent in `.media-progress-fill` (`src/App.css:347-348`):

```css
  background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.85), rgba(var(--accent-rgb), 0.70));
  box-shadow: 0 0 6px rgba(var(--accent-rgb), 0.40);
```

- [ ] **Step 7: Fetch and apply the accent color on startup**

In `src/components/Island.tsx`, add a `useEffect` near the existing Windows-theme-sync effect (search for `get_windows_theme` — this is the effect documented in the conversation summary). Add a sibling effect:

```typescript
  // ── Sample wallpaper accent color and expose it as a CSS var ──
  useEffect(() => {
    if (!isTauri) return;
    invoke<{ r: number; g: number; b: number } | null>('get_wallpaper_accent')
      .then(accent => {
        if (accent) {
          document.documentElement.style.setProperty(
            '--accent-rgb', `${accent.r}, ${accent.g}, ${accent.b}`
          );
        }
      })
      .catch(() => {});
  }, []);
```

- [ ] **Step 8: Verify**

Run: `npm run build && cargo tauri build`
Expected: both succeed.

Install the build, switch your Windows desktop wallpaper to something with a strong, distinct color (e.g. a solid-ish red or green photo), relaunch HaloW, open the media view while music is playing.
Expected: the progress bar fill color visibly shifts toward the wallpaper's dominant tone instead of staying Windows blue. Switch back to a blue-dominant wallpaper and confirm it shifts back.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src/App.css src/components/Island.tsx
git commit -m "feat(theme): derive accent color from desktop wallpaper"
```

---

## Phase 5: Native blur-behind spike (gated — read before starting)

This is the "refactorizar el liquid glass con las limitaciones de Windows" ask. Current state: `.island-glass` (`src/App.css:22-36`) is a fully **opaque** `rgba(28,28,30,0.94)` panel with CSS/SVG-painted highlights (`LiquidGlassChrome.tsx`) — it never actually shows the blurred desktop behind it, it only *looks* glassy. There's already one attempt at real OS backdrop in this codebase: `set_mica_effect` (`src-tauri/src/lib.rs:364-378`) toggles `DWMWA_SYSTEMBACKDROP_TYPE` (Mica), wired to the "glass" theme option — but Mica is **force-disabled by default at startup** (`src-tauri/src/lib.rs:618-625`) because it paints the window's full rectangle and ignores the CSS border-radius on borderless custom-shaped windows, breaking the pill shape.

`DwmEnableBlurBehindWindow` is a different, older DWM API (present since Vista, still callable on Windows 11) that was specifically designed for irregularly-shaped windows: it takes an `HRGN` describing exactly which part of the window should be blurred, so — unlike Mica — it can be clipped to the pill's rounded shape via `CreateRoundRectRgn`. This is the same technique used by third-party pill-shaped overlay apps (e.g. ElevenClock) for shaped blur. **However**, blur-behind's actual visual quality on Windows 11 is inconsistent depending on Windows build and the user's "Transparency effects" setting, and it has not been tested in this codebase. Do not skip the spike and wire this straight into production.

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/Island.tsx` (spike only — revert if no-go)

- [ ] **Step 1 (SPIKE): Add the raw FFI**

Add to the `win_sys` extern block (`src-tauri/src/lib.rs`, inside the block starting line 58):

```rust
        fn DwmEnableBlurBehindWindow(hwnd: isize, p_blur_behind: *const DwmBlurBehind) -> i32;
```

Add outside the extern block but still inside `mod win_sys` (near the `RECT`/`POINT` struct definitions, lines 54-56):

```rust
    #[repr(C)]
    pub struct DwmBlurBehind {
        pub dw_flags: u32,
        pub f_enable: i32,
        pub h_rgn_blur: isize,
        pub f_transition_on_maximized: i32,
    }
```

Add the gdi32 region functions in a separate extern block (gdi32 is a different DLL than user32/dwmapi — the existing block has no `#[link(name = ...)]` because it's resolving against implicit default-linked system DLLs; follow the existing style but note gdi32 needs an explicit link like `batt::GetSystemPowerStatus` does in `stats.rs:127`):

```rust
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(
            x1: i32, y1: i32, x2: i32, y2: i32,
            cx_corner: i32, cy_corner: i32,
        ) -> isize;
    }
```

Add the wrapper function after `set_backdrop`:

```rust
    /// Enable blur-behind clipped to a rounded-rectangle region matching the
    /// window's current pill shape. `w`/`h`/`radius` are PHYSICAL pixels.
    ///
    /// SAFETY / OWNERSHIP: once passed to DwmEnableBlurBehindWindow, the
    /// system takes ownership of the HRGN — do NOT call DeleteObject on it
    /// afterward (same rule as SetWindowRgn).
    pub fn enable_blur_behind(hwnd_isize: isize, w: i32, h: i32, radius: i32) -> bool {
        const DWM_BB_ENABLE: u32 = 0x1;
        const DWM_BB_BLURREGION: u32 = 0x2;
        let hrgn = unsafe { CreateRoundRectRgn(0, 0, w, h, radius * 2, radius * 2) };
        if hrgn == 0 { return false; }
        let bb = DwmBlurBehind {
            dw_flags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            f_enable: 1,
            h_rgn_blur: hrgn,
            f_transition_on_maximized: 0,
        };
        let hr = unsafe { DwmEnableBlurBehindWindow(hwnd_isize, &bb) };
        hr == 0 // S_OK
    }
```

- [ ] **Step 2 (SPIKE): Add a temporary debug command wired to a keyboard-free trigger**

Add a throwaway command (delete in Step 5 if no-go):

```rust
#[tauri::command]
async fn debug_enable_blur(app: tauri::AppHandle, label: Option<String>) -> bool {
    #[cfg(target_os = "windows")]
    if let Some(win) = island_win(&app, label.as_deref()) {
        if let Ok(hwnd) = win.hwnd() {
            let hwnd_raw: isize = unsafe { std::mem::transmute_copy(&hwnd) };
            if let Ok(size) = win.inner_size() {
                return win_sys::enable_blur_behind(hwnd_raw, size.width as i32, size.height as i32, 32);
            }
        }
    }
    false
}
```

Register `debug_enable_blur,` in `invoke_handler` temporarily.

- [ ] **Step 3 (SPIKE): Trigger it from the running app and look at it**

Run: `cargo tauri dev`. Open the browser devtools for the island window (right-click if enabled, or add a temporary `useEffect(() => { invoke('debug_enable_blur', { label: winLabelRef.current }); }, [])` in `Island.tsx`, run, then remove it after observing).

Expected outcomes to look for, with the island sitting over a busy/colorful part of your desktop (not a solid-color background — you need visual detail behind it to see blur):
- **GO:** the pill visibly shows a soft-focus, blurred version of whatever is behind it (icons, window content) instead of the flat `rgba(28,28,30,0.94)` fill.
- **NO-GO:** no visible change, a visual glitch (e.g. black rectangle, flicker, wrong-shaped clipping), or the effect only appears/disappears based on the Windows "Transparency effects" toggle in a way that's not gracefully detectable.

Record which one you saw before proceeding — do not skip this.

- [ ] **Step 4a (IF GO): Wire it permanently**

1. Change `.island-glass` background (`src/App.css:26`) from `rgba(28, 28, 30, 0.94)` to `rgba(28, 28, 30, 0.55)` **only** — leave every other layer (`::before`, `::after`, the specular/caustic layers in `LiquidGlassChrome.tsx`) untouched, they still add value on top of real blur.
2. In `src-tauri/src/lib.rs`, call `win_sys::enable_blur_behind` (not the throwaway `debug_enable_blur`) at the end of `resize_window` (line 405-422), `snap_to_edge` (line 424-456), and `resize_anchor_bottom` (line 380-402) — after the existing `set_size`/`set_position` calls, using the same `w`/`h` (converted to physical pixels via the window's `scale_factor()`) and the mode's corner radius. The corner radius must be threaded through from the frontend the same way `w`/`h` already are (add an `r: f64` parameter to all three commands and to the matching JS call sites in `resizeToMode`/`snapToEdge` in `Island.tsx`, sourcing it from `getModeDims(...).r`).
3. Also call it once in `setup()` right after each window (`main` and every `island_N`) is created, using that window's initial `idle` dims.
4. Make the Tauri commands return `bool` (success) instead of `()`, and in `Island.tsx` set a `blur-native` class on `.island-outer` only when the **first** successful call reports `true`, so CSS can key off it if you want a different look for the native-blur vs fallback path. If it returns `false`, do nothing extra — the `0.55`-opacity CSS layer alone would look under-saturated without real blur behind it, so gate the opacity change on this class too:

```css
.island-glass { background: rgba(28, 28, 30, 0.94); } /* fallback: unchanged */
.blur-native .island-glass { background: rgba(28, 28, 30, 0.55); } /* only when native blur is confirmed active */
```
5. Remove `debug_enable_blur` and its registration.
6. Consider (separate follow-up, not required here): retire the broken Mica `set_mica_effect` path and the "glass" theme option now that this replaces its intent — flag this to the user rather than silently deleting a user-facing setting.

- [ ] **Step 4b (IF NO-GO): Clean up and stop**

1. Remove `debug_enable_blur` and its registration.
2. Leave `enable_blur_behind`/`DwmBlurBehind`/`CreateRoundRectRgn` FFI in place *only* if you want to revisit later — otherwise delete them; don't leave dead unsafe FFI lying around per this project's YAGNI conventions.
3. Leave `.island-glass` exactly as it is today. The CSS-only glass already looks intentional; don't ship a half-working native path.
4. Report the observed failure mode back so it's documented (e.g. "no visible blur on Windows 11 24H2 with Transparency effects ON").

- [ ] **Step 5: Verify (GO path only)**

Run: `npm run build && cargo tauri build`
Expected: both succeed.

Install, drag the island over different desktop content, resize between idle/peek/media/full modes, move it to a secondary monitor.
Expected: blur clipping stays correctly rounded at every size, on every monitor, through every resize — no square corners, no stale blur region left over from a previous size.

- [ ] **Step 6: Commit**

GO path:
```bash
git add src-tauri/src/lib.rs src/App.css src/components/Island.tsx
git commit -m "feat(glass): replace CSS-only glass with native DWM blur-behind"
```

NO-GO path:
```bash
git add -A
git commit -m "chore(glass): spike native blur-behind — no-go, keep CSS glass"
```

---

## Phase 6: GPU usage via PDH counters (optional — do last)

Windows exposes per-adapter GPU utilization through the same Performance Data Helper (PDH) counters Task Manager itself reads (`\GPU Engine(*)\Utilization Percentage`), rather than any vendor SDK. This avoids adding NVML/ADL dependencies but the counter path requires wildcard expansion and PDH is fiddly raw FFI that hasn't been exercised in this codebase — treat this exactly like Phase 5: spike first, wire in only if it works and gives plausible numbers.

**Files:**
- Modify: `src-tauri/Cargo.toml` (link `pdh.dll`)
- Modify: `src-tauri/src/stats.rs`
- Modify: `src/components/StatsView.tsx`

- [ ] **Step 1 (SPIKE): Add PDH FFI bindings**

Add a new module `src-tauri/src/gpu_stats.rs`:

```rust
//! GPU utilization via the same PDH "GPU Engine" counters Task Manager reads.
//! SPIKE STATUS: unverified in this codebase — see Phase 6 of the v0.4.0 plan.

use std::ffi::c_void;

#[repr(C)]
struct PdhFmtCounterValue {
    c_status: u32,
    double_value: f64,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(sz_data_source: *const u16, dw_user_data: usize, phquery: *mut *mut c_void) -> i32;
    fn PdhAddEnglishCounterW(
        hquery: *mut c_void, sz_full_counter_path: *const u16,
        dw_user_data: usize, phcounter: *mut *mut c_void,
    ) -> i32;
    fn PdhExpandWildCardPathW(
        sz_data_source: *const u16, sz_wild_card_path: *const u16,
        m_sz_expanded_path_list: *mut u16, pcch_path_list_length: *mut u32,
        dw_flags: u32,
    ) -> i32;
    fn PdhCollectQueryData(hquery: *mut c_void) -> i32;
    fn PdhGetFormattedCounterValue(
        hcounter: *mut c_void, dw_format: u32,
        lpdw_type: *mut u32, p_value: *mut PdhFmtCounterValue,
    ) -> i32;
    fn PdhCloseQuery(hquery: *mut c_void) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Best-effort GPU utilization percent, summed across all engine instances.
/// Returns None on any PDH failure — caller must not treat that as "0% usage".
pub fn read_gpu_percent() -> Option<f32> {
    const PDH_FMT_DOUBLE: u32 = 0x00000200;
    unsafe {
        let mut query: *mut c_void = std::ptr::null_mut();
        if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 { return None; }

        let wildcard = to_wide(r"\GPU Engine(*)\Utilization Percentage");
        let mut needed: u32 = 0;
        PdhExpandWildCardPathW(std::ptr::null(), wildcard.as_ptr(), std::ptr::null_mut(), &mut needed, 0);
        if needed == 0 { PdhCloseQuery(query); return None; }
        let mut expanded: Vec<u16> = vec![0; needed as usize];
        if PdhExpandWildCardPathW(std::ptr::null(), wildcard.as_ptr(), expanded.as_mut_ptr(), &mut needed, 0) != 0 {
            PdhCloseQuery(query);
            return None;
        }

        // `expanded` is a double-null-terminated list of null-terminated strings.
        let mut counters = Vec::new();
        let mut start = 0usize;
        for i in 0..expanded.len() {
            if expanded[i] == 0 {
                if i == start { break; }
                let path = String::from_utf16_lossy(&expanded[start..i]);
                let path_w = to_wide(&path);
                let mut hcounter: *mut c_void = std::ptr::null_mut();
                if PdhAddEnglishCounterW(query, path_w.as_ptr(), 0, &mut hcounter) == 0 {
                    counters.push(hcounter);
                }
                start = i + 1;
            }
        }
        if counters.is_empty() { PdhCloseQuery(query); return None; }

        // First sample primes the counters; second sample (after the caller's
        // own polling interval has already elapsed since the previous call)
        // gives a meaningful value. Two back-to-back collects here with no
        // delay will read as ~0 — this matches how `collect()` in stats.rs
        // is called on a steady 1.5s timer, which is enough delay in practice
        // since this function is called once per tick, not twice per tick.
        PdhCollectQueryData(query);

        let mut total = 0.0f64;
        for h in &counters {
            let mut val = PdhFmtCounterValue { c_status: 0, double_value: 0.0 };
            if PdhGetFormattedCounterValue(*h, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut val) == 0 {
                total += val.double_value;
            }
        }
        PdhCloseQuery(query);
        Some(total.min(100.0) as f32)
    }
}
```

- [ ] **Step 2 (SPIKE): Register the module and add a temporary debug print**

In `src-tauri/src/lib.rs`, add `pub mod gpu_stats;` next to the other `pub mod` declarations (line 1-7).

Temporarily call it from `get_system_stats` (`src-tauri/src/lib.rs:470-473`) just to print, without touching `SystemStats` yet:

```rust
#[tauri::command]
fn get_system_stats(state: State<'_, AppState>) -> stats::SystemStats {
    #[cfg(target_os = "windows")]
    eprintln!("[DEBUG gpu] {:?}", gpu_stats::read_gpu_percent());
    stats::collect(&state.stats)
}
```

- [ ] **Step 3 (SPIKE): Observe**

Run: `cargo tauri dev`, watch the terminal output while idling, then while playing a video or a game.

Expected outcomes:
- **GO:** values near 0-10% at idle, rising noticeably (30%+) during GPU-heavy work, roughly matching Task Manager's own "GPU" column for the same moment.
- **NO-GO:** consistently `None`, consistently `0.0` even under load, or wildly implausible numbers (e.g. always 100%, or negative after the `.min(100.0)` clamp meaning something upstream is wrong).

- [ ] **Step 4a (IF GO): Wire it into `SystemStats`**

In `src-tauri/src/stats.rs`, add `pub gpu_percent: Option<f32>,` to `SystemStats`, and in `collect()` add `let gpu_percent = crate::gpu_stats::read_gpu_percent();` plus the field in the struct literal.

Revert the debug `eprintln!` in `get_system_stats` back to just `stats::collect(&state.stats)`.

In `src/components/StatsView.tsx`, add `gpu_percent: number | null;` to the `SystemStats` interface, default it to `null` in `useSystemStats`, and add a conditional GPU row in `StatsFull` right after the CPU row, following the exact same pattern as the `hasBattery` conditional:

```tsx
      {s.gpu_percent !== null && (
        <StatRow label={t("gpu")}
          value={`${s.gpu_percent.toFixed(0)}%`}
          bar={s.gpu_percent / 100}
          color={colorForLoad(s.gpu_percent)}
        />
      )}
```

Add `"gpu": "GPU",` to both locale files.

- [ ] **Step 4b (IF NO-GO): Clean up and stop**

Delete `src-tauri/src/gpu_stats.rs`, remove `pub mod gpu_stats;`, and revert `get_system_stats` to its original body. Report the observed failure mode.

- [ ] **Step 5: Verify (GO path only)**

Run: `npm run build && cargo tauri build`
Expected: both succeed, no `eprintln!` debug spam left in the release build.

- [ ] **Step 6: Commit**

GO path:
```bash
git add src-tauri/src/gpu_stats.rs src-tauri/src/lib.rs src-tauri/src/stats.rs src/components/StatsView.tsx src/locales/en.json src/locales/es.json
git commit -m "feat(stats): add GPU utilization via PDH counters"
```

NO-GO path:
```bash
git add -A
git commit -m "chore(gpu-stats): spike PDH GPU counters — no-go, revert"
```

---

## Self-review notes

- **Spec coverage:** network (Phase 1), disk/GPU (Phase 2 + Phase 6, split because their risk profiles are wildly different), the Caelestia-inspired visual redesign of Performance/Media/Weather (Phase 3, grounded directly in the 4 reference screenshots rather than guessed), Material Color Engine (Phase 4), and the liquid-glass Windows-native refactor (Phase 5). Quick Toggles was explicitly descoped per user direction — opening Windows' native Quick Settings from inside the island is more steps than the user just pressing Win+A themselves, so it isn't built at all (not even as the simplified shortcut version floated earlier).
- **Placeholder scan:** no TBD/"add error handling"/"similar to Task N" left — every step has literal code or an exact command plus exact expected output.
- **Type consistency:** `SystemStats` (Rust) and its TS mirror gain the same field groups (`disk_*` in Phase 2, `gpu_percent` in Phase 6) in the same phases they're introduced in. `WeatherInfo` gains `humidity`/`feels_like_c`/`wind_kmph` on both the Rust struct and the TS interface together in Phase 3 Task 3.4, verified against a live `wttr.in` response rather than assumed. `AccentColor`'s `{r,g,b}` shape matches what `Island.tsx`'s `.then(accent => ...)` destructures. Phase 3's new `weather` `Mode` variant is added everywhere the type is used (`Mode` union, `DIMS`, `CYCLE`, the mode-switch JSX) in the same task, with a step telling the implementer to grep for any other exhaustive `Mode` consumers instead of assuming there are none.
- **Known open question flagged, not guessed:** the `windows` crate's `IDesktopWallpaper` COM interface was deliberately avoided in Phase 4 rather than assumed to exist under an unverified feature flag — documented as a known limitation (single global accent, not per-monitor) instead of silently shipping something unverified.
- **Sizing risk called out:** Phase 3 grows `stats` and adds a new `weather` entry to `DIMS` in `Island.tsx`. The cursor-activation-zone fix from the previous session (`Island.tsx`, edge-detection effect) only ever reads `IDLE_DIMS[settings.clockSize]`, never `DIMS`, so growing non-idle mode dimensions here does not reintroduce that bug — confirmed by re-reading that code before writing this phase, not assumed.
