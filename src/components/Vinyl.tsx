interface Props {
  isPlaying: boolean;
  size?: number;
  label?: string;
}

export function Vinyl({ isPlaying, size = 62, label = "♪" }: Props) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        flexShrink: 0,
        position: "relative",
        animation: isPlaying ? "vinyl-spin 2.8s linear infinite" : "none",
        animationPlayState: isPlaying ? "running" : "paused",
      }}
    >
      {/* Grooves */}
      <div style={{
        position: "absolute",
        inset: 0,
        borderRadius: "50%",
        background: `
          repeating-radial-gradient(circle at 50% 50%,
            rgba(8,8,18,0.97)   0px,
            rgba(18,18,32,0.93) 1.2px,
            rgba(10,10,22,0.96) 2.4px,
            rgba(22,22,38,0.9)  3.6px,
            rgba(8,8,18,0.97)   4.8px
          )
        `,
      }} />

      {/* Reflective sheen */}
      <div style={{
        position: "absolute",
        inset: 0,
        borderRadius: "50%",
        background: "conic-gradient(from 200deg, rgba(80,120,255,0.07) 0deg, transparent 60deg, rgba(60,90,200,0.04) 180deg, transparent 240deg, rgba(80,120,255,0.06) 360deg)",
        mixBlendMode: "screen",
      }} />

      {/* Center label */}
      <div style={{
        position: "absolute",
        top: "50%", left: "50%",
        transform: "translate(-50%,-50%)",
        width: "34%", height: "34%",
        borderRadius: "50%",
        background: "radial-gradient(circle at 40% 35%, #2e1a52, #110820)",
        boxShadow: "0 0 8px rgba(80,40,140,0.6), inset 0 1px 0 rgba(120,80,200,0.2)",
        display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        <span style={{ fontSize: size * 0.1, color: "rgba(160,120,255,0.7)", lineHeight: 1 }}>
          {label}
        </span>
      </div>

      {/* Center spindle hole */}
      <div style={{
        position: "absolute",
        top: "50%", left: "50%",
        transform: "translate(-50%,-50%)",
        width: "6%", height: "6%",
        borderRadius: "50%",
        background: "rgba(4,4,10,0.95)",
        boxShadow: "0 0 3px rgba(0,0,0,0.8)",
      }} />

      {/* Outer rim */}
      <div style={{
        position: "absolute",
        inset: 0,
        borderRadius: "50%",
        boxShadow: "inset 0 0 4px rgba(0,0,0,0.8), 0 2px 12px rgba(0,0,0,0.7), 0 0 8px rgba(60,80,180,0.15)",
      }} />
    </div>
  );
}
