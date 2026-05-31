import { useState, useEffect, useRef } from "react";

export interface AudioData {
  bars: number[];
  bass: number;
}

const EMPTY = (n: number): AudioData => ({ bars: new Array(n).fill(0), bass: 0 });

function makePhases(n: number) {
  return Array.from({ length: n }, (_, i) => i * 2.399); // golden-ratio spread
}

/**
 * useAudioVisualizer — parametric fake visualizer driven by a multi-octave sine bank.
 *
 * Notes:
 *  - We deliberately do NOT call getUserMedia(): in Tauri/WebView2 it prompts for
 *    microphone permission via the OS, which is bad UX for a clock widget that
 *    isn't actually capturing audio. Two instances of this hook on the same page
 *    would also race for the stream and one would silently fail.
 *  - The fake bars look "real enough" with golden-ratio phase spread + 3 harmonics.
 *  - When isPlaying flips off we smoothly decay to zero, then stop the rAF loop.
 */
export function useAudioVisualizer(isPlaying: boolean, barCount = 22) {
  const [data, setData] = useState<AudioData>(() => EMPTY(barCount));
  const frameRef = useRef<number>(0);
  const phases   = useRef(makePhases(barCount));

  useEffect(() => {
    cancelAnimationFrame(frameRef.current);

    if (isPlaying) {
      const tick = (t: number) => {
        const s = t / 1000;
        const bars = phases.current.map((p, i) => {
          const fm = 0.6 + (i / barCount) * 1.8;
          const a  = Math.sin(s * fm * 2.6 + p) * 0.38 + 0.38;
          const b  = Math.sin(s * fm * 5.3 + p * 1.9) * 0.16;
          const c  = Math.sin(s * fm * 9.7 + p * 3.4) * 0.07;
          return Math.max(0.04, Math.min(1, a + b + c));
        });
        const bass = Math.min(1, Math.max(0.05,
          Math.sin(s * 2.1) * 0.38 + 0.38 +
          Math.sin(s * 4.3) * 0.15 +
          Math.sin(s * 0.7) * 0.12
        ));
        setData({ bars, bass });
        frameRef.current = requestAnimationFrame(tick);
      };
      frameRef.current = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(frameRef.current);
    }

    // Not playing: smooth decay to zero, then stop
    let last = data;
    const decay = () => {
      const bars = last.bars.map(v => Math.max(0, v * 0.85));
      const bass = Math.max(0, last.bass * 0.85);
      last = { bars, bass };
      setData(last);
      if (last.bass > 0.01) {
        frameRef.current = requestAnimationFrame(decay);
      } else {
        setData(EMPTY(barCount));
      }
    };
    frameRef.current = requestAnimationFrame(decay);
    return () => cancelAnimationFrame(frameRef.current);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isPlaying, barCount]);

  return data;
}
