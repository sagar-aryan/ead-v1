# Problems

## PROB-001 — Attached invoice does not contain the handoff BOM

**Status:** Resolved

### Symptoms
The user asked to design the PCB with "all the components" in `invoice_3698208 (1).pdf`. That invoice (Robu #3698208, 16/09/2026) lists only 17× 100 nF 1206 MLCC.

### Investigation
Handoff `09_SOURCE_NOTES.md` lists the board's parts from a different Robu invoice dated 13/09/2026 (IRLML6344, 1N5819W, resistors, capacitors, 220 µF polymer, and so on). The 100 nF value does not appear anywhere in the handoff.

### Root Cause
Two separate orders. The 100 nF capacitors were bought after the handoff was written.

### Resolution
Asked the user. The 100 nF parts become per-channel local decoupling C15–C20 (DEC-001); all other parts follow the handoff.

## PROB-002 — Grouped edge wire pads are not routable on one layer without jumpers

**Status:** Resolved (design choice DEC-003)

### Symptoms
Found during planning: the natural arrangement (an ESP32 pad row and a motor pad row on the board edges) cannot be routed with one copper layer and zero jumpers.

### Investigation
Graph-planarity argument; each step below is confirmed reasoning.
1. Contract each net's copper to a vertex and each channel (Q, Rg, Rpd, D, Cs) to a vertex. The result is a minor of the physical layout, so if the minor is non-planar, the layout is too.
2. A pad on the outer board edge is on the outer face. Adding one vertex O outside the board, connected to every edge pad, keeps the graph planar only if the layout existed.
3. Channels 1–3 each connect to GND (MOSFET source), to HAPTIC_3V3_A (diode/capacitor), and to O (their PWM or M- pad).
4. So {O, GND, HAPTIC_3V3_A} × {ch1, ch2, ch3} = K3,3, which is non-planar. The same holds for channels 4–6 and rail B.
5. The same argument, with U1 standing in for a channel through its SWITCHED_SYSTEM+ pad, limits edge pads further.

### Resolution
Put all PWM and M- pads inside the board, each enclosed by a GND or rail trace (DEC-003). Only power and ESP-power pads (PWR+, PWR GND, ESP BAT+, ESP GND) sit on the edge.

### Lessons
Traces passing between the pads of a component count as crossings in this model. They were rejected because the toner-transfer board has no solder mask.

## PROB-003 — Generated schematic: every pin reported unconnected (304 ERC violations)

**Status:** Resolved

### Symptoms
First ERC run found 304 violations: pins not connected, dangling labels, and wires 0.0254 mm long in the report. Pins were reported as `Pin 1 [???, ?]`.

### Investigation
The ERC JSON showed every pin located at its symbol origin, and the `???` pin type showed KiCad could not resolve the symbol definitions. Inspecting the generated `lib_symbols` found the top-level names were `R`, `C`, … instead of `Device:R`, `Device:C`.

### Root Cause
`lib_symbol()` in `build.py` started the extracted block one character early (at the tab after the newline). The `^\(symbol` rename regex therefore never matched, so instances (`lib_id "Device:R"`) had no matching embedded symbol.

### Resolution
Skip both characters (`index + 2`).

### Verification
ERC now reports 0 errors and 0 warnings with `--severity-all`.

## PROB-004 — Stock SOT-89 land violates the 0.4 mm rule

**Status:** Resolved by DEC-010

Found while computing pad geometry, before any DRC: the pin-1/3 to tab-corner gap is 0.254 mm in both KiCad SOT-89-3 lands. See DEC-010.

## PROB-005 — First DRC after routing: 1 error, 7 warnings, 64 parity warnings

**Status:** Resolved

| Item | Root cause | Fix |
|---|---|---|
| `copper_edge_clearance` 0.75 mm | GND bus (1.5 mm) ends on the last tooth; the outline was sized to the 1.0 mm tooth | Outline width uses the bus half-width |
| 7× `connection_width` 0.31–0.38 mm | GND pour slivers between pads under part bodies | DEC-011 keepouts, zone min width 0.5 mm |
| 64× parity `Datasheet` field mismatch | Footprints created by script had empty Datasheet/Description fields | `build.py` copies both fields from the model |

Verification: DRC reports 0 violations, 0 unconnected and 0 parity issues with `--severity-all`.

## PROB-006 — Tight variant: 3 edge-clearance errors of 2.5 µm

**Status:** Resolved

### Symptoms
First DRC of the tight variant: 3× `copper_edge_clearance`, actual 0.4975 mm against 0.5 mm, all at the right-edge GND link.

### Root Cause
`W = round(..., 2)` rounded 36.0625 mm down to 36.06 mm, pulling the outline 2.5 µm into the required margin. The earlier variants happened to land on exact 2-decimal values.

### Resolution
The outline width and height are now rounded up with `math.ceil`, so the margin can never be shaved.

### Verification
DRC reports 0 violations (TEST-011).

### Lessons
Round board outlines outward, never to nearest.

## PROB-007 — Toner panel sheet clipped at the page edge

**Status:** Resolved

### Symptoms
The first three 6-up panel PDFs ran off the A4 page: the labels were cut in half, and the ruler was missing from the third attempt.

### Environment
KiCad 10.0.6 `kicad-cli pcb export pdf --mirror --scale 1`, A4 landscape, `EAS_ERM_Driver_tight/scripts/panel.py`, 2026-09-23.

### Expected Behavior
The whole sheet — six board copies, two rulers and the labels — inside the page, centred.

### Actual Behavior
- Attempt 1: ink bbox x [154.73, 296.97] mm on a 297 mm page, i.e. hard against the right edge with the labels cut off.
- Attempt 2 (after a centring correction): x [77.4, 297.0], still against the right edge.
- Attempt 3 (after flipping the label justification): x [226.1, 297.0], with the ruler pushed off the page entirely; the longest ink run measured 36.20 mm, which is a board outline, not the 100 mm ruler.

### Investigation
Plotted the same panel board mirrored and unmirrored and measured both:
```
unmirrored: x [0.00, 219.46]
mirrored:   x [77.34, 296.80]
```
**Confirmed:** the unmirrored plot starts at exactly 0.00, so the content was already clipped before mirroring, and `0.00 ↔ 296.80` with `219.46 ↔ 77.34` shows the mirror axis *is* the page centre.

`pcbnew`'s item bounding boxes then showed where the content went: text placed with `SetMirrored(True)` extends away from its anchor in the *opposite* direction, so the left-justified labels anchored at x = 18 mm ran to x = −53 mm, off the page.

### Hypotheses
1. KiCad mirrors about some axis other than the page centre. **Disproved** by the measurement above.
2. Mirrored text extends the other way. **Confirmed.**
3. The centring correction was computed from a bbox that was itself clipped, so it could not see the true extent. **Confirmed** — this is why each corrected attempt failed differently.

### Root Cause
Two faults at once. Mirrored label text ran off the left of the layout, and every correction was measured from a clipped raster, so the measurement could not reveal it. A further trap: `BOARD.GetBoundingBox()` is not the plotted extent, because it counts the F.Fab legend text (about 80 mm wide per copy) that the toner plot does not include.

### Resolution
`panel.py` now:
1. right-justifies the mirrored labels, so they extend the same way as the layout;
2. computes the extent from the plotted layers only (`Edge.Cuts` and `Dwgs.User`; copper lies inside the outlines), so no raster measurement is involved in positioning;
3. shifts the layout to the page centre and rebuilds;
4. rasterizes the finished PDF at 600 dpi and exits non-zero unless the ruler, the outline and the margins all check out.

### Verification
TEST-012: printed area x [80.6, 215.8], y [53.4, 157.0] mm on a 297 × 210 mm page (centre 148.5, 105.2 — the page centre), ruler 100.29 mm of ink, outline 36.15 mm, six copies matching the verified artwork.

### Lessons
- A bbox measured from a raster is only usable when the content is known to be inside the page. Clipping makes the measurement lie in the direction that hides the fault.
- For placement, ask the source (`pcbnew` geometry, plotted layers only) and use the raster to verify the result, not to drive it.

## PROB-008 — Motors stay on at their last duty; sliders and failsafe both do nothing

**Status:** Investigating - mechanism supported by observation, fix applied, reproduction pending

### Symptoms
While driving motors from the dashboard, the channels stick at their last duty. The sliders have no
effect, the board's 3 s link-loss failsafe does not fire, and the motors stop when the user clicks
**Connect** in the dashboard - not on unplug, and not on Disconnect. The user corrected both of
those in turn. Reported twice by the
user, 2026-09-24 and 2026-09-25.

### Environment
XIAO ESP32-S3, Arduino-ESP32 core 3.3.10, USB CDC on boot, `firmware/erm_channel_test`, dashboard
`firmware/dashboard/erm_dashboard.py`, driver board powered from a LiPo.

### Expected Behavior
A duty command takes effect immediately; if the link goes quiet, the failsafe cuts all channels
within 3 s.

### Actual Behavior
Channels hold their last duty indefinitely. Recovery happens on USB disconnect, or when a program
that *reads* the port (a diagnostic script here) connects and drains it.

### Investigation
- **Confirmed:** LEDC holds its last duty with no CPU involvement, so a stuck output only requires
  that no further command is processed.
- **Confirmed:** the failsafe runs in `loop()`. It not firing means `loop()` itself was not running.
- **Confirmed:** the dashboard only ever wrote to the port; it never read.
- **Confirmed:** the firmware printed a report line (about 60 characters) for every `<ch> <percent>`
  command, and a slider drag sends roughly 25 commands per second.
- **Confirmed (decisive):** Disconnect did **not** stop the motors; Connect did. Opening the port
  is the moment the PC begins draining the device again, and `serial.Serial.open()` also flushes
  the tty buffers. Every recorded recovery is a host that starts reading: the Connect click, or a
  diagnostic script here. Closing the port, which changes nothing about draining, did not recover.
- **Confirmed:** Disconnect does not cut power to the XIAO. The ESP stays powered and its pins stay
  driven, so "the module lost power and the 100 k pulldowns took over" is excluded. The only
  remaining explanation for the motors stopping is that the firmware resumed processing.

### Mechanism
The board's USB CDC transmit buffer fills, because nothing on the PC drains it. `Serial.print()`
inside `handle()` then blocks in `loop()`, so commands stop being processed and the failsafe cannot
run, while LEDC keeps driving the pins. Closing the port removes the host, the blocked write aborts, and either the queued stop commands
are processed or the failsafe fires 3 s later - which is why the motors stop on Disconnect.

Timing would separate those two: an immediate stop means the queued command was processed, a stop
about 3 s later means the failsafe did it. Not yet observed.

The Connect-not-Disconnect observation matches this and excludes the alternatives: the ESP keeps
power and its pins throughout, so neither a reset nor the 100 k pulldowns are involved.

**Still to do:** reproduce it deliberately (flood set commands with no reader) to turn a strongly
supported explanation into a repeatable one, then confirm the fix removes it.

### Fix applied
1. `Serial.setTxTimeoutMs(0)` in `setup()`: output is dropped rather than allowed to stall the
   control loop.
2. The per-command report was removed; reports are printed only for `?`, `0` and the failsafe.
3. The dashboard drains the port on every flush and surfaces a failsafe message in the status line.

### Verification
Firmware compiles; dashboard `--selftest` passes with a new assertion that draining a closed port is
a safe no-op. **The hypothesis itself is untested.** The plan is: with the current (unfixed)
firmware, flood set commands without reading and see whether the board wedges; then flash the fix
and repeat.

### Lessons
A control loop must never be able to block on logging. Diagnostics that are silently discarded are
better than diagnostics that stop the motors from being commanded.
