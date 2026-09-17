/**
 * Live signal strip: a uPlot line chart fed from a ring buffer outside React.
 * The chart is created once and updated in an animation frame, so a 20 Hz
 * stream never re-renders the component tree.
 */
import { useEffect, useRef } from "react";
import uPlot from "uplot";

import type { Ring } from "../useDevice";

export interface StripProps {
  ring: Ring;
  /** One label and colour per series; also used for the legend. */
  series: { label: string; color: string }[];
  unit: string;
  /** Fixed y range where the expected value is known (for example 1 g). */
  range?: [number, number];
  height?: number;
}

export function Strip({ ring, series, unit, range, height = 128 }: StripProps) {
  const host = useRef<HTMLDivElement>(null);
  const plot = useRef<uPlot | null>(null);

  useEffect(() => {
    if (!host.current) return;
    const options: uPlot.Options = {
      width: host.current.clientWidth || 600,
      height,
      // Recessive axes and grid; the trace is the only assertive mark.
      axes: [
        {
          stroke: "#868c93",
          grid: { stroke: "#eceeed", width: 1 },
          ticks: { stroke: "#e3e4e2", width: 1 },
          font: '11px "IBM Plex Mono", monospace',
          size: 28,
        },
        {
          stroke: "#868c93",
          grid: { stroke: "#eceeed", width: 1 },
          ticks: { stroke: "#e3e4e2", width: 1 },
          font: '11px "IBM Plex Mono", monospace',
          size: 46,
        },
      ],
      scales: {
        x: { time: false },
        y: range ? { range: [range[0], range[1]] } : {},
      },
      legend: { show: false },
      cursor: { drag: { x: false, y: false } },
      series: [
        { label: `t (s)` },
        ...series.map((s) => ({
          label: `${s.label} (${unit})`,
          stroke: s.color,
          width: 2,
          points: { show: false },
        })),
      ],
    };
    const data: uPlot.AlignedData = [
      new Float64Array(0),
      ...series.map(() => new Float64Array(0)),
    ] as unknown as uPlot.AlignedData;
    plot.current = new uPlot(options, data, host.current);

    const resize = () => {
      if (host.current && plot.current) {
        plot.current.setSize({ width: host.current.clientWidth, height });
      }
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host.current);

    let frame = 0;
    let drawnRevision = -1;
    const tick = () => {
      frame = requestAnimationFrame(tick);
      if (!plot.current || ring.revision === drawnRevision) return;
      drawnRevision = ring.revision;
      plot.current.setData([
        ring.time.subarray(0, ring.length),
        ...ring.values.map((v) => v.subarray(0, ring.length)),
      ] as unknown as uPlot.AlignedData);
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      plot.current?.destroy();
      plot.current = null;
    };
  }, [ring, series, unit, range, height]);

  return <div ref={host} className="chart" />;
}
