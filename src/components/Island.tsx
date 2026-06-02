import { useState, useEffect, useRef, useCallback } from "react";
import { motion, AnimatePresence, useAnimation } from "framer-motion";
import { Clock, type ClockFormat } from "./Clock";
import { MediaMini, MediaFull, MediaCompact } from "./MediaView";
import { StatsFull, StatsMini } from "./StatsView";
import { WeatherView } from "./WeatherView";
import { AudioVisualizer } from "./AudioVisualizer";
import { useAudioVisualizer } from "../hooks/useAudioVisualizer";
import { LiquidBackground } from "./LiquidBackground";
import { LiquidGlassChrome } from "./LiquidGlassChrome";
import { isTauri } from "../App";
import { invoke } from '@tauri-apps/api/core';

// ─── Types ────────────────────────────────────────────────────────────────────

type Mode = "idle" | "peek" | "media" | "stats" | "full" | "settings";

/** "top" = snapped to top edge (cursor must reach screen top to activate).
 *  "floating" = free drag, always reacts to hover. */
export type PositionMode = "top" | "floating";

type SnapEdge = "top" | "bottom" | "left" | "right";

function edgeRadius(r: number, edge: SnapEdge): string {
  switch (edge) {
    case "top":    return `0px 0px ${r}px ${r}px`;
    case "bottom": return `${r}px ${r}px 0px 0px`;
    case "left":   return `0px ${r}px ${r}px 0px`;
    case "right":  return `${r}px 0px 0px ${r}px`;
  }
}

export type PeekContent = "weather" | "media" | "stats";
export type Theme       = "dark" | "light" | "glass";
export type ClockSize   = "S" | "M" | "L";

interface Settings {
  clockFormat:  ClockFormat;
  positionMode: PositionMode;
  peekContent:  PeekContent;
  theme:        Theme;
  clockSize:    ClockSize;
}

function defaultSettings(): Settings {
  return {
    clockFormat:  "24h",
    positionMode: "top",
    peekContent:  "weather",
    theme:        "dark",
    clockSize:    "M",
  };
}

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem("island-settings");
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Settings>;
      // Migrate removed "bottom" mode to "top"
      if ((parsed.positionMode as string) === "bottom") parsed.positionMode = "top";
      return { ...defaultSettings(), ...parsed };
    }
  } catch { /* ignore */ }
  return defaultSettings();
}

function saveSettings(s: Settings) {
  localStorage.setItem("island-settings", JSON.stringify(s));
}

// ─── Dimensions ───────────────────────────────────────────────────────────────

// Idle pill varies by clock size
const IDLE_DIMS: Record<ClockSize, { w: number; h: number; r: number }> = {
  S: { w: 140, h: 52, r: 26 },
  M: { w: 160, h: 64, r: 32 },
  L: { w: 184, h: 80, r: 40 },
};

// Other modes are fixed
const DIMS: Record<Mode, { w: number; h: number; r: number }> = {
  idle:     { w: 160, h: 64,  r: 32 },
  peek:     { w: 310, h: 68,  r: 34 },
  media:    { w: 350, h: 122, r: 28 },
  stats:    { w: 280, h: 110, r: 24 },
  full:     { w: 370, h: 158, r: 30 },
  settings: { w: 310, h: 232, r: 28 },
};

function getModeDims(mode: Mode, clockSize: ClockSize) {
  return mode === "idle" ? IDLE_DIMS[clockSize] : DIMS[mode];
}

const CYCLE: Mode[] = ["peek", "media", "stats", "full"];
const spring     = { type: "spring" as const, stiffness: 480, damping: 36, mass: 0.75 };
const springFast = { type: "spring" as const, stiffness: 600, damping: 38, mass: 0.6 };

// ─── Tauri helpers ────────────────────────────────────────────────────────────

// +MARGIN prevents sub-pixel clipping at window edges
const MARGIN = 4;

async function resizeToMode(
  mode: Mode,
  clockSize: ClockSize = "M",
  _posMode: PositionMode = "top", // kept for call-site compat; bottom removed
) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const d = getModeDims(mode, clockSize);
    await invoke("resize_window", { w: d.w + MARGIN, h: d.h + MARGIN });
  } catch { /* browser preview */ }
}

async function startDrag() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch { /* browser preview */ }
}

async function snapToEdge(
  positionMode: PositionMode,
  mode: Mode,
  clockSize: ClockSize = "M",
): Promise<SnapEdge | null> {
  if (positionMode === "floating") return null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const d = getModeDims(mode, clockSize);
    await invoke("snap_to_edge", { edge: "top", w: d.w + MARGIN, h: d.h + MARGIN });
    return "top";
  } catch { return null; }
}

