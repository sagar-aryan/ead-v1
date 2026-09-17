/**
 * Device and live-telemetry state for the whole UI.
 *
 * Live samples arrive at 20 Hz and go into ring buffers held in refs, so the
 * charts read them without re-rendering React; only the numeric readouts
 * re-render, at 5 Hz.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { api, Channel, type DeviceConfig, type LiveTick, type Snapshot, type Vocabulary } from "./api";

/** Seconds of live signal kept for the strips. */
const WINDOW_S = 10;
const TICK_HZ = 20;
export const RING_SIZE = WINDOW_S * TICK_HZ;

const SNAPSHOT_INTERVAL_MS = 250;
const READOUT_INTERVAL_MS = 200;

export interface Ring {
  /** Seconds since the first sample, for the x axis. */
  time: Float64Array;
  values: Float64Array[];
  length: number;
  /** Bumped on every write so consumers can tell whether anything is new. */
  revision: number;
}

function createRing(series: number): Ring {
  return {
    time: new Float64Array(RING_SIZE),
    values: Array.from({ length: series }, () => new Float64Array(RING_SIZE)),
    length: 0,
    revision: 0,
  };
}

function clear(ring: Ring) {
  ring.length = 0;
  ring.revision += 1;
}

function push(ring: Ring, time: number, samples: number[]) {
  if (ring.length === RING_SIZE) {
    ring.time.copyWithin(0, 1);
    for (const series of ring.values) series.copyWithin(0, 1);
    ring.length -= 1;
  }
  const at = ring.length;
  ring.time[at] = time;
  for (let i = 0; i < ring.values.length; i++) ring.values[i][at] = samples[i] ?? 0;
  ring.length += 1;
  ring.revision += 1;
}

export interface LiveRings {
  /** Foot and shank accel magnitude, the quickest check that scaling is right. */
  magnitude: Ring;
  footGyro: Ring;
  shankGyro: Ring;
}

export interface DeviceApi {
  snapshot: Snapshot | null;
  config: DeviceConfig | null;
  vocabulary: Vocabulary | null;
  /** Latest tick, re-rendered at 5 Hz for the numeric readouts. */
  tick: LiveTick | null;
  rings: LiveRings;
  connected: boolean;
  connect: (target: Parameters<typeof api.connect>[0]) => Promise<void>;
  disconnect: () => Promise<void>;
  error: string | null;
  clearError: () => void;
}

export function useDevice(): DeviceApi {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [config, setConfig] = useState<DeviceConfig | null>(null);
  const [vocabulary, setVocabulary] = useState<Vocabulary | null>(null);
  const [tick, setTick] = useState<LiveTick | null>(null);
  const [error, setError] = useState<string | null>(null);

  const rings = useMemo<LiveRings>(
    () => ({ magnitude: createRing(2), footGyro: createRing(3), shankGyro: createRing(3) }),
    [],
  );
  const latest = useRef<LiveTick | null>(null);
  const firstTime = useRef<number | null>(null);
  // Units change from raw counts to physical units once the device
  // configuration arrives; a chart must never mix the two.
  const anatomical = useRef<boolean | null>(null);

  useEffect(() => {
    api.vocabulary().then(setVocabulary).catch((e) => setError(String(e)));
  }, []);

  // One live channel for the app's lifetime; ticks land in refs.
  useEffect(() => {
    const channel = new Channel<LiveTick>();
    channel.onmessage = (message) => {
      latest.current = message;
      if (anatomical.current !== message.anatomical) {
        anatomical.current = message.anatomical;
        firstTime.current = null;
        clear(rings.magnitude);
        clear(rings.footGyro);
        clear(rings.shankGyro);
      }
      if (firstTime.current === null) firstTime.current = message.device_time_us;
      const t = (message.device_time_us - firstTime.current) / 1e6;
      push(rings.magnitude, t, [message.foot_accel_magnitude_g, message.shank_accel_magnitude_g]);
      push(rings.footGyro, t, message.foot_gyro_dps);
      push(rings.shankGyro, t, message.shank_gyro_dps);
    };
    api.subscribeLive(channel).catch((e) => setError(String(e)));
    return () => {
      api.unsubscribeLive().catch(() => undefined);
    };
  }, [rings]);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const next = await api.snapshot();
        if (cancelled) return;
        setSnapshot(next);
        // The configuration arrives shortly after the handshake.
        if (next.link_state === "connected") {
          const current = await api.config();
          if (!cancelled) setConfig(current);
        } else if (!cancelled) {
          setConfig(null);
          firstTime.current = null;
          anatomical.current = null;
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    };
    poll();
    const timer = setInterval(poll, SNAPSHOT_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  // Readouts refresh at 5 Hz: fast enough to look live, slow enough to read.
  useEffect(() => {
    const timer = setInterval(() => setTick(latest.current), READOUT_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);

  const connect = useCallback(async (target: Parameters<typeof api.connect>[0]) => {
    setError(null);
    try {
      await api.connect(target);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const disconnect = useCallback(async () => {
    await api.disconnect().catch((e) => setError(String(e)));
  }, []);

  return {
    snapshot,
    config,
    vocabulary,
    tick,
    rings,
    connected: snapshot?.link_state === "connected",
    connect,
    disconnect,
    error,
    clearError: () => setError(null),
  };
}
