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
    <div className="ring-stat" style={{ minWidth: size }}>
      <div style={{ position: "relative", width: size, height: size }}>
        <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
          <circle className="ring-stat-track" cx={size / 2} cy={size / 2} r={r} fill="none" strokeWidth={stroke} />
          <circle cx={size / 2} cy={size / 2} r={r} fill="none"
            stroke={color} strokeWidth={stroke} strokeLinecap="round"
            strokeDasharray={circumference} strokeDashoffset={offset}
            style={{ transition: "stroke-dashoffset 0.4s ease" }} />
        </svg>
        <div className="ring-stat-value" style={{ fontSize: size * 0.24 }}>
          {clamped.toFixed(0)}%
        </div>
      </div>
      <div className="ring-stat-label">{label}</div>
      <div className="ring-stat-sub">{sub}</div>
    </div>
  );
}
