import { useState, useEffect } from "react";

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
}

/** Poll system stats every 1.5s. Single source of truth for both Full and Compact views. */
export function useSystemStats() {
  const [stats, setStats] = useState<SystemStats>({
    cpu_percent: 0, ram_percent: 0, ram_used_mb: 0, ram_total_mb: 0,
    net_down_kbps: 0, net_up_kbps: 0,
    battery_percent: -1, battery_charging: false, cpu_temp_c: null,
  });

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const r = await invoke<SystemStats>("get_system_stats");
        if (alive) setStats(r);
      } catch { /* browser preview — keep defaults */ }
    };
    tick();
    const id = setInterval(tick, 1500);
    return () => { alive = false; clearInterval(id); };
  }, []);

  return stats;
}

// ─── Visual sub-components ───────────────────────────────────────────────

/** Horizontal capsule bar — 0..1 fill */
function StatBar({ value, color }: { value: number; color: string }) {
  return (
    <div style={{
      width: "100%", height: 4, borderRadius: 2,
      background: "rgba(80,100,140,0.25)", overflow: "hidden",
    }}>
      <div style={{
        width: `${Math.max(0, Math.min(100, value * 100))}%`,
        height: "100%", background: color,
        boxShadow: `0 0 6px ${color}`,
        transition: "width 0.4s ease",
      }} />
    </div>
  );
}

function colorForLoad(p: number): string {
  // 0..50 → blue, 50..80 → yellow, 80..100 → red
  if (p < 50) return "rgba(100,180,255,0.85)";
  if (p < 80) return "rgba(255,200,90,0.85)";
  return "rgba(255,110,110,0.95)";
}

// ─── Full view — used as a dedicated cycle mode ──────────────────────────

export function StatsFull() {
  const s = useSystemStats();
  const hasBattery = s.battery_percent >= 0;

  return (
    <div style={{ width: "100%", display: "flex", flexDirection: "column", gap: 7 }}>
      <StatRow label="CPU"
        value={`${s.cpu_percent.toFixed(0)}%`}
        sub={s.cpu_temp_c !== null ? `${s.cpu_temp_c.toFixed(0)}°C` : undefined}
        bar={s.cpu_percent / 100}
        color={colorForLoad(s.cpu_percent)}
      />
      <StatRow label="RAM"
        value={`${s.ram_percent.toFixed(0)}%`}
        sub={`${(s.ram_used_mb / 1024).toFixed(1)} / ${(s.ram_total_mb / 1024).toFixed(1)} GB`}
        bar={s.ram_percent / 100}
        color={colorForLoad(s.ram_percent)}
      />
      {hasBattery && (
        <StatRow label="BAT"
          value={`${s.battery_percent}%`}
          sub={s.battery_charging ? "⚡ cargando" : undefined}
          bar={s.battery_percent / 100}
          color={
            s.battery_percent < 20 ? "rgba(255,110,110,0.95)" :
            s.battery_percent < 50 ? "rgba(255,200,90,0.85)" :
            "rgba(120,220,140,0.85)"
          }
        />
      )}
    </div>
  );
}

function StatRow({ label, value, sub, bar, color }: {
  label: string; value: string; sub?: string; bar: number; color: string;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <div style={{
        display: "flex", justifyContent: "space-between", alignItems: "baseline",
        fontFamily: "-apple-system,'SF Pro Text','Segoe UI',system-ui,sans-serif",
      }}>
        <span style={{
          fontSize: 10, fontWeight: 600, letterSpacing: 0.5,
          color: "rgba(140,170,220,0.7)",
        }}>{label}</span>
        <span style={{
          fontSize: 11, fontWeight: 600, color: "rgba(220,230,255,0.95)",
          fontVariantNumeric: "tabular-nums",
        }}>
          {value}
          {sub && <span style={{ fontSize: 9, fontWeight: 400, color: "rgba(140,170,220,0.6)", marginLeft: 6 }}>{sub}</span>}
        </span>
      </div>
      <StatBar value={bar} color={color} />
    </div>
  );
}

// ─── Mini view — used inside `peek` mode next to the clock ───────────────

export function StatsMini() {
  const s = useSystemStats();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 3, minWidth: 0 }}>
      <MiniLine label="CPU" pct={s.cpu_percent} color={colorForLoad(s.cpu_percent)} />
      <MiniLine label="RAM" pct={s.ram_percent} color={colorForLoad(s.ram_percent)} />
    </div>
  );
}

function MiniLine({ label, pct, color }: { label: string; pct: number; color: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <span style={{
        fontSize: 9, fontWeight: 600, color: "rgba(140,170,220,0.65)",
        letterSpacing: 0.4, width: 22,
      }}>{label}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <StatBar value={pct / 100} color={color} />
      </div>
      <span style={{
        fontSize: 9, fontWeight: 600, color: "rgba(220,230,255,0.9)",
        fontVariantNumeric: "tabular-nums", width: 26, textAlign: "right",
      }}>{pct.toFixed(0)}%</span>
    </div>
  );
}
