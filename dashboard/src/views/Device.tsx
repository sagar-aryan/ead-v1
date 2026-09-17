/**
 * DEVICE: connect a link and inspect what the device reports about itself.
 * Everything shown here comes from the device (HELLO, STATUS, CONFIG_GET), so
 * it can be compared against docs/hardware.md without trusting the app.
 */
import { useEffect, useState } from "react";

import { api, type LinkTarget, type UsbPortInfo } from "../api";
import type { DeviceApi } from "../useDevice";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="readout">
      <span className="label">{label}</span>
      <span className="value" style={{ fontSize: 14 }}>
        {children}
      </span>
    </div>
  );
}

function mountToText(map: number[][]): string {
  const axes = ["X", "Y", "Z"];
  return map
    .map((row, axis) => {
      const terms = row
        .map((sign, source) =>
          sign === 0 ? null : `${sign < 0 ? "−" : "+"}chip${axes[source]}`,
        )
        .filter(Boolean);
      return `${axes[axis]} = ${terms.join(" ")}`;
    })
    .join(", ");
}

export function Device({ device }: { device: DeviceApi }) {
  const { snapshot, config, connect, disconnect, error } = device;
  const [ports, setPorts] = useState<UsbPortInfo[]>([]);
  const [wifiUrl, setWifiUrl] = useState("");

  useEffect(() => {
    api.listUsbPorts().then(setPorts).catch(() => setPorts([]));
  }, [snapshot?.link_state]);

  useEffect(() => {
    if (device.vocabulary && wifiUrl === "") setWifiUrl(device.vocabulary.default_wifi_url);
  }, [device.vocabulary, wifiUrl]);

  const connected = snapshot?.link_state === "connected";
  const busy = snapshot?.link_state === "connecting";
  const status = snapshot?.status;

  const connectUsb = (port: string | null) => connect({ kind: "usb", port } as LinkTarget);

  return (
    <>
      <h1>Device</h1>
      <p className="page-hint">
        The Wi-Fi link is the one the wearable uses in a session. USB carries the
        same data for bench work, and keeps this computer on its own network.
      </p>

      <div className="panel">
        <h2>Connection</h2>
        {connected ? (
          <>
            <p className="hint">Connected over {snapshot?.link_description}.</p>
            <button onClick={() => disconnect()}>Disconnect</button>
          </>
        ) : (
          <>
            <div className="row" style={{ marginBottom: 12 }}>
              {ports.length === 0 ? (
                <p className="empty">
                  No device found on USB. Check the cable, or connect over Wi-Fi below.
                </p>
              ) : (
                ports.map((port) => (
                  <button
                    key={port.port}
                    className="primary"
                    disabled={busy}
                    onClick={() => connectUsb(port.port)}
                  >
                    Connect {port.port}
                    {port.serial_number ? ` (${port.serial_number})` : ""}
                  </button>
                ))
              )}
            </div>
            <div className="row">
              <div className="field" style={{ flex: 1, maxWidth: 360 }}>
                <label htmlFor="wifi-url">Wi-Fi WebSocket address</label>
                <input
                  id="wifi-url"
                  value={wifiUrl}
                  onChange={(e) => setWifiUrl(e.target.value)}
                  spellCheck={false}
                />
              </div>
              <button disabled={busy || wifiUrl === ""} onClick={() => connect({ kind: "wifi", url: wifiUrl })}>
                Connect over Wi-Fi
              </button>
            </div>
            <p className="hint" style={{ marginTop: 8 }}>
              Join this computer to the device access point first; its name starts
              with EAD-V1.
            </p>
          </>
        )}
        {error && <p className="error">{error}</p>}
        {snapshot?.last_error && !connected && <p className="error">{snapshot.last_error}</p>}
        {snapshot?.schema_mismatch && (
          <p className="error">
            The device speaks a different protocol schema than this build. Update one of them.
          </p>
        )}
      </div>

      {connected && (
        <div className="panel">
          <h2>Identity</h2>
          <div className="readouts">
            <Field label="Firmware">
              <span className="num">{snapshot?.firmware}</span>
            </Field>
            <Field label="MAC">
              <span className="num">{snapshot?.mac}</span>
            </Field>
            <Field label="Boot ID">
              <span className="num">
                {snapshot?.boot_id?.toString(16).padStart(8, "0")}
              </span>
            </Field>
            <Field label="Capabilities">
              {snapshot?.capabilities.length ? snapshot.capabilities.join(", ") : <span className="absent">none</span>}
            </Field>
            <Field label="Haptics">
              <span className="absent">not fitted</span>
            </Field>
          </div>
        </div>
      )}

      {connected && status && (
        <div className="panel">
          <h2>Health</h2>
          <p className="hint">
            Counters are cumulative since the device booted.
          </p>
          <div className="readouts">
            <Field label="Frames">
              <span className="num">{status.frame_index.toLocaleString()}</span>
            </Field>
            <Field label="Dropped">
              <span className="num">{status.frames_dropped}</span>
            </Field>
            <Field label="I²C errors">
              <span className="num">{status.i2c_errors}</span>
            </Field>
            <Field label="Shank repeats">
              <span className="num">{status.shank_repeated}</span>
            </Field>
            <Field label="IMU re-inits">
              <span className="num">{status.imu_reinits}</span>
            </Field>
            <Field label="Corrupted link frames">
              <span className="num">{snapshot.rejected_frames}</span>
            </Field>
            <Field label="Stored on device">
              <span className="num">
                {status.oldest_seq}–{status.last_seq}
              </span>
            </Field>
            <Field label="Free heap (min)">
              <span className="num">{(status.heap_free_min / 1024).toFixed(0)}<span className="unit">kB</span></span>
            </Field>
            <Field label="Wi-Fi clients">
              <span className="num">
                {status.ap_stations}
                {status.ap_stations > 0 && (
                  <>
                    {" "}
                    <span className="unit">{status.ap_rssi_dbm} dBm</span>
                  </>
                )}
              </span>
            </Field>
          </div>
        </div>
      )}

      {connected && config && (
        <div className="panel">
          <h2>Configuration</h2>
          <p className="hint">
            Read from the device and verified against the hash it reports. This is
            what gets recorded with every session.
          </p>
          <div className="readouts">
            <Field label="Sample rate">
              <span className="num">{config.imu.sample_hz}<span className="unit">Hz</span></span>
            </Field>
            <Field label="Accelerometer">
              <span className="num">±{config.imu.accel_range_g}<span className="unit">g</span></span>
            </Field>
            <Field label="Gyroscope">
              <span className="num">±{config.imu.gyro_range_dps}<span className="unit">°/s</span></span>
            </Field>
            <Field label="Low-pass">
              <span className="num">{config.imu.dlpf_hz}<span className="unit">Hz</span></span>
            </Field>
            <Field label="I²C addresses">
              <span className="num">
                0x{config.imu.foot_address.toString(16)} / 0x{config.imu.shank_address.toString(16)}
              </span>
            </Field>
            <Field label="Mahony gains">
              <span className="num">
                {config.mahony_kp} / {config.mahony_ki}
              </span>
            </Field>
          </div>
          <table style={{ marginTop: 16 }}>
            <thead>
              <tr>
                <th>Sensor</th>
                <th>Mount map (anatomical from chip axes)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <span className="sensor foot">
                    <span className="name">Foot</span>
                  </span>
                </td>
                <td className="num">{mountToText(config.imu.foot_mount)}</td>
              </tr>
              <tr>
                <td>
                  <span className="sensor shank">
                    <span className="name">Shank</span>
                  </span>
                </td>
                <td className="num">{mountToText(config.imu.shank_mount)}</td>
              </tr>
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}
