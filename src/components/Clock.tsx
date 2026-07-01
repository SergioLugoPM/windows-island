import { useState, useEffect } from "react";

export type ClockFormat = "24h" | "12h";

interface ClockData {
  time: string;
  ampm: string;
  date: string;
  day: string;
}

function getNow(format: ClockFormat): ClockData {
  const now = new Date();
  const use12 = format === "12h";

  const time = now.toLocaleTimeString("es", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: use12,
  });

  // Separate AM/PM from time string for 12h mode
  const ampm = use12
    ? now.getHours() < 12 ? "AM" : "PM"
    : "";

  // For 12h, strip the locale-appended suffix so we control display
  const cleanTime = use12
    ? time.replace(/\s*(a\.?\s*m\.?|p\.?\s*m\.?)/i, "").trim()
    : time;

  const date = now.toLocaleDateString("es", { day: "numeric", month: "short" });
  const day = now.toLocaleDateString("es", { weekday: "long" });
  return { time: cleanTime, ampm, date, day };
}

interface Props {
  format?: ClockFormat;
  variant?: "idle" | "compact" | "expanded";
}

export function Clock({ format = "24h", variant = "expanded" }: Props) {
  const [data, setData] = useState<ClockData>(() => getNow(format));

  useEffect(() => {
    const id = setInterval(() => setData(getNow(format)), 1000);
    return () => clearInterval(id);
  }, [format]);

  if (variant === "compact") {
    return (
      <div className="clock-compact">
        <div className="clock-dot" />
        <span>{data.time}{data.ampm ? ` ${data.ampm}` : ""}</span>
      </div>
    );
  }

  if (variant === "idle") {
    return (
      <div className="clock-idle">
        <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
          <span className="clock-idle-time">{data.time}</span>
          {data.ampm && <span className="clock-idle-ampm">{data.ampm}</span>}
        </div>
        <span className="clock-idle-date">
          {data.day.slice(0, 3).toUpperCase()} · {data.date}
        </span>
      </div>
    );
  }

  // expanded
  return (
    <div className="clock-expanded">
      <div style={{ display: "flex", alignItems: "baseline", gap: 4 }}>
        <span className="clock-time-big">{data.time}</span>
        {data.ampm && (
          <span style={{ fontSize: 18, color: "rgba(140,165,255,0.65)", fontWeight: 500 }}>
            {data.ampm}
          </span>
        )}
      </div>
      <span className="clock-date">
        {data.day.charAt(0).toUpperCase() + data.day.slice(1)} · {data.date}
      </span>
    </div>
  );
}
