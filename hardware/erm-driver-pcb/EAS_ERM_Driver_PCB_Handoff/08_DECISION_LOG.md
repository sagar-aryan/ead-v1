# Final Decision Log

## Locked decisions
1. PCB purpose: six-channel ERM haptic driver for EAS V1.
2. One standalone compact rectangular PCB.
3. Single copper layer.
4. No drilling, vias, or through-hole parts.
5. Direct SMD solder pads instead of connectors.
6. Six independent ERM outputs.
7. Low-side MOSFET topology.
8. IRLML6344 MOSFETs.
9. 1N5819W flyback diode per motor.
10. 100 ohm gate resistor per channel.
11. 100 kOhm gate pulldown per channel.
12. Two HT7833 SOT-89 regulators.
13. Three motors per regulator.
14. Regulator outputs remain isolated.
15. 1 uF minimum input/output ceramic per HT7833, plus selected bulk capacitors.
16. 220 uF polymer bulk on each motor output rail.
17. 10 nF suppression capacitor populated across each ERM.
18. External master switch, not on PCB.
19. Charger separate; no charging circuit on driver PCB.
20. Charger BAT connection stays with battery outside the PCB.
21. Driver PCB receives switched charger/load-side system power.
22. ESP32 is separate but receives switched raw battery/load power through an 8-pad interface.
23. Six PWM control signals and common ground are exchanged with ESP32.
24. No battery monitoring.
25. No test points.
26. EMI-aware layout is mandatory.
27. Board area is minimized by routing/placement optimization rather than fixed dimensions.
28. 0 ohm/insulated crossover is permitted only when necessary.

## Intentionally not specified
- Exact physical motor location on wearable; this board is a driver, not the mechanical harness.
- Exact ESP32 GPIO numbers; those belong to firmware/ESP32 integration.
- Exact board orientation; placement optimizer chooses it.
- Exact board outline dimensions; derive after placement/routing.
- Exact jumper count; target zero, permit one if needed.
