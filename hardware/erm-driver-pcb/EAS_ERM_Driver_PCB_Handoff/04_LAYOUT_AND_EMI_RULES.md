# PCB Layout, Thermal, and EMI Rules

## 1. Board optimization objective
Minimize total board area first, while maintaining all of the rules below. Do not choose a fixed board outline before placement/routing. Generate the smallest practical rectangle that can be toner-transferred and hand assembled.

There is no required board orientation or motor-pad ordering.

## 2. Copper topology
Single copper layer only. Use no vias.

Recommended minimum trace widths for the hand-etched prototype, subject to final verification:
- Main `SWITCHED_SYSTEM+` trunk: 1.5 mm or wider.
- Main `GND` trunk/high-current returns: 1.5 mm or wider.
- HT7833 input routes: >= 1.0 mm, wider where possible.
- Each regulator output motor rail: >= 1.0 mm trunk.
- Individual motor branches: >= 0.8 mm.
- PWM/gate traces: 0.30-0.50 mm.

Prefer wider traces when space allows.

## 3. Grounding
Use one low-impedance common ground network.

Keep the ESP32 interface return and sensitive control wiring away from the narrowest portion of the motor current returns. Do not force motor current to travel through thin signal-return traces.

A large contiguous copper ground region is preferred on the single available copper layer, while preserving clear routing paths for power and signals.

## 4. Regulator thermal design
The 2025 Holtek HT78xx datasheet specifies the 3-pin SOT-89 package with theta_JA = 200 degC/W and power dissipation of 0.50 W at Ta=25 degC. The HT7833 is the 3.3 V member of this family and the series is specified for 500 mA continuous output; HT78xx thermal shutdown and current limiting are included. The datasheet identifies SOT-89 pins as 1=GND, 2=VIN, 3=VOUT and specifies at least 1 uF input/output capacitors in its electrical-characteristic conditions. 

For this design load, three 90 mA nominal/max-rated motors per regulator correspond to about 270 mA; three 120 mA starting-current motors correspond to about 360 mA. At 4.2 V input and 3.3 V output, regulator dissipation is approximately 0.243 W at 270 mA and 0.324 W at 360 mA before accounting for any other load. The startup figure is transient, not the assumed continuous load.

Therefore:
- Give each SOT-89 a generous copper heat-spreading area on the ground/input side as permitted by the pinout.
- Follow the manufacturer's thermal-pad/copper guidance.
- Keep the regulator away from the hottest motor switching cluster when possible.
- Do not place the 220 uF polymer capacitor so close that it blocks the regulator's heat-spreading copper.
- Do not thermally isolate the SOT-89 from its copper land.

## 5. Regulator capacitor placement
For each HT7833:
- 1 uF input capacitor must be immediately adjacent to VIN/GND.
- 1 uF output capacitor must be immediately adjacent to VOUT/GND.
- 10 uF input bulk capacitor should be nearby, not remote.
- 220 uF output bulk capacitor should be close to the regulator/three-motor rail entry, with short return to the main ground region.

Holtek's regulator layout guidance explicitly calls for the VIN capacitor close to VIN, the VOUT capacitor close to VOUT, and thermally conductive copper for SOT-89. 

## 6. Motor switching loop
For each channel keep this loop as small as practical:
`HAPTIC_3V3_X -> ERM -> MOSFET drain/source -> GND -> local supply return`.

Place each flyback diode physically adjacent to the motor/MOSFET switching node and its motor positive connection.

Place each 10 nF motor capacitor directly across the two motor pads/networks, close to the MOSFET/ERM interface.

## 7. Gate routing
- PWM trace from interface pad to 100 ohm resistor should be short.
- 100 ohm resistor should be close to MOSFET gate.
- Gate pulldown should connect directly from the gate node to local ground.
- Keep gate traces away from high-current motor copper when practical.

## 8. EMI strategy
The main EMI sources are the PWM current edges, motor brush noise, and fast switching-node voltage transitions.

The layout must:
- minimize switching loop area;
- keep motor rail and return paths short/wide;
- keep control/signal paths physically separated from motor switching regions;
- place suppression components at the source rather than at a distant point;
- keep the large 220 uF capacitors electrically close to their respective motor rail groups;
- avoid long parallel runs of PWM and motor traces;
- avoid using a narrow trace neck as the sole connection between the motor ground and system ground.

## 9. No component substitutions
Do not replace parts simply to make routing easier.

If a purchased footprint is missing or ambiguous, create a project-local footprint and verify it against the relevant manufacturer's mechanical/pinout drawing.

## 10. Edge clearances
Prefer at least 1 mm copper-to-board-edge clearance for normal copper. External solder pads may approach the board edge as needed, but retain enough copper and mechanical area for reliable hand soldering and wire strain relief.

## 11. Jumper rule
Zero-jumper routing is preferred.

If one-layer routing creates a genuine unavoidable crossing, use either:
- a 0 ohm SMD jumper footprint populated with a suitable 1206 0 ohm part if available; or
- an explicitly documented insulated wire jumper after etching.

Do not create hidden trace crossings by using an unsupported two-layer assumption.
