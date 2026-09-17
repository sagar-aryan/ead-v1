# Source Requirements Captured in This Handoff

This handoff was rebuilt from the project specification and the later clinical-insight requirements supplied in the conversation.

## Original project requirements
- Wearable IMU gait system.
- Dual IMU placement on shank + foot.
- 6-channel vibrotactile feedback.
- 100 Hz sensor data target.
- Wi-Fi/BLE wireless concept; V1 selects Wi-Fi.
- Desktop Windows/Linux dashboard.
- Gait speed, cadence, step symmetry.
- Stance/swing.
- Foot-drop-like detection.
- Original requirement says knee-angle estimation; V1 explicitly excludes direct knee-angle measurement because the final hardware has only foot + shank sensing.
- Heel-strike/toe-off events.
- Real-time plots.
- Clinical/research PDF report.
- CSV/MATLAB export. fileciteturn9file1L69-L96

## Later clinical requirements
- Distinguish real movement errors from noisy/off sensors.
- Keep gait numbers accurate over a session.
- Use ZUPT during foot-flat/zero velocity intervals.
- One quantitative error value per gait cycle/step drives haptic strength.
- More deviation -> stronger vibration, with a hard upper limit.
- Compare each patient to their own normal/reference gait.
- Short reference capture and persistent stored baseline.
- Export full raw accelerometer/gyro/orientation with a synchronized timestamp on every sample. fileciteturn9file0L9-L49 fileciteturn9file0L51-L59

## Final hardware decisions supplied later
- V1 sensors are MPU6050 ×2.
- Foot sensor address 0x68.
- Shank sensor address 0x69.
- Existing breakout-board I²C pull-ups are used.
- No FSR.
- Direct PWM through six MOSFETs.
- Two HT7833 haptic rails, three motors each; firmware does not implement power-management logic.
- No battery ADC.
- No switch status.
- Internal ESP32 flash for recovery storage.
