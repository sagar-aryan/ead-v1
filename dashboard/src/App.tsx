import { useEffect, useState } from "react";

import { api } from "./api";
import { StateBar } from "./components/StateBar";
import { useDevice } from "./useDevice";
import { Check } from "./views/Check";
import { Device } from "./views/Device";
import { Live } from "./views/Live";
import { Cycles } from "./views/Cycles";
import { Events } from "./views/Events";
import { Export } from "./views/Export";
import { References } from "./views/References";
import { Raw } from "./views/Raw";
import { Sessions } from "./views/Sessions";

/**
 * Doc 11 order. HAPTICS is absent: no error-driven feedback exists (DEC-006).
 * Check is doc 11 §6's motor test, with the sensor wiring check beside it.
 */
const VIEWS = [
  "Live",
  "Cycles",
  "Events",
  "Raw data",
  "References",
  "Sessions",
  "Export",
  "Device",
  "Check",
] as const;
type View = (typeof VIEWS)[number];

export default function App() {
  const device = useDevice();
  const [view, setView] = useState<View>("Device");
  const [recording, setRecording] = useState<string | null>(null);
  const [endedByRestart, setEndedByRestart] = useState<string | null>(null);
  const [checkMode, setCheckMode] = useState(false);

  // `ead --check` opens on the checks and stays there once connected.
  useEffect(() => {
    api
      .launchMode()
      .then((mode) => {
        if (mode !== "check") return;
        setCheckMode(true);
        setView("Check");
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    const poll = () => {
      api.recordingSession().then(setRecording).catch(() => undefined);
      api.endedByRestart().then(setEndedByRestart).catch(() => undefined);
    };
    poll();
    const timer = setInterval(poll, 1000);
    return () => clearInterval(timer);
  }, []);

  // Once the device is live, the sensors are the interesting view.
  useEffect(() => {
    if (device.connected && view === "Device" && !checkMode) setView("Live");
    // Only on the transition into a live link.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.connected]);

  return (
    <div className="app">
      <StateBar
        snapshot={device.snapshot}
        config={device.config}
        vocabulary={device.vocabulary}
        recording={recording}
        endedByRestart={endedByRestart}
      />
      <div className="body">
        <nav>
          {VIEWS.map((name) => (
            <button key={name} aria-current={view === name} onClick={() => setView(name)}>
              {name}
            </button>
          ))}
        </nav>
        <main>
          {view === "Live" && <Live device={device} />}
          {view === "Cycles" && <Cycles device={device} />}
          {view === "Events" && <Events />}
          {view === "References" && <References device={device} />}
          {view === "Raw data" && <Raw />}
          {view === "Sessions" && <Sessions device={device} />}
          {view === "Export" && <Export />}
          {view === "Device" && <Device device={device} />}
          {view === "Check" && <Check device={device} recording={recording} />}
        </main>
      </div>
    </div>
  );
}
