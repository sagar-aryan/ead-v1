# 11 — Desktop Dashboard Specification

## 1. Technology
- Tauri 2.
- React.
- TypeScript.
- Rust backend.
- Desktop targets: Windows and Linux.

## 2. Main areas
### LIVE
Show:
- patient name/ID;
- selected reference profile;
- session/segment;
- elapsed time;
- current cycle ID;
- cadence;
- speed;
- stance/swing;
- unilateral symmetry proxy;
- foot/shank orientation;
- error score;
- confidence;
- current error class;
- ZUPT state/quality;
- sensor status;
- Wi-Fi status;
- motor state.

### TRENDS
Exactly seven primary trend panels:
1. Cadence
2. Unilateral cycle symmetry proxy
3. Error score
4. Stance/swing
5. Foot angle
6. Haptic response
7. ZUPT quality

### STEP/CYCLE TABLE
Columns:
- cycle ID;
- start/end;
- cycle time;
- stance;
- swing;
- speed;
- error score;
- confidence;
- error class;
- haptic channels/level.

### RAW DATA
Filters:
- Foot/Shank;
- time range;
- cycle;
- signal;
- error markers.

Display full raw accel/gyro plus calculated orientation/status. Clicking an error marker must jump to the corresponding raw timestamp range.

### EVENTS
Timeline of initial contact, toe-off, foot-flat, ZUPT, cycle boundaries, errors, haptic ON/OFF, faults.

### HAPTICS
Table and timeline for motor channels, PWM, duration, error class, error score and confidence.

### REFERENCE PROFILES
Create/view/select/reference version history. A reference is immutable once used for an evaluation session.

### SESSIONS
Hierarchy:
```text
Patient
  -> Session
      -> Segment 001
      -> Segment 002
```

### EXPORT
Buttons/actions:
- CSV package;
- MATLAB `.mat`;
- PDF report.

## 3. Rendering rule
Do not render charts at raw 100 Hz sample granularity. The backend keeps all data; the UI receives downsampled/aggregated visual data (target 20 Hz live display) while the raw store remains untouched.

## 4. Configuration
Researcher-configurable:
- segmentation step limit;
- segmentation error limit;
- calibration/reference workflow controls;
- analysis threshold/gain parameters explicitly exposed by the firmware;
- report/export options.

Safety-critical maxima are read-only from the dashboard during a running session.

## 5. Device states
Dashboard must visibly distinguish:
- disconnected;
- booting;
- calibrating;
- reference capture;
- ready;
- running;
- paused;
- fault;
- recovered session.

## 6. Motor test
Service/test UI is disabled when a patient session is RUNNING.
