import { useState, useEffect } from "react";
import { useI18n } from "../hooks/useI18n";
import { RingStat } from "./RingStat";

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

/** Poll system stats every 1.5s. Single source of truth for both Full and Compact views. */
export function useSystemStats() {
  const [stats, setStats] = useState<SystemStats>({
    cpu_percent: 0, ram_percent: 0, ram_used_mb: 0, ram_total_mb: 0,
    net_down_kbps: 0, net_up_kbps: 0,
    battery_percent: -1, battery_charging: false, cpu_temp_c: null,
    disk_percent: 0, disk_used_gb: 0, disk_total_gb: 0,
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

function formatKbps(kbps: number): string {
  if (kbps < 1024) return `${kbps.toFixed(0)} KB/s`;
  return `${(kbps / 1024).toFixed(1)} MB/s`;
}

// ─── Full view — used as a dedicated cycle mode ──────────────────────────

export function StatsFull() {
  const s = useSystemStats();
  const { t } = useI18n();
  const hasBattery = s.battery_percent >= 0;
  const netHistory = useNetHistory(s.net_down_kbps);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="stat-card-row">
        <div className="stat-card">
          <div className="stat-card-header">{t("cpu")}</div>
          <StatBar value={s.cpu_percent / 100} color={colorForLoad(s.cpu_percent)} />
          <div className="stat-card-footer" style={{ display: "flex", justifyContent: "space-between", fontSize: 9 }}>
            <span>{s.cpu_percent.toFixed(0)}%</span>
            {s.cpu_temp_c !== null && <span>{s.cpu_temp_c.toFixed(0)}°C</span>}
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-card-header">{t("network")}</div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 6 }}>
            <Sparkline values={netHistory} color="rgba(120,200,140,0.85)" width={54} />
            <div style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 8, fontVariantNumeric: "tabular-nums", color: "rgba(220,230,255,0.9)" }}>
              <span>↓{formatKbps(s.net_down_kbps)}</span>
              <span>↑{formatKbps(s.net_up_kbps)}</span>
            </div>
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
          <div className="stat-card-footer" style={{ fontSize: 9 }}>
            {s.battery_percent}%{s.battery_charging ? ` · ${t("charging")}` : ""}
          </div>
        </div>
      )}
    </div>
  );
}

