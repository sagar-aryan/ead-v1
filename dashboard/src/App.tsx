import { useEffect, useState } from "react";

import { api } from "./api";
import { StateBar } from "./components/StateBar";
import { useDevice } from "./useDevice";
import { Device } from "./views/Device";
import { Live } from "./views/Live";
import { Sessions } from "./views/Sessions";

/** Views that exist today. The remaining doc 11 areas arrive with the
 *  measurements they display (gait events M4, error engine M5, export M6). */
const VIEWS = ["Live", "Sessions", "Device"] as const;
type View = (typeof VIEWS)[number];

export default function App() {
  const device = useDevice();
  const [view, setView] = useState<View>("Device");
  const [recording, setRecording] = useState<string | null>(null);

  useEffect(() => {
    const poll = () => api.recordingSession().then(setRecording).catch(() => undefined);
    poll();
    const timer = setInterval(poll, 1000);
    return () => clearInterval(timer);
  }, []);

  // Once the device is live, the sensors are the interesting view.
  useEffect(() => {
    if (device.connected && view === "Device") setView("Live");
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
          {view === "Sessions" && <Sessions device={device} />}
          {view === "Device" && <Device device={device} />}
        </main>
      </div>
    </div>
  );
}
