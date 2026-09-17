# 16 — Research References Used to Finalize V1 Choices

## Haptic placement / gait-retraining evidence
1. **Afzal et al., “A Portable Gait Asymmetry Rehabilitation System for Individuals with Stroke Using a Vibrotactile Feedback,” BioMed Research International (2015).**
   - Describes a six-vibrotactor array covering the shank from front to back.
   - Uses a lower-leg belt rather than an in-shoe vibrotactor.
   - Uses proportional intensity/duration control.
   - This directly informed the V1 six-motor circumferential shank layout and the decision not to put motors under the barefoot sole. citeturn644476search2turn644476search7

2. **Configurable, wearable sensing and vibrotactile feedback system for real-time postural balance and gait training (Journal of NeuroEngineering and Rehabilitation, 2017).**
   - Uses medial/lateral shank vibration to cue changes in foot progression angle.
   - Uses continuous vibration outside a no-feedback zone, which supports V1's persistent error-correction/hysteresis model. citeturn644476search10turn644476search13

3. **Rhythmic Haptic Cueing for Gait Rehabilitation of People With Hemiparesis: Quantitative Gait Study (JMIR Biomedical Engineering, 2020).**
   - Places vibrotactile components on the lower leg near the knee and deliberately separates them from the IMU placement to minimize unwanted gait-data noise.
   - This supports keeping the haptic array mechanically separated from the sensing modules. citeturn644476search0

4. **Wearable lower limb haptic feedback device for retraining Foot Progression Angle and Step Width (Gait & Posture, 2017).**
   - Demonstrates ankle/shank-mounted vibrotactile feedback for spatial gait modification and distributed actuator layouts. citeturn644476search9turn644476search12

## ESP32-S3 GPIO evidence
5. **Espressif ESP32-S3 GPIO documentation.**
   - GPIO0, GPIO3, GPIO45 and GPIO46 are strapping pins.
   - GPIO19/20 are USB-JTAG defaults.
   - GPIO26–37 have SPI flash/PSRAM restrictions and are not preferred for general use on common ESP32-S3 module variants. citeturn346183search0turn346183search3

6. **Espressif ESP32-S3 hardware design / programming documentation.**
   - UART0 commonly uses GPIO43/44 for TX/RX; V1 intentionally reserves UART0 functionality in favor of native USB and uses GPIO43/44 as two motor PWM outputs. citeturn346183search4turn346183search24

## Important evidence boundary
These references inform engineering placement and architecture. They do **not** validate the V1 numerical error thresholds, haptic limits, Mahony gains, or clinical performance. Those values are explicitly V1 engineering defaults and must be validated experimentally before any clinical interpretation.
