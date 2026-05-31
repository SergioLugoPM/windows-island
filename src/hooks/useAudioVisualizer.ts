import { useState, useEffect, useRef, useCallback } from "react";

export interface AudioData {
  bars: number[];
  bass: number;
}

const EMPTY = (n: number): AudioData => ({ bars: new Array(n).fill(0), bass: 0 });

function makePhases(n: number) {
  return Array.from({ length: n }, (_, i) => i * 2.399);
}

export function useAudioVisualizer(isPlaying: boolean, barCount = 22) {
  const [data, setData] = useState<AudioData>(() => EMPTY(barCount));

  const frameRef    = useRef<number>(0);
  const analyzerRef = useRef<AnalyserNode | null>(null);
  const ctxRef      = useRef<AudioContext | null>(null);
  const phases      = useRef(makePhases(barCount));
  const initDone    = useRef(false); // tried getUserMedia at least once

  // ── Animated fake (always available) ──────────────────────────────────────
  const runFake = useCallback(() => {
    cancelAnimationFrame(frameRef.current);
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
  }, [barCount]);

  // ── Real audio via Web Audio API ──────────────────────────────────────────
  const runReal = useCallback((stream: MediaStream) => {
    const ctx      = new AudioContext();
    const analyzer = ctx.createAnalyser();
    analyzer.fftSize = 256;
    analyzer.smoothingTimeConstant = 0.78;
    ctx.createMediaStreamSource(stream).connect(analyzer);
    ctxRef.current      = ctx;
    analyzerRef.current = analyzer;

    cancelAnimationFrame(frameRef.current);
    const freq = new Uint8Array(analyzer.frequencyBinCount);
    const tick = () => {
      analyzer.getByteFrequencyData(freq);
      const step = Math.max(1, Math.floor(freq.length / barCount));
      const bars = Array.from({ length: barCount }, (_, i) =>
        freq[Math.min(i * step, freq.length - 1)] / 255
      );
      const bass = (freq[0] + freq[1] + freq[2] + freq[3]) / (4 * 255);
      setData({ bars, bass });
      frameRef.current = requestAnimationFrame(tick);
    };
    frameRef.current = requestAnimationFrame(tick);
  }, [barCount]);

  useEffect(() => {
    cancelAnimationFrame(frameRef.current);

    if (!isPlaying) {
      // Smooth decay to zero
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
    }

    // If we already have a live analyzer, reuse it
    if (analyzerRef.current) {
      const analyzer = analyzerRef.current;
      const freq = new Uint8Array(analyzer.frequencyBinCount);
      const tick = () => {
        analyzer.getByteFrequencyData(freq);
        const step = Math.max(1, Math.floor(freq.length / barCount));
        const bars = Array.from({ length: barCount }, (_, i) =>
          freq[Math.min(i * step, freq.length - 1)] / 255
        );
        const bass = (freq[0] + freq[1] + freq[2] + freq[3]) / (4 * 255);
        setData({ bars, bass });
        frameRef.current = requestAnimationFrame(tick);
      };
      frameRef.current = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(frameRef.current);
    }

    // First time: try real audio, fall back to fake
    if (!initDone.current) {
      initDone.current = true;
      navigator.mediaDevices
        .getUserMedia({ audio: true, video: false })
        .then(runReal)
        .catch(() => runFake());
    } else {
      runFake();
    }

    return () => cancelAnimationFrame(frameRef.current);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isPlaying]);

  useEffect(() => {
    return () => {
      cancelAnimationFrame(frameRef.current);
      ctxRef.current?.close();
    };
  }, []);

  return data;
}
