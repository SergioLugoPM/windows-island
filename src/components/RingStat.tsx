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
