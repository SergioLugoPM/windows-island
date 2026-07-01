import { useState, useEffect, useCallback } from "react";
import { Vinyl } from "./Vinyl";
import { AudioVisualizer } from "./AudioVisualizer";
import { useAudioVisualizer } from "../hooks/useAudioVisualizer";
import { useI18n } from "../hooks/useI18n";

export interface MediaInfo {
  title: string;
  artist: string;
  is_playing: boolean;
  has_session: boolean;
  position_s: number;
  duration_s: number;
}

async function invokeMedia(cmd: string): Promise<unknown> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke(cmd);
  } catch {
    return null;
  }
}

/**
 * useMediaInfo
 *  - Polls the Rust backend every 3s for media state.
 *  - Returns `progress` (0..1) interpolated client-side between polls so the
 *    progress bar advances smoothly at 30 fps without 30 round-trips per second.
 */
function useMediaInfo() {
  const [info, setInfo] = useState<MediaInfo>({
    title: "", artist: "", is_playing: false, has_session: false,
    position_s: 0, duration_s: 0,
  });
  // pollAt = monotonic timestamp (ms) when the current `info` was received.
  // We interpolate position forward from pollAt + position_s while is_playing.
  const [pollAt, setPollAt] = useState(() => performance.now());
  const [now, setNow] = useState(() => performance.now());

  const refresh = useCallback(async () => {
    const r = await invokeMedia("get_media_info") as MediaInfo | null;
    if (r) {
      setInfo(r);
      setPollAt(performance.now());
    }
  }, []);

  // Poll every 3s
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 3000);
    return () => clearInterval(id);
  }, [refresh]);

  // Tick `now` 4×/s only when playing — drives smooth progress bar interpolation
  useEffect(() => {
    if (!info.is_playing) return;
    const id = setInterval(() => setNow(performance.now()), 250);
    return () => clearInterval(id);
  }, [info.is_playing]);

  // Interpolated current position
  const elapsedSincePoll = info.is_playing ? (now - pollAt) / 1000 : 0;
  const liveSeconds = Math.min(
    info.position_s + elapsedSincePoll,
    info.duration_s > 0 ? info.duration_s : Number.MAX_SAFE_INTEGER,
  );
  const progress = info.duration_s > 0 ? liveSeconds / info.duration_s : 0;

  const togglePlay = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("toggle_play_pause"); setTimeout(refresh, 400); };
  const skipNext   = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("skip_next");         setTimeout(refresh, 400); };
  const skipPrev   = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("skip_previous");     setTimeout(refresh, 400); };

  return { info, progress, liveSeconds, togglePlay, skipNext, skipPrev };
}

/** Format seconds as M:SS / H:MM:SS */
function fmtTime(s: number): string {
  if (!isFinite(s) || s <= 0) return "0:00";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  return h > 0
    ? `${h}:${m.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`
    : `${m}:${sec.toString().padStart(2, "0")}`;
}

interface Props {
  variant?: "full" | "compact" | "mini";
}

// ── Mini: solo controles + título (para peek) ──────────────────────────────
export function MediaMini() {
  const { info, togglePlay, skipNext, skipPrev } = useMediaInfo();

  if (!info.has_session) return <span className="empty-label">Sin música</span>;

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
      <span style={{
        fontSize: 11, fontWeight: 600, color: "rgba(200,215,255,0.85)",
        whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis",
        maxWidth: 88, fontFamily: "-apple-system,'SF Pro Text','Segoe UI',system-ui,sans-serif",
        textShadow: "0 0 8px rgba(100,140,255,0.3)",
      }}>
        {info.title || "♪"}
      </span>
      <div style={{ display: "flex", gap: 2, flexShrink: 0 }}>
        <button className="media-btn" onClick={skipPrev} style={{ fontSize: 11, width: 20, height: 20 }}>⏮</button>
        <button className="media-btn play" onClick={togglePlay} style={{ fontSize: 14, width: 22, height: 22 }}>
          {info.is_playing ? "⏸" : "▶"}
        </button>
        <button className="media-btn" onClick={skipNext} style={{ fontSize: 11, width: 20, height: 20 }}>⏭</button>
      </div>
    </div>
  );
}

// ── Full: vinyl + visualizador + controles ────────────────────────────────
export function MediaFull() {
  const { info, progress, liveSeconds, togglePlay, skipNext, skipPrev } = useMediaInfo();
  const audio = useAudioVisualizer(info.is_playing, 22);
  const { t } = useI18n();

  const hasDuration = info.duration_s > 0;
  const widthPct = `${Math.min(100, Math.max(0, progress * 100))}%`;

  return (
    <div className="media-vinyl-layout">
      <Vinyl isPlaying={info.is_playing} idleSpin={!info.has_session} size={64} />

      <div className="media-vinyl-right">
        <div className="media-vinyl-meta">
          {info.has_session ? (
            <>
              <div className="media-title">{info.title || "Desconocido"}</div>
              <div className="media-artist">{info.artist || "—"}</div>
            </>
          ) : (
            <>
              <div className="media-title">{t("noMedia")}</div>
              <div className="media-artist">{t("noMediaHint")}</div>
            </>
          )}
        </div>

        {info.has_session && (
          <AudioVisualizer bars={audio.bars} bass={audio.bass} width={168} height={26} />
        )}

        <div className="media-vinyl-controls">
          <button className="media-btn" onClick={skipPrev} disabled={!info.has_session}>⏮</button>
          <button className="media-btn play" onClick={togglePlay} disabled={!info.has_session} style={{ fontSize: 17 }}>
            {info.is_playing ? "⏸" : "▶"}
          </button>
          <button className="media-btn" onClick={skipNext} disabled={!info.has_session}>⏭</button>

          {info.has_session && hasDuration && (
            <div className="media-progress" style={{ flex: 1, marginLeft: 6, marginBottom: 0, position: "relative" }}>
              <div className="media-progress-fill"
                style={{ width: widthPct, animation: "none", transition: "width 0.25s linear" }}
              />
              <div style={{
                position: "absolute", right: -38, top: -6, fontSize: 8,
                color: "rgba(160,180,220,0.7)", fontVariantNumeric: "tabular-nums",
                whiteSpace: "nowrap",
              }}>
                {fmtTime(liveSeconds)} / {fmtTime(info.duration_s)}
              </div>
            </div>
          )}
          {info.has_session && !hasDuration && (
            <div className="media-progress" style={{ flex: 1, marginLeft: 6, marginBottom: 0 }}>
              <div className="media-progress-fill"
                style={{ animation: info.is_playing ? undefined : "none", width: info.is_playing ? undefined : "35%" }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Compact: arte + título + play (para modo full de la isla) ─────────────
export function MediaCompact() {
  const { info, togglePlay } = useMediaInfo();
  const audio = useAudioVisualizer(info.is_playing, 16);

  if (!info.has_session) {
    return <span className="empty-label">Sin reproducción activa</span>;
  }

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10, width: "100%" }}>
      <Vinyl isPlaying={info.is_playing} size={36} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div className="media-title" style={{ fontSize: 11 }}>{info.title || "Desconocido"}</div>
        <AudioVisualizer bars={audio.bars} bass={audio.bass} width={110} height={14} />
      </div>
      <button className="media-btn play" onClick={togglePlay}>{info.is_playing ? "⏸" : "▶"}</button>
    </div>
  );
}

// ── Default export backward-compat ───────────────────────────────────────
export function MediaView({ variant = "full" }: Props) {
  if (variant === "mini")    return <MediaMini />;
  if (variant === "compact") return <MediaCompact />;
  return <MediaFull />;
}
