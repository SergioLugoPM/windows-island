import { useRef, useEffect } from "react";

interface Props {
  bars: number[];
  bass?: number;
  width?: number;
  height?: number;
  color?: [number, number, number];
}

export function AudioVisualizer({
  bars,
  bass = 0,
  width = 160,
  height = 28,
  color = [90, 140, 255],
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width  = width  * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
  }, [width, height]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    const n = bars.length;
    const gap = 2;
    const bw = (width - gap * (n - 1)) / n;
    const [r, g, b] = color;
    const bassBright = 1 + bass * 0.6; // bass boosts brightness

    bars.forEach((level, i) => {
      const bh = Math.max(2, level * height);
      const x  = i * (bw + gap);
      const y  = height - bh;

      const grad = ctx.createLinearGradient(x, y, x, height);
      grad.addColorStop(0, `rgba(${Math.min(255, r * bassBright)}, ${Math.min(255, g * bassBright)}, 255, ${0.85 * level + 0.1})`);
      grad.addColorStop(1, `rgba(${r * 0.5}, ${g * 0.5}, ${b * 0.8}, ${0.4 * level})`);

      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, Math.min(bw / 2, 2));
      ctx.fill();

      // Glow on tall bars
      if (level > 0.5) {
        ctx.shadowColor = `rgba(${r}, ${g}, 255, ${(level - 0.5) * 0.6})`;
        ctx.shadowBlur = 4;
        ctx.fill();
        ctx.shadowBlur = 0;
      }
    });
  }, [bars, bass, width, height, color]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width, height, display: "block", imageRendering: "auto" }}
    />
  );
}
