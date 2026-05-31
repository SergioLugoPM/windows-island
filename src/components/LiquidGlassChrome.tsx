import { useEffect, useRef } from "react";

interface Props {
  intensity?: number; // 0–1
}

/**
 * Apple Liquid Glass — CSS approximation for Windows
 *
 * Layers (back → front):
 *  1. glass-chromatic    — chromatic aberration (blue-violet left, red-pink right)
 *  2. glass-border-distort — iridescent conic-gradient border distorted by SVG
 *  3. glass-refract-ring — static white definition ring
 *  4. glass-specular-main — large elliptical specular blob at top (Apple's signature)
 *  5. glass-specular-top — thin bright fresnel line at very top edge
 *  6. glass-caustics      — distorted caustic light blobs (SVG filter)
 *  7. glass-shimmer       — slow diagonal light sweep
 *  8. glass-inner-shadow  — depth / thickness illusion
 */
export function LiquidGlassChrome({ intensity = 0.5 }: Props) {
  const causticTurbRef = useRef<SVGFETurbulenceElement>(null);
  const borderTurbRef  = useRef<SVGFETurbulenceElement>(null);
  const animRef        = useRef<number>(0);
  const phase          = useRef(0);

  useEffect(() => {
    let running = true;

    const tick = () => {
      if (!running) return;
      phase.current += 0.0012;

      // Caustic blobs — medium frequency
      if (causticTurbRef.current) {
        const bx = 0.007 + Math.sin(phase.current * 0.55) * 0.004 * intensity;
        const by = 0.011 + Math.cos(phase.current * 0.38) * 0.005 * intensity;
        causticTurbRef.current.setAttribute(
          "baseFrequency", `${bx.toFixed(5)} ${by.toFixed(5)}`
        );
      }

      // Border distortion — low frequency, big slow waves
      if (borderTurbRef.current) {
        const bx = 0.003 + Math.sin(phase.current * 0.22) * 0.002 * intensity;
        const by = 0.005 + Math.cos(phase.current * 0.18) * 0.003 * intensity;
        borderTurbRef.current.setAttribute(
          "baseFrequency", `${bx.toFixed(5)} ${by.toFixed(5)}`
        );
      }

      animRef.current = requestAnimationFrame(tick);
    };

    animRef.current = requestAnimationFrame(tick);
    return () => { running = false; cancelAnimationFrame(animRef.current); };
  }, [intensity]);

  return (
    <div className="glass-chrome-wrapper">
      {/* ── SVG Filters ── */}
      <svg aria-hidden="true"
        style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}>
        <defs>
          {/* Caustic light blobs */}
          <filter id="glass-caustic" x="0%" y="0%" width="100%" height="100%">
            <feTurbulence ref={causticTurbRef} type="turbulence"
              baseFrequency="0.007 0.011" numOctaves="3" seed="15" result="noise" />
            <feDisplacementMap in="SourceGraphic" in2="noise"
              scale="14" xChannelSelector="R" yChannelSelector="G" />
          </filter>

          {/* Border liquid distortion — clipped to element bounds to avoid corner bleed */}
          <filter id="glass-border" x="0%" y="0%" width="100%" height="100%">
            <feTurbulence ref={borderTurbRef} type="turbulence"
              baseFrequency="0.003 0.005" numOctaves="4" seed="8" result="noise" />
            <feDisplacementMap in="SourceGraphic" in2="noise"
              scale="14" xChannelSelector="R" yChannelSelector="G" />
          </filter>
        </defs>
      </svg>

      {/* 1. Chromatic aberration — color fringing at edges like real glass optics */}
      <div className="glass-chromatic" />

      {/* 2. Iridescent conic border (distorted by SVG) — the main Apple signature */}
      <div className="glass-border-distort" style={{ filter: "url(#glass-border)" }} />

      {/* 3. Static white definition ring */}
      <div className="glass-refract-ring" />

      {/* 4. Large oval specular — Apple's prominent top highlight */}
      <div className="glass-specular-main" />

      {/* 5. Thin fresnel top-edge line */}
      <div className="glass-specular-top" />

      {/* 6. Caustic light patterns */}
      <div className="glass-caustics" style={{ filter: "url(#glass-caustic)" }} />

      {/* 7. Slow diagonal shimmer sweep */}
      <div className="glass-shimmer" />

      {/* 8. Inner shadow for depth */}
      <div className="glass-inner-shadow" />
    </div>
  );
}
