import { useEffect, useRef } from "react";

interface Props {
  intensity?: number; // 0–1, default 0.4
  onClick?: () => void;
}

export function LiquidBackground({ intensity = 0.4, onClick }: Props) {
  const turb = useRef<SVGFETurbulenceElement>(null);
  const disp = useRef<SVGFEDisplacementMapElement>(null);
  const animFrame = useRef<number>(0);
  const phase = useRef(0);

  useEffect(() => {
    let running = true;

    const tick = () => {
      if (!running) return;
      phase.current += 0.003;

      if (turb.current) {
        const bx = 0.012 + Math.sin(phase.current * 0.7) * 0.006 * intensity;
        const by = 0.018 + Math.cos(phase.current * 0.5) * 0.007 * intensity;
        turb.current.setAttribute("baseFrequency", `${bx.toFixed(5)} ${by.toFixed(5)}`);
      }

      if (disp.current) {
        const scale = 3 + Math.sin(phase.current * 1.3) * 2 * intensity;
        disp.current.setAttribute("scale", scale.toFixed(2));
      }

      animFrame.current = requestAnimationFrame(tick);
    };

    animFrame.current = requestAnimationFrame(tick);
    return () => {
      running = false;
      cancelAnimationFrame(animFrame.current);
    };
  }, [intensity]);

  return (
    <>
      {/* SVG filter definition (invisible, 0-size) */}
      <svg
        style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}
        aria-hidden="true"
      >
        <defs>
          <filter id="liquid-distort" x="-5%" y="-5%" width="110%" height="110%">
            <feTurbulence
              ref={turb}
              type="turbulence"
              baseFrequency="0.012 0.018"
              numOctaves="4"
              seed="7"
              result="noise"
            />
            <feDisplacementMap
              ref={disp}
              in="SourceGraphic"
              in2="noise"
              scale="3"
              xChannelSelector="R"
              yChannelSelector="G"
            />
          </filter>

          {/* Separate, stronger filter for the click burst */}
          <filter id="liquid-burst" x="-10%" y="-10%" width="120%" height="120%">
            <feTurbulence
              type="turbulence"
              baseFrequency="0.025 0.03"
              numOctaves="3"
              seed="12"
              result="noise"
            >
              <animate
                attributeName="baseFrequency"
                values="0.025 0.03;0.08 0.09;0.025 0.03"
                dur="0.6s"
                begin="indefinite"
                id="burst-anim"
              />
            </feTurbulence>
            <feDisplacementMap
              in="SourceGraphic"
              in2="noise"
              scale="0"
              xChannelSelector="R"
              yChannelSelector="G"
            >
              <animate
                attributeName="scale"
                values="0;12;0"
                dur="0.6s"
                begin="burst-anim.begin"
              />
            </feDisplacementMap>
          </filter>
        </defs>
      </svg>

      {/* Glass face */}
      <div className="island-glass" onClick={onClick}>
        <div className="island-top-sheen" />
      </div>
    </>
  );
}