// ─── Component ────────────────────────────────────────────────────────────────

export function Island() {
  const [mode, setMode]           = useState<Mode>("idle");
  const [opacity, setOpacity]     = useState(0.72);
  const [liqIntensity, setLiq]    = useState(0.4);
  const [settings, setSettings]   = useState<Settings>(loadSettings);
  const [isPlaying, setIsPlaying] = useState(false);
  const [snapEdge, setSnapEdge]   = useState<SnapEdge>("top");

  // Theme injection state
  const [injectionActive, setInjectionActive] = useState(false);
  const [selectedTheme, setSelectedTheme] = useState<'dark' | 'light' | 'vidrio'>('dark');
  const [injectionLoading, setInjectionLoading] = useState(false);
  const [injectionError, setInjectionError]     = useState<string | null>(null);

  const idleAudio = useAudioVisualizer(isPlaying, 18);

  const pulseControls  = useAnimation();
  const hoverTimer     = useRef<ReturnType<typeof setTimeout>>();
  const collapseTimer  = useRef<ReturnType<typeof setTimeout>>();
  const burstTimer     = useRef<ReturnType<typeof setTimeout>>();
  const isDragging     = useRef(false);

  // Effective dims for the current mode + clock size
  const dims = getModeDims(mode, settings.clockSize);

  // ── Persist settings ──
  useEffect(() => { saveSettings(settings); }, [settings]);

  // ── Check initial injection state ──
  useEffect(() => {
    if (isTauri) {
      invoke<boolean>('is_injection_active')
        .then(active => setInjectionActive(active))
        .catch(err => console.error('Failed to check injection status:', err));
    }
  }, []);

  // ── Snap when positionMode changes ──
  useEffect(() => {
    if (settings.positionMode !== "floating") {
      snapToEdge(settings.positionMode, mode, settings.clockSize).then(e => {
        if (e) setSnapEdge(e);
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.positionMode]);

  // ── Resize idle window when clockSize changes (hot-reload dims) ──
  useEffect(() => {
    if (mode === "idle") resizeToMode("idle", settings.clockSize, settings.positionMode);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.clockSize]);

  // Mica disabled: DWM fills the entire HWND rectangle (including transparent
  // pill corners), making the window border visually apparent. CSS-only glass
  // uses a semi-transparent background instead — no corner artifacts.

  // ── Poll media playing state ──
  useEffect(() => {
    const poll = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const info = await invoke<{ is_playing: boolean }>("get_media_info");
        setIsPlaying(info?.is_playing ?? false);
      } catch { setIsPlaying(true); }
    };
    poll();
    const id = setInterval(poll, 4000);
    return () => clearInterval(id);
  }, []);

  // ── Schedule collapse ──
  const scheduleCollapse = useCallback((ms = 3000) => {
    clearTimeout(collapseTimer.current);
    collapseTimer.current = setTimeout(() => {
      resizeToMode("idle", settings.clockSize, settings.positionMode);
      setMode("idle");
      setOpacity(0.72);
      setLiq(0.4);
      // Passthrough re-engages via the useEffect that watches mode
    }, ms);
  }, [settings.clockSize, settings.positionMode]);

  const cancelCollapse = useCallback(() => clearTimeout(collapseTimer.current), []);

  // ── Cursor passthrough + edge polling ──────────────────────────────────────
  // When idle + edge mode: window is fully transparent to mouse events.
  // A 60ms poll (GetCursorPos via Rust) detects when cursor reaches the
  // screen-edge side; then re-enables events and expands the island.
  // Uses set_cursor_passthrough (Rust command) — more reliable than the JS API.
  useEffect(() => {
    if (!isTauri) return;

    const isEdgeIdle = mode === "idle" && settings.positionMode !== "floating";

    const coreMod = import("@tauri-apps/api/core");

    if (!isEdgeIdle) {
      // Ensure cursor events are on whenever not in passthrough state
      coreMod.then(m => m.invoke("set_cursor_passthrough", { enabled: false })).catch(() => {});
      return;
    }

    let cancelled   = false;
    let passthrough = false;
    let expanding   = false;
    let infoReady = false;
    let scale     = 1;

    import("@tauri-apps/api/window").then(m => m.currentMonitor()).then(mon => {
      if (mon) scale = mon.scaleFactor;
      infoReady = true;
    }).catch(() => { infoReady = true; });

    const setPass = async (on: boolean) => {
      if (on === passthrough) return;
      passthrough = on;
      const core = await coreMod;
      await core.invoke("set_cursor_passthrough", { enabled: on }).catch(() => {});
    };

    // Enable passthrough immediately
    setPass(true);

    const id = setInterval(async () => {
      if (cancelled || expanding || !infoReady) return;
      try {
        const core = await coreMod;
        const [, cy] = await core.invoke<[number, number]>("get_cursor_screen_pos");

        // Trigger when cursor is in the topmost 8 physical pixels of the screen
        const TOP_PX = Math.round(8 * scale);
        const atEdge = cy < TOP_PX;

        if (atEdge && passthrough) {
          expanding = true;
          await setPass(false);
          setTimeout(async () => {
            if (cancelled) return;
            setOpacity(1);
            setLiq(0.65);
            await resizeToMode("peek", settings.clockSize, settings.positionMode);
            setMode("peek");
            // Safety net: if mouseenter/mouseleave never fire (cursor moved away
            // before the island appeared, or GetCursorPos false-positive),
            // collapse automatically after 4 s so the island doesn't stay stuck.
            clearTimeout(collapseTimer.current);
            collapseTimer.current = setTimeout(() => {
              resizeToMode("idle", settings.clockSize, settings.positionMode);
              setMode("idle");
              setOpacity(0.72);
              setLiq(0.4);
            }, 4000);
          }, 80);

        } else if (!atEdge && !passthrough && !expanding) {
          await setPass(true);
          setOpacity(0.72);
          setLiq(0.4);
        }
      } catch { /* browser preview */ }
    }, 60);

    return () => {
      cancelled = true;
      clearInterval(id);
      coreMod.then(m => m.invoke("set_cursor_passthrough", { enabled: false })).catch(() => {});
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, settings.positionMode, settings.clockSize]);

  // ── Mouse enter ──
  // In edge+idle: the polling already handles expansion — nothing to do here.
  // In floating mode or expanded modes: activate immediately.
  const handleMouseEnter = useCallback(() => {
    cancelCollapse();
    // Edge + idle: handled entirely by the passthrough/polling effect above
    if (mode === "idle" && settings.positionMode !== "floating") return;

    clearTimeout(hoverTimer.current);
    setOpacity(1);
    setLiq(0.65);
    if (mode === "idle") {
      hoverTimer.current = setTimeout(async () => {
        await resizeToMode("peek", settings.clockSize, settings.positionMode);
        setMode("peek");
      }, 180);
    }
  }, [mode, settings.positionMode, settings.clockSize, cancelCollapse]);

  const handleMouseLeave = useCallback(() => {
    clearTimeout(hoverTimer.current);
    setOpacity(0.72);
    setLiq(0.4);
    // Settings mode: the panel resizes on open, which fires a spurious mouseleave
    // that would override the 8 s timer set by the long-press handler.
    // Don't shorten it — let the existing 8 s (or longer) timer run.
    if (mode === "settings") return;
    scheduleCollapse(2800);
  }, [scheduleCollapse, mode]);

  // ── Click: cycle modes ──
  const handleClick = useCallback(async () => {
    if (isDragging.current) return;
    cancelCollapse();

    pulseControls.start({
      opacity: [0.7, 0],
      scale:   [1, 1.08],
      transition: { duration: 0.45, ease: "easeOut" },
    });

    setLiq(1);
    clearTimeout(burstTimer.current);
    burstTimer.current = setTimeout(() => setLiq(0.65), 550);

    setMode((prev) => {
      const nextMode = (() => {
        if (prev === "settings") return "idle" as Mode;
        const idx = CYCLE.indexOf(prev as Exclude<Mode, "idle" | "settings">);
        return CYCLE[(idx + 1) % CYCLE.length];
      })();
      resizeToMode(nextMode, settings.clockSize, settings.positionMode);
      return nextMode;
    });

    // Collapse triggered by handleMouseLeave only — never while cursor is over island.
  }, [cancelCollapse, pulseControls, settings.clockSize, settings.positionMode]);

  // ── Long press → settings ──
  const longPressTimer = useRef<ReturnType<typeof setTimeout>>();

  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const tag = (e.target as HTMLElement).closest("button, input, select, a");
    if (tag) return;

    isDragging.current = false;
    const origin = { x: e.clientX, y: e.clientY };

    const onMove = (mv: MouseEvent) => {
      if (Math.abs(mv.clientX - origin.x) > 5 || Math.abs(mv.clientY - origin.y) > 5) {
        isDragging.current = true;
        clearTimeout(longPressTimer.current);
      }
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", () => document.removeEventListener("mousemove", onMove), { once: true });

    longPressTimer.current = setTimeout(async () => {
      if (!isDragging.current) {
        cancelCollapse();
        await resizeToMode("settings", settings.clockSize, settings.positionMode);
        setMode("settings");
        scheduleCollapse(8000);
      }
    }, 600);

    await startDrag();
  }, [cancelCollapse, scheduleCollapse, settings.clockSize]);

  const handleMouseUp = useCallback(async () => {
    clearTimeout(longPressTimer.current);
    if (isDragging.current) {
      if (settings.positionMode !== "floating") {
        const edge = await snapToEdge(settings.positionMode, mode, settings.clockSize);
        if (edge) setSnapEdge(edge);
      }
      isDragging.current = false;
    }
  }, [settings.positionMode, settings.clockSize, mode]);

  // ── Keyboard ──
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        resizeToMode("idle", settings.clockSize, settings.positionMode);
        setMode("idle");
        cancelCollapse();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cancelCollapse, settings.clockSize, settings.positionMode]);

  // ── Settings updaters ──
  const setClockFormat  = (v: ClockFormat)  => setSettings(s => ({ ...s, clockFormat: v }));
  const setPositionMode = (v: PositionMode) => setSettings(s => ({ ...s, positionMode: v }));
  const setPeekContent  = (v: PeekContent)  => setSettings(s => ({ ...s, peekContent: v }));
  const setTheme        = (v: Theme)        => setSettings(s => ({ ...s, theme: v }));
  const setClockSize    = (v: ClockSize)    => setSettings(s => ({ ...s, clockSize: v }));

  // ── Theme change handler (injection) ──
  // Centralizes theme selection so the injected DLL gets refreshed live.
  // The Rust `update_injected_theme` command only accepts "dark" | "light";
  // "vidrio" maps to the dark config for IPC purposes.
  const handleThemeChange = async (newTheme: 'dark' | 'light' | 'vidrio') => {
    setSelectedTheme(newTheme);

    if (injectionActive) {
      try {
        const configName = newTheme === 'vidrio' ? 'dark' : newTheme;
        // Update the theme in the IPC server
        await invoke('update_injected_theme', { configName });
        // Signal the DLL to refresh its cached config
        await invoke('refresh_injected_theme_config');
      } catch (error) {
        console.error('Failed to update theme:', error);
        alert('Failed to update theme');
      }
    }
  };

  // ── Theme injection handler ──
  const handleToggleInjection = async () => {
    if (!isTauri) return;
    setInjectionLoading(true);
    setInjectionError(null);
    try {
      if (!injectionActive) {
        await invoke('enable_theme_injection', { themeName: selectedTheme });
        setInjectionActive(true);
      } else {
        await invoke('disable_theme_injection');
        setInjectionActive(false);
      }
    } catch (error) {
      const msg = String(error);
      setInjectionError(msg);
      console.error('Injection toggle failed:', msg);
    } finally {
      setInjectionLoading(false);
    }
  };

  const cycleIndex = CYCLE.indexOf(mode as Exclude<Mode, "idle" | "settings">);

  const motionStyle = isTauri
    ? { width: "100%", height: "100%", cursor: "default" as const }
    : { cursor: "default" as const };

  const isEdgeMode = settings.positionMode !== "floating";
  const borderRadiusValue = (isTauri && isEdgeMode)
    ? edgeRadius(dims.r, snapEdge)
    : dims.r;

  const motionAnimate = isTauri
    ? { borderRadius: borderRadiusValue, opacity }
    : { width: dims.w, height: dims.h, borderRadius: dims.r, opacity };

  const island = (
      <motion.div
        className={`island-outer theme-${settings.theme}`}
        animate={motionAnimate}
        transition={spring}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onClick={handleClick}
        style={motionStyle}
      >
        {/* Boom rings — visible in any mode whenever media is playing.
            The inset box-shadow respects border-radius (incl. animated radius),
            so they work in all three themes. */}
        {isPlaying && (
          <>
            <div className="boom-ring" />
            <div className="boom-ring boom-ring-2" />
            <div className="boom-ring boom-ring-3" />
          </>
        )}

        <LiquidBackground intensity={liqIntensity} />
        {settings.theme === "glass" && <LiquidGlassChrome intensity={liqIntensity} />}

        {/* Click pulse ring */}
        <motion.div
          className="island-pulse"
          animate={pulseControls}
          style={{ borderRadius: dims.r + 3 }}
          initial={{ opacity: 0, scale: 1 }}
        />

        <div className="island-content">
          <AnimatePresence mode="wait">

            {/* ── IDLE ── */}
            {mode === "idle" && (
              <motion.div key="idle"
                className={`clock-size-${settings.clockSize.toLowerCase()}`}
                initial={{ opacity: 0, scale: 0.82 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.82 }}
                transition={springFast}
                style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 3 }}
              >
                <Clock variant="idle" format={settings.clockFormat} />
                {isPlaying && (
                  <AudioVisualizer
                    bars={idleAudio.bars}
                    bass={idleAudio.bass}
                    width={90}
                    height={10}
                    color={[80, 130, 255]}
                  />
                )}
              </motion.div>
            )}

            {/* ── PEEK ── */}
            {mode === "peek" && (
              <motion.div key="peek"
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                transition={springFast}
                style={{ display: "flex", alignItems: "center", gap: 14, width: "100%" }}
              >
                <Clock variant="expanded" format={settings.clockFormat} />
                <div style={{ width: 1, height: 28, background: "rgba(80,100,200,0.2)", flexShrink: 0 }} />
                {settings.peekContent === "weather" && <WeatherView compact />}
                {settings.peekContent === "media"   && <MediaMini />}
                {settings.peekContent === "stats"   && <StatsMini />}
              </motion.div>
            )}

            {/* ── MEDIA ── */}
            {mode === "media" && (
              <motion.div key="media"
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={springFast}
                style={{ width: "100%" }}
              >
                <MediaFull />
              </motion.div>
            )}

            {/* ── STATS ── */}
            {mode === "stats" && (
              <motion.div key="stats"
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={springFast}
                style={{ width: "100%" }}
              >
                <StatsFull />
              </motion.div>
            )}

            {/* ── FULL ── */}
            {mode === "full" && (
              <motion.div key="full"
                initial={{ opacity: 0, scale: 0.92 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                transition={springFast}
                style={{ width: "100%", display: "flex", flexDirection: "column", gap: 10 }}
              >
                <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
                  <Clock variant="expanded" format={settings.clockFormat} />
                  <div style={{ width: 1, height: 32, background: "rgba(80,100,200,0.2)", flexShrink: 0 }} />
                  <WeatherView compact />
                </div>
                <div style={{ height: 1, background: "linear-gradient(90deg,transparent,rgba(80,100,200,0.2),transparent)" }} />
                <MediaCompact />
              </motion.div>
            )}

            {/* ── SETTINGS ── */}
            {mode === "settings" && (
              <motion.div key="settings"
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.9 }}
                transition={springFast}
                style={{ width: "100%" }}
              >
                <div className="settings-panel">
                  <div className="settings-title">⚙ Configuración</div>

                  {/* Formato del reloj */}
                  <div className="settings-row">
                    <span className="settings-label">Reloj</span>
                    <div className="settings-toggle">
                      <button className={`settings-opt ${settings.clockFormat === "24h" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setClockFormat("24h"); }}>24h</button>
                      <button className={`settings-opt ${settings.clockFormat === "12h" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setClockFormat("12h"); }}>12h</button>
                    </div>
                  </div>

                  {/* Tamaño del reloj en reposo */}
                  <div className="settings-row">
                    <span className="settings-label">Tamaño</span>
                    <div className="settings-toggle">
                      <button className={`settings-opt ${settings.clockSize === "S" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setClockSize("S"); }}>S</button>
                      <button className={`settings-opt ${settings.clockSize === "M" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setClockSize("M"); }}>M</button>
                      <button className={`settings-opt ${settings.clockSize === "L" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setClockSize("L"); }}>L</button>
                    </div>
                  </div>

                  {/* Tema */}
                  <div className="settings-row">
                    <span className="settings-label">Tema</span>
                    <div className="settings-toggle">
                      <button className={`settings-opt ${settings.theme === "dark" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setTheme("dark"); }}>Oscuro</button>
                      <button className={`settings-opt ${settings.theme === "light" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setTheme("light"); }}>Claro</button>
                      <button className={`settings-opt ${settings.theme === "glass" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setTheme("glass"); }}>Vidrio</button>
                    </div>
                  </div>

                  {/* Contenido del hover */}
                  <div className="settings-row">
                    <span className="settings-label">Hover</span>
                    <div className="settings-toggle">
                      <button className={`settings-opt ${settings.peekContent === "weather" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setPeekContent("weather"); }}>Clima</button>
                      <button className={`settings-opt ${settings.peekContent === "media" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setPeekContent("media"); }}>Música</button>
                      <button className={`settings-opt ${settings.peekContent === "stats" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setPeekContent("stats"); }}>Sistema</button>
                    </div>
                  </div>

                  {/* Posición */}
                  <div className="settings-row">
                    <span className="settings-label">Posición</span>
                    <div className="settings-toggle">
                      <button className={`settings-opt ${settings.positionMode === "top" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setPositionMode("top"); }}>↑ Borde</button>
                      <button className={`settings-opt ${settings.positionMode === "floating" ? "active" : ""}`}
                        onClick={(e) => { e.stopPropagation(); setPositionMode("floating"); }}>Libre</button>
                    </div>
                  </div>

                  {/* Theme Injection */}
                  <div style={{ marginTop: 12, paddingTop: 12, borderTop: '1px solid rgba(80,100,200,0.2)' }}>
                    <div style={{ fontSize: 11, fontWeight: 'bold', color: 'rgba(180,190,220,0.9)', marginBottom: 8 }}>
                      🧪 Theme Injection (Experimental)
                    </div>

                    <div style={{ marginBottom: 8 }}>
                      <span style={{ fontSize: 10, color: 'rgba(120,140,180,0.8)', display: 'block', marginBottom: 4 }}>
                        Tema:
                      </span>
                      <div style={{ display: 'flex', gap: 6 }}>
                        {(['dark', 'light', 'vidrio'] as const).map(theme => (
                          <label key={theme} style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 9 }}>
                            <input
                              type="radio"
                              name="injectionTheme"
                              value={theme}
                              checked={selectedTheme === theme}
                              onChange={(e) => { e.stopPropagation(); handleThemeChange(theme); }}
                              disabled={injectionLoading}
                              style={{ transform: 'scale(0.8)' }}
                            />
                            <span style={{ textTransform: 'capitalize', color: 'rgba(160,170,200,0.9)' }}>
                              {theme === 'vidrio' ? 'Vidrio' : theme === 'light' ? 'Claro' : 'Oscuro'}
                            </span>
                          </label>
                        ))}
                      </div>
                    </div>

                    <button
                      onClick={(e) => { e.stopPropagation(); handleToggleInjection(); }}
                      disabled={injectionLoading}
                      style={{
                        padding: '6px 12px',
                        backgroundColor: injectionActive ? '#ff6b6b' : '#51cf66',
                        color: 'white',
                        border: 'none',
                        borderRadius: '3px',
                        cursor: injectionLoading ? 'not-allowed' : 'pointer',
                        opacity: injectionLoading ? 0.6 : 1,
                        fontSize: 10,
                        fontWeight: 'bold',
                        marginBottom: 6,
                        width: '100%'
                      }}
                    >
                      {injectionLoading ? 'Cargando...' : injectionActive ? 'Desactivar Injection' : 'Activar Injection'}
                    </button>

                    <div style={{ fontSize: 8, color: 'rgba(100,120,160,0.6)', textAlign: 'center' }}>
                      Estado: {injectionActive ? '✓ Activo' : '○ Inactivo'}
                    </div>

                    {injectionError && (
                      <div style={{
                        fontSize: 8,
                        color: '#ff6b6b',
                        textAlign: 'center',
                        marginTop: 4,
                        lineHeight: 1.3,
                        padding: '3px 4px',
                        background: 'rgba(255,60,60,0.08)',
                        borderRadius: 4,
                      }}>
                        ⚠ {injectionError}
                      </div>
                    )}
                  </div>

                  <div style={{ fontSize: 9, color: "rgba(100,120,180,0.4)", textAlign: "center", marginTop: 8 }}>
                    mantén presionado · Esc para cerrar
                  </div>
                </div>
              </motion.div>
            )}

          </AnimatePresence>
        </div>

        {/* Mode indicator dots */}
        <AnimatePresence>
          {mode !== "idle" && mode !== "settings" && (
            <motion.div className="mode-dots"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2 }}
            >
              {CYCLE.map((m, i) => (
                <div key={m} className={`mode-dot ${i === cycleIndex ? "active" : ""}`} />
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
  );

  // In Tauri the motion.div IS root — no wrapper to paint rectangular corners.
  // In browser preview the wrapper centers the island on the page.
  return isTauri
    ? island
    : <div style={{ position: "relative", display: "inline-flex", justifyContent: "center" }}>{island}</div>;
}
