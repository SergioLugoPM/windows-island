import { useState, useEffect, useCallback } from "react";
import { Vinyl } from "./Vinyl";
import { AudioVisualizer } from "./AudioVisualizer";
import { useAudioVisualizer } from "../hooks/useAudioVisualizer";

export interface MediaInfo {
  title: string;
  artist: string;
  is_playing: boolean;
  has_session: boolean;
}

async function invokeMedia(cmd: string): Promise<unknown> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke(cmd);
  } catch {
    return null;
  }
}

function useMediaInfo() {
  const [info, setInfo] = useState<MediaInfo>({
    title: "", artist: "", is_playing: false, has_session: false,
  });

  const refresh = useCallback(async () => {
    const r = await invokeMedia("get_media_info") as MediaInfo | null;
    if (r) setInfo(r);
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 3000);
    return () => clearInterval(id);
  }, [refresh]);

  const togglePlay = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("toggle_play_pause"); setTimeout(refresh, 400); };
  const skipNext   = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("skip_next");         setTimeout(refresh, 400); };
  const skipPrev   = async (e: React.MouseEvent) => { e.stopPropagation(); await invokeMedia("skip_previous");     setTimeout(refresh, 400); };

  return { info, togglePlay, skipNext, skipPrev };
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
  const { info, togglePlay, skipNext, skipPrev } = useMediaInfo();
  const audio = useAudioVisualizer(info.is_playing, 22);

  if (!info.has_session) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", width: "100%", height: "100%" }}>
        <span className="empty-label">Sin reproducción activa</span>
      </div>
    );
  }

  return (
    <div className="media-vinyl-layout">
      <Vinyl isPlaying={info.is_playing} size={64} />

      <div className="media-vinyl-right">
        <div className="media-vinyl-meta">
          <div className="media-title">{info.title || "Desconocido"}</div>
          <div className="media-artist">{info.artist || "—"}</div>
        </div>

        <AudioVisualizer bars={audio.bars} bass={audio.bass} width={168} height={26} />

        <div className="media-vinyl-controls">
          <button className="media-btn" onClick={skipPrev}>⏮</button>
          <button className="media-btn play" onClick={togglePlay} style={{ fontSize: 17 }}>
            {info.is_playing ? "⏸" : "▶"}
          </button>
          <button className="media-btn" onClick={skipNext}>⏭</button>
          <div className="media-progress" style={{ flex: 1, marginLeft: 6, marginBottom: 0 }}>
            <div className="media-progress-fill"
              style={{ animation: info.is_playing ? undefined : "none", width: info.is_playing ? undefined : "35%" }}
            />
          </div>
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
