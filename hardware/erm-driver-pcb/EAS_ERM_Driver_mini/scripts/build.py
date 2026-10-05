#!/usr/bin/env python3
"""Build the EAS V1 six-channel ERM driver KiCad project, MINI variant (copy of ../EAS_ERM_Driver_compact with
standard footprints, 0.4 mm wire-pad gap, 0.5 mm edge margin, 1.0 mm power trunks: DEC-017), from one netlist model.

Writes (overwriting) EAS.kicad_sym, EAS.pretty/, sym-lib-table, fp-lib-table,
EAS_ERM_Driver_mini.kicad_sch, .kicad_pcb, .kicad_pro and .kicad_dru.
Hand edits made in the KiCad GUI are lost when this script is re-run.

Usage:  python3 scripts/build.py          (needs KiCad 10 python module `pcbnew`)
"""
import json
import os
import re
import uuid

import pcbnew

PROJ = "EAS_ERM_Driver_mini"
DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KI = "/usr/share/kicad"
NS = uuid.UUID("5f6b3c1e-8d2a-4b7e-9c4d-2a1e0f3b6d7c")


def uid(key):
    """Deterministic UUID so re-running the script gives stable files."""
    return str(uuid.uuid5(NS, key))


# --------------------------------------------------------------------------
# Netlist model (single source of truth for schematic and PCB)
# --------------------------------------------------------------------------
SW, GND, RA, RB = "SWITCHED_SYSTEM+", "GND", "HAPTIC_3V3_A", "HAPTIC_3V3_B"
FP_R = "Resistor_SMD:R_1206_3216Metric"
FP_C = "Capacitor_SMD:C_1206_3216Metric"
FP_CP = "Capacitor_SMD:CP_Elec_5x5.9"
FP_U = "EAS:SOT-89-3_0.4mm"
FP_Q = "Package_TO_SOT_SMD:SOT-23"
FP_D = "Diode_SMD:D_SOD-123"
WIRE_CLR = 0.4  # mini variant: wire pads use the same 0.4 mm rule as everything else (DEC-017)
PADS = {"PWR": (3.0, 2.5), "ESP": (2.0, 2.0), "MOT": (2.5, 2.5), "RAIL": (2.5, 7.0)}

HT_URL = "https://www.holtek.com/webapi/116711/HT78xxv151.pdf"
Q_URL = "https://www.infineon.com/assets/row/public/documents/24/49/infineon-irlml6344-datasheet-en.pdf"

PARTS = []


def add(ref, sym, value, fp, pins, desc="", ds="~"):
    PARTS.append(dict(ref=ref, sym=sym, value=value, fp=fp, pins=pins, desc=desc, ds=ds))


def rail(n):
    return RA if n <= 3 else RB


add("U1", "EAS:HT7833", "HT7833", FP_U, {"1": GND, "2": SW, "3": RA}, "3.3 V LDO, rail A (M1-M3)", HT_URL)
add("U2", "EAS:HT7833", "HT7833", FP_U, {"1": GND, "2": SW, "3": RB}, "3.3 V LDO, rail B (M4-M6)", HT_URL)
add("C1", "Device:C", "1uF", FP_C, {"1": SW, "2": GND}, "U1 input, X7R 50 V")
add("C2", "Device:C", "1uF", FP_C, {"1": SW, "2": GND}, "U2 input, X7R 50 V")
add("C3", "Device:C", "10uF", FP_C, {"1": SW, "2": GND}, "U1 input bulk, X7R")
add("C4", "Device:C", "10uF", FP_C, {"1": SW, "2": GND}, "U2 input bulk, X7R")
add("C5", "Device:C", "1uF", FP_C, {"1": RA, "2": GND}, "U1 output, X7R 50 V")
add("C6", "Device:C", "1uF", FP_C, {"1": RB, "2": GND}, "U2 output, X7R 50 V")
add("C7", "Device:C_Polarized", "220uF", FP_CP, {"1": RA, "2": GND}, "Rail A bulk, 6.3 V polymer, SMD V-chip D5")
add("C8", "Device:C_Polarized", "220uF", FP_CP, {"1": RB, "2": GND}, "Rail B bulk, 6.3 V polymer, SMD V-chip D5")
for n in range(1, 7):
    pwm, gate, mneg, r = f"PWM{n}", f"GATE{n}", f"M{n}-", rail(n)
    add(f"R{n}", "Device:R", "100", FP_R, {"1": pwm, "2": gate}, f"Gate series, channel {n}")
    add(f"R{n + 6}", "Device:R", "100k", FP_R, {"1": gate, "2": GND}, f"Gate pulldown, channel {n}")
    add(f"Q{n}", "Transistor_FET:Q_NMOS_GSD", "IRLML6344", FP_Q, {"1": gate, "2": GND, "3": mneg},
        f"Low-side switch, motor {n}", Q_URL)
    add(f"D{n}", "Device:D_Schottky", "1N5819W", FP_D, {"1": r, "2": mneg}, f"Flyback, motor {n}")
    add(f"C{n + 8}", "Device:C", "10nF", FP_C, {"1": r, "2": mneg}, f"Motor {n} suppression, across motor")
    add(f"C{n + 14}", "Device:C", "100nF", FP_C, {"1": r, "2": GND},
        f"Channel {n} local rail decoupling (DEC-001)")

# Wire pads: (ref, function label, net, pad style)
WIREPADS = [("J1", "SWITCHED_SYSTEM+", SW, "PWR"), ("J2", "GND", GND, "PWR"),
            ("J3", "ESP_SW_BAT+", SW, "ESP"), ("J4", "ESP_GND", GND, "ESP")]
WIREPADS += [(f"J{4 + n}", f"PWM{n}", f"PWM{n}", "ESP") for n in range(1, 7)]
WIREPADS += [("J11", "MOTOR_A+", RA, "RAIL")] + [(f"J{11 + n}", f"M{n}-", f"M{n}-", "MOT") for n in (1, 2, 3)]
WIREPADS += [("J15", "MOTOR_B+", RB, "RAIL")] + [(f"J{12 + n}", f"M{n}-", f"M{n}-", "MOT") for n in (4, 5, 6)]


def padfp(style):
    w, h = PADS[style]
    return f"EAS:WirePad_{w:.1f}x{h:.1f}mm"


for ref, label, net, style in WIREPADS:
    add(ref, "Connector_Generic:Conn_01x01", label, padfp(style), {"1": net}, "SMD wire solder pad (no hole)")

for p in PARTS:
    p["uuid"] = uid("sym:" + p["ref"])


# --------------------------------------------------------------------------
# S-expression helpers
# --------------------------------------------------------------------------
def block_at(text, start):
    """Balanced (...) block starting at text[start] == '(' (string aware)."""
    depth, i, instr = 0, start, False
    while i < len(text):
        c = text[i]
        if instr:
            if c == "\\":
                i += 1
            elif c == '"':
                instr = False
        elif c == '"':
            instr = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
        i += 1
    raise ValueError("unbalanced s-expression")


def lib_symbol(lib, name):
    t = open(f"{KI}/symbols/{lib}.kicad_sym").read()
    i = t.index(f'\n\t(symbol "{name}"') + 2  # skip the newline and tab
    b = block_at(t, i)
    assert "(extends" not in b, f"{lib}:{name} extends another symbol"
    return b


def set_prop(block, name, value):
    return re.sub(r'(\(property "%s" )"[^"]*"' % re.escape(name), lambda m: m.group(1) + json.dumps(value), block,
                  count=1)


def ht7833_symbol():
    """Holtek HT78xx SOT-89 has the same pinout as KiCad's HT75xx-1-SOT89 (1 GND, 2 VIN, 3 VOUT)."""
    b = lib_symbol("Regulator_Linear", "HT75xx-1-SOT89").replace("HT75xx-1-SOT89", "HT7833")
    b = set_prop(b, "Value", "HT7833")
    b = set_prop(b, "Footprint", FP_U)
    b = set_prop(b, "Datasheet", HT_URL)
    b = set_prop(b, "Description", "Holtek HT7833 3.3 V 500 mA LDO, SOT-89 (1 GND, 2 VIN/tab, 3 VOUT)")
    b = set_prop(b, "ki_keywords", "LDO regulator 3.3V Holtek HT78xx")
    return b


def pins_of(block):
    """[(number, x, y, angle)] from a library symbol block (library coords, Y up)."""
    out = []
    for m in re.finditer(r"\(pin \w+ \w+\s*\(at ([-\d.]+) ([-\d.]+) ([-\d.]+)\)", block):
        pb = block_at(block, m.start())
        num = re.search(r'\(number "([^"]*)"', pb).group(1)
        out.append((num, float(m.group(1)), float(m.group(2)), float(m.group(3))))
    return out


def props_of(block):
    """{name: (x, y, angle, effects-block)} for the top-level properties of a symbol."""
    out = {}
    for m in re.finditer(r'\(property "([^"]+)" "', block):
        pb = block_at(block, m.start())
        at = re.search(r"\(at ([-\d.]+) ([-\d.]+) ([-\d.]+)\)", pb)
        eff = block_at(pb, pb.index("(effects"))
        out.setdefault(m.group(1), (float(at.group(1)), float(at.group(2)), float(at.group(3)), eff))
    return out


# --------------------------------------------------------------------------
# Project-local libraries
# --------------------------------------------------------------------------
def write_libs():
    with open(f"{DIR}/EAS.kicad_sym", "w") as f:
        f.write('(kicad_symbol_lib\n\t(version 20251024)\n\t(generator "eas_build_py")\n'
                '\t(generator_version "10.0")\n\t' + ht7833_symbol() + "\n)\n")
    with open(f"{DIR}/sym-lib-table", "w") as f:
        f.write('(sym_lib_table\n\t(version 7)\n\t(lib (name "EAS")(type "KiCad")(uri "${KIPRJMOD}/EAS.kicad_sym")'
                '(options "")(descr "EAS ERM driver project symbols"))\n)\n')
    with open(f"{DIR}/fp-lib-table", "w") as f:
        f.write('(fp_lib_table\n\t(version 7)\n\t(lib (name "EAS")(type "KiCad")(uri "${KIPRJMOD}/EAS.pretty")'
                '(options "")(descr "EAS ERM driver project footprints"))\n)\n')
    os.makedirs(f"{DIR}/EAS.pretty", exist_ok=True)

    # SOT-89: KiCad's SOT-89-3 land leaves only 0.25 mm between the tab pad and pins 1/3 (inner
    # tab corner). The user's toner-transfer rule is 0.4 mm, so the wide tab area starts 0.275 mm
    # further from the pin row (joined to the pin-2 lead by a 0.9 mm neck). Mini variant: standard land.
    t = open(f"{KI}/footprints/Package_TO_SOT_SMD.pretty/SOT-89-3.kicad_mod").read()
    t = t.replace('(footprint "SOT-89-3"', '(footprint "SOT-89-3_0.4mm"', 1)
    t = re.sub(r'\(descr "[^"]*"', '(descr "SOT-89-3 land, tab pad trimmed so every pad gap is '
               '>= 0.4 mm (derived from KiCad SOT-89-3 for toner-transfer etching)"', t, count=1)
    old = "(xy 3.8625 0.8665) (xy 0.7375 0.8665) (xy 0.7375 -0.8665) (xy 3.8625 -0.8665)"
    assert old in t, "unexpected SOT-89 footprint geometry"
    t = t.replace(old, "(xy 3.8625 0.8665) (xy 1.0125 0.8665) (xy 1.0125 -0.8665) (xy 3.8625 -0.8665)")
    neck = ("\n\t\t\t(gr_poly\n\t\t\t\t(pts\n\t\t\t\t\t(xy 1.1 0.45) (xy 0.6 0.45) (xy 0.6 -0.45) (xy 1.1 -0.45)"
            "\n\t\t\t\t)\n\t\t\t\t(width 0)\n\t\t\t\t(fill yes)\n\t\t\t)")
    t = t.replace("(primitives", "(primitives" + neck, 1)
    open(f"{DIR}/EAS.pretty/SOT-89-3_0.4mm.kicad_mod", "w").write(t)

    for style, (w, h) in PADS.items():
        name = padfp(style).split(":")[1]
        open(f"{DIR}/EAS.pretty/{name}.kicad_mod", "w").write(f'''(footprint "{name}"
	(version 20251024)
	(generator "eas_build_py")
	(layer "F.Cu")
	(descr "SMD wire solder pad {w} x {h} mm, no hole, {WIRE_CLR} mm local clearance (EAS ERM driver)")
	(tags "wire pad solder")
	(property "Reference" "REF**" (at 0 {-h / 2 - 0.8:.2f} 0) (layer "F.Fab") (hide yes)
		(effects (font (size 0.8 0.8) (thickness 0.12))))
	(property "Value" "{name}" (at 0 0 {90 if h > w else 0}) (layer "F.Fab")
		(effects (font (size 0.7 0.7) (thickness 0.1))))
	(attr smd)
	(fp_rect (start {-w / 2} {-h / 2}) (end {w / 2} {h / 2}) (stroke (width 0.1) (type solid)) (fill no) (layer "F.Fab"))
	(pad "1" smd rect (at 0 0) (size {w} {h}) (layers "F.Cu" "F.Mask") (clearance {WIRE_CLR}))
	(embedded_fonts no)
)
''')


# --------------------------------------------------------------------------
# Schematic
# --------------------------------------------------------------------------
G = 2.54


def sch_positions():
    """Symbol origin (x, y) on an A3 sheet, grouped by function."""
    pos = {}
    y = 30.48
    for i, (ref, _, _, _) in enumerate(WIREPADS):
        pos[ref] = (60.96, y)
        y += 7.62 + (5.08 if ref in ("J2", "J10", "J14") else 0)
    for u, y0, cin, cbulk, cout, cp in (("U1", 40.64, "C1", "C3", "C5", "C7"), ("U2", 101.6, "C2", "C4", "C6", "C8")):
        pos[u] = (149.86, y0)
        pos[cin], pos[cbulk] = (124.46, y0 + 27.94), (134.62, y0 + 27.94)
        pos[cout], pos[cp] = (162.56, y0 + 27.94), (172.72, y0 + 27.94)
    for n in range(1, 7):
        bx = 187.96 + ((n - 1) % 3) * 76.2
        by = 60.96 + ((n - 1) // 3) * 99.06
        pos[f"R{n}"] = (bx, by)
        pos[f"R{n + 6}"] = (bx + 12.7, by + 15.24)
        pos[f"Q{n}"] = (bx + 27.94, by)
        pos[f"D{n}"] = (bx + 43.18, by - 22.86)
        pos[f"C{n + 8}"] = (bx + 55.88, by - 7.62)
        pos[f"C{n + 14}"] = (bx + 66.04, by - 7.62)
    return pos


def eff(size=1.27, justify=None, hide=False):
    j = f" (justify {justify})" if justify else ""
    h = " (hide yes)" if hide else ""
    return f"(effects (font (size {size} {size})){j}{h})"


def sch_symbol(p, x, y, lib_block):
    props = props_of(lib_block)
    flags = {k: re.search(r"\(%s (\w+)\)" % k, lib_block) for k in ("in_bom", "on_board")}
    vals = {"Reference": p["ref"], "Value": p["value"], "Footprint": p.get("fp", ""),
            "Datasheet": p.get("ds", "~"), "Description": p.get("desc", "")}
    out = [f'\t(symbol (lib_id "{p["sym"]}") (at {x:.2f} {y:.2f} 0) (unit 1) (body_style 1)',
           f'\t\t(exclude_from_sim no) (in_bom {flags["in_bom"].group(1)}) (on_board {flags["on_board"].group(1)})'
           f' (in_pos_files yes) (dnp no)',
           f'\t\t(uuid "{p["uuid"]}")']
    for name, val in vals.items():
        px, py, pa, pe = props.get(name, (0, 0, 0, eff(hide=True)))
        if name in ("Footprint", "Datasheet", "Description") and "(hide yes)" not in pe:
            pe = pe[:-1] + " (hide yes))"
        out.append(f'\t\t(property "{name}" {json.dumps(val)} (at {x + px:.2f} {y - py:.2f} {pa:g}) '
                   f'(show_name no) (do_not_autoplace no) {pe})')
    for num, *_ in pins_of(lib_block):
        out.append(f'\t\t(pin "{num}" (uuid "{uid("pin:" + p["ref"] + ":" + num)}"))')
    out.append(f'\t\t(instances (project "{PROJ}" (path "/{uid("root")}" (reference "{p["ref"]}") (unit 1))))')
    out.append("\t)")
    return "\n".join(out)


def pin_label(key, x, y, ang, net):
    """Short wire stub from a pin end plus a global label carrying the exact net name."""
    dx, dy = {0: (-1, 0), 180: (1, 0), 90: (0, 1), 270: (0, -1)}[int(ang) % 360]
    ex, ey = x + dx * G, y + dy * G
    lang, just = {(-1, 0): (180, "right"), (1, 0): (0, "left"), (0, -1): (90, "left"), (0, 1): (270, "right")}[(dx, dy)]
    return (f'\t(wire (pts (xy {x:.2f} {y:.2f}) (xy {ex:.2f} {ey:.2f})) (stroke (width 0) (type default)) '
            f'(uuid "{uid("w:" + key)}"))\n'
            f'\t(global_label {json.dumps(net)} (shape passive) (at {ex:.2f} {ey:.2f} {lang}) (fields_autoplaced yes) '
            f'{eff(justify=just)} (uuid "{uid("gl:" + key)}")\n'
            f'\t\t(property "Intersheetrefs" "${{INTERSHEET_REFS}}" (at {ex:.2f} {ey:.2f} 0) {eff(hide=True)}))')


NOTES = """EAS V1 six-channel ERM driver (single-sided, SMD only, no vias, no holes).
Source of truth: scripts/build.py (edit the script, then re-run it).
Power: SWITCHED_SYSTEM+ comes from the external master switch (MCP73833 LOAD+ side), never raw BAT.
U1 -> HAPTIC_3V3_A -> motors 1-3; U2 -> HAPTIC_3V3_B -> motors 4-6. The two rails are never joined.
Each channel: PWMn -> 100R -> IRLML6344 gate, 100k pulldown, 1N5819W flyback (K to rail),
10 nF across the motor, 100 nF rail-to-GND local decoupling (C15-C20, DEC-001).
Wire pads J1-J18 are SMD solder pads, not connectors. See docs/ for decisions and pad map."""


def write_schematic():
    libs = {"EAS:HT7833": ht7833_symbol()}
    for p in PARTS:
        if p["sym"] not in libs:
            lib, name = p["sym"].split(":")
            libs[p["sym"]] = lib_symbol(lib, name)
    libs["power:PWR_FLAG"] = lib_symbol("power", "PWR_FLAG")
    lib_blocks = {k: re.sub(r'^\(symbol "[^"]+"', f'(symbol "{k}"', v, count=1) for k, v in libs.items()}

    pos = sch_positions()
    body = []
    for p in PARTS:
        x, y = pos[p["ref"]]
        blk = libs[p["sym"]]
        body.append(sch_symbol(p, x, y, blk))
        for num, px, py, pa in pins_of(blk):
            body.append(pin_label(p["ref"] + ":" + num, x + px, y - py, pa, p["pins"][num]))
    for i, net in enumerate((SW, GND)):
        flag = dict(ref=f"#FLG0{i + 1}", sym="power:PWR_FLAG", value="PWR_FLAG", fp="", ds="~",
                    desc="Marks the net as driven by the external supply", uuid=uid("flag" + net))
        x, y = 81.28 + i * 12.7, 25.4
        body.append(sch_symbol(flag, x, y, libs["power:PWR_FLAG"]))
        body.append(pin_label("flag:" + net, x, y, 90, net))
    body.append(f'\t(text {json.dumps(NOTES)} (exclude_from_sim no) (at 30.48 205.74 0) '
                f'{eff(1.5, "left bottom")} (uuid "{uid("notes")}"))')

    sch = ["(kicad_sch", "\t(version 20260101)", '\t(generator "eeschema")', '\t(generator_version "10.0")',
           f'\t(uuid "{uid("root")}")', '\t(paper "A3")',
           '\t(title_block (title "EAS V1 six-channel ERM driver") (date "2026-09-22") (rev "V1")'
           ' (comment 1 "Generated by scripts/build.py from the netlist model; do not hand-edit")'
           ' (comment 2 "Handoff: EAS_ERM_Driver_PCB_Handoff/; decisions: docs/decisions.md"))',
           "\t(lib_symbols"]
    sch += ["\t\t" + b for b in lib_blocks.values()]
    sch += ["\t)"] + body
    sch += ['\t(sheet_instances (path "/" (page "1")))', "\t(embedded_fonts no)", ")"]
    open(f"{DIR}/{PROJ}.kicad_sch", "w").write("\n".join(sch) + "\n")


# --------------------------------------------------------------------------
# Project file and DRC rules
# --------------------------------------------------------------------------
def netclass(name, clearance, width):
    return {"name": name, "clearance": clearance, "track_width": width, "via_diameter": 0.8, "via_drill": 0.4,
            "microvia_diameter": 0.3, "microvia_drill": 0.1, "diff_pair_width": 0.4, "diff_pair_gap": 0.4,
            "diff_pair_via_gap": 0.4, "bus_width": 12, "wire_width": 6, "line_style": 0,
            "pcb_color": "rgba(0, 0, 0, 0.000)", "schematic_color": "rgba(0, 0, 0, 0.000)",
            "priority": 2147483647 if name == "Default" else {"Power": 0, "Rail": 1, "Motor": 2}[name]}


def write_project():
    pro = {
        "board": {"design_settings": {
            "rules": {"min_clearance": 0.4, "min_track_width": 0.4, "min_copper_edge_clearance": 0.5,
                      "min_connection": 0.4, "min_resolved_spokes": 1, "min_via_diameter": 0.8,
                      "min_through_hole_diameter": 0.3, "min_hole_clearance": 0.25, "min_hole_to_hole": 0.25,
                      "min_microvia_diameter": 0.2, "min_microvia_drill": 0.1, "min_via_annular_width": 0.1,
                      "allow_blind_buried_vias": False, "allow_microvias": False},
            "rule_severities": {
                # Silkscreen is not fabricated on a toner-transfer board (DEC-006); fab text only.
                "silk_overlap": "ignore", "silk_over_copper": "ignore", "silk_edge_clearance": "ignore",
                "text_height": "ignore", "text_thickness": "ignore"},
        }},
        "net_settings": {
            "classes": [netclass("Default", 0.4, 0.5), netclass("Power", 0.4, 1.0),
                        netclass("Rail", 0.4, 1.0), netclass("Motor", 0.4, 0.8)],
            "netclass_patterns": [{"netclass": "Power", "pattern": SW}, {"netclass": "Power", "pattern": GND},
                                  {"netclass": "Rail", "pattern": "HAPTIC_3V3_*"},
                                  {"netclass": "Motor", "pattern": "M?-"}],
            "meta": {"version": 5},
        },
        "libraries": {"pinned_footprint_libs": [], "pinned_symbol_libs": []},
        "meta": {"filename": f"{PROJ}.kicad_pro", "version": 3},
        "sheets": [[uid("root"), "Root"]],
        "text_variables": {},
    }
    open(f"{DIR}/{PROJ}.kicad_pro", "w").write(json.dumps(pro, indent=2) + "\n")
    open(f"{DIR}/{PROJ}.kicad_dru", "w").write("""(version 1)
# Single-sided, drill-free toner-transfer board (handoff 05 hard requirements 1-4).
(rule "no vias"
	(constraint disallow via))
(rule "no holes"
	(constraint disallow hole))
(rule "back copper stays empty"
	(layer B.Cu)
	(constraint disallow track via zone pad text graphic))
# Handoff 04 section 2: every high-current branch at least 0.8 mm (trunks are drawn wider by build.py).
(rule "power and motor copper width"
	(condition "A.NetClass == 'Power' || A.NetClass == 'Rail' || A.NetClass == 'Motor'")
	(constraint track_width (min 0.8mm)))
# PWM and gate traces: 0.4-0.5 mm.
(rule "signal width"
	(condition "A.NetClass == 'Default'")
	(constraint track_width (min 0.4mm) (max 0.5mm)))
""")


# --------------------------------------------------------------------------
# PCB layout (board-local mm, origin = top-left corner of the outline)
# --------------------------------------------------------------------------
OX, OY = 100.0, 80.0  # outline position on the drawing sheet
EDGE = 0.5            # mini variant: copper to board edge 0.5 mm, so cut carefully along the line
W_SW, W_GB, W_RAIL, W_MOT, W_TOOTH, W_SIG = 1.0, 1.0, 1.0, 0.8, 0.8, 0.5
# Two rows (A: U1 + ch1-3, B: U2 + ch4-6). y offsets are relative to each row's 3.3 V rail-bus
# centre yr; x offsets in a cell are relative to the MOSFET centre xq. Derived for the standard
# SOT-23 / SOT-89 / 1206 lands (docs/implementation.md, mini variant).
CELL_Q, CELL_GB = 6.24, 12.94                           # MOSFET centre, GND-bus centre below the rail bus
YR_A = EDGE + W_RAIL / 2                                # row A rail bus along the top edge
YR_B = YR_A + CELL_GB + W_GB / 2 + 0.4 + W_RAIL / 2     # row B rail bus 0.4 mm under GND bus A
H = round(YR_B + CELL_GB + W_GB / 2 + EDGE, 2)
P = 8.1                                  # channel pitch (tooth to next cell's gate resistor 0.45 mm)
X1 = 18.45                               # first MOSFET centre, both rows
XS = [X1 + i * P for i in range(3)] * 2  # channels 1..3 (row A, rail A) and 4..6 (row B, rail B)
XSW = EDGE + W_SW / 2                    # SWITCHED_SYSTEM+ strip up the left edge, feeds both U tabs
XLINK = XS[2] + 3.26                     # GND link down the right edge, 0.4 mm clear of C14's pad
W = round(XLINK + W_GB / 2 + EDGE, 2)


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x + OX), mm(y + OY))


class Board:
    def __init__(self):
        self.b = pcbnew.BOARD()
        self.nets, self.fp = {}, {}

    def net(self, name):
        if name not in self.nets:
            n = pcbnew.NETINFO_ITEM(self.b, name)
            self.b.Add(n)
            self.nets[name] = n
        return self.nets[name]

    def place(self, ref, x, y, angle):
        p = next(q for q in PARTS if q["ref"] == ref)
        lib, name = p["fp"].split(":")
        path = f"{DIR}/EAS.pretty" if lib == "EAS" else f"{KI}/footprints/{lib}.pretty"
        f = pcbnew.FootprintLoad(path, name)
        f.SetFPID(pcbnew.LIB_ID(lib, name))
        f.SetReference(ref)
        f.SetValue(p["value"])
        f.GetField(pcbnew.FIELD_T_DATASHEET).SetText(p["ds"])
        f.GetField(pcbnew.FIELD_T_DESCRIPTION).SetText(p["desc"])
        f.SetPath(pcbnew.KIID_PATH("/" + p["uuid"]))
        f.SetOrientationDegrees(angle)
        f.SetPosition(V(x, y))
        self.b.Add(f)
        for pad in f.Pads():
            pad.SetNet(self.net(p["pins"][pad.GetNumber()]))
        self.fp[ref] = f

    def pc(self, ref, num="1"):
        """Board-local centre of a pad."""
        pad = next(pd for pd in self.fp[ref].Pads() if pd.GetNumber() == num)
        pos = pad.GetPosition()
        return round(pcbnew.ToMM(pos.x) - OX, 4), round(pcbnew.ToMM(pos.y) - OY, 4)

    def trk(self, netname, w, pts):
        pts = [p for i, p in enumerate(pts) if i == 0 or p != pts[i - 1]]
        for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
            t = pcbnew.PCB_TRACK(self.b)
            t.SetStart(V(x1, y1))
            t.SetEnd(V(x2, y2))
            t.SetWidth(mm(w))
            t.SetLayer(pcbnew.F_Cu)
            t.SetNet(self.net(netname))
            self.b.Add(t)

    def zone(self, netname, pts, prio, solid):
        z = pcbnew.ZONE(self.b)
        z.SetLayer(pcbnew.F_Cu)
        z.SetNet(self.net(netname))
        z.SetAssignedPriority(prio)
        z.SetLocalClearance(mm(0.5))
        z.SetMinThickness(mm(0.5))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL if solid else pcbnew.ZONE_CONNECTION_THERMAL)
        z.SetThermalReliefGap(mm(0.5))
        z.SetThermalReliefSpokeWidth(mm(0.5))
        z.SetIslandRemovalMode(pcbnew.ISLAND_REMOVAL_MODE_ALWAYS)
        ol = z.Outline()
        ol.NewOutline()
        for x, y in pts:
            ol.Append(mm(x + OX), mm(y + OY))
        self.b.Add(z)


def channel(bd, n, xq, yr):
    """One low-side channel. Topology (clockwise): rail on top, GND tooth on the right,
    PWM pad at the bottom, M- pad on the left, so the rail bus comes from above and the
    GND bus from below without any crossing (docs/implementation.md)."""
    r, gate, mneg, pwm = rail(n), f"GATE{n}", f"M{n}-", f"PWM{n}"
    cs, cd, jm, jp = f"C{n + 8}", f"C{n + 14}", f"J{11 + n if n <= 3 else 12 + n}", f"J{4 + n}"
    yq, ygb, xt = yr + CELL_Q, yr + CELL_GB, xq + 2.525  # MOSFET, GND bus, GND tooth (0.45 mm past the gate)
    bd.place(f"Q{n}", xq, yq, 180)                 # drain left, source top-right, gate bottom-right
    bd.place(f"D{n}", xq - 0.9375, yr + 2.10, 270)  # cathode on the rail bus, anode above the drain
    bd.place(cs, xq - 3.3275, yr + 2.05, 270)       # pin 1 (rail) up
    bd.place(cd, xq + 1.4525, yr + 2.05, 270)       # pin 1 (rail) up, pin 2 GND down to the source
    bd.place(f"R{n}", xq - 2.7, yq + 2.92, 0)       # pin 1 (PWM) left, pin 2 (gate) right
    bd.place(f"R{n + 6}", xq + 0.8, yq + 4.07, 270)  # pin 1 (gate) up, pin 2 GND down to the bus
    bd.place(jm, xq - 2.85, yq, 0)                  # M- pad touching the drain pad
    bd.place(jp, xq - 3.6, yq + 4.8, 0)             # PWM pad under the gate resistor

    # motor-minus node
    bd.trk(mneg, W_MOT, [bd.pc(f"D{n}", "2"), bd.pc(f"Q{n}", "3")])
    bd.trk(mneg, W_MOT, [bd.pc(f"Q{n}", "3"), bd.pc(jm)])
    bd.trk(mneg, W_MOT, [bd.pc(cs, "2"), (bd.pc(cs, "2")[0], yq)])
    # GND: local cap -> source -> tooth; pulldown -> bus
    s = bd.pc(f"Q{n}", "2")
    bd.trk(GND, W_MOT, [bd.pc(cd, "2"), (bd.pc(cd, "2")[0], s[1])])
    bd.trk(GND, W_MOT, [(bd.pc(cd, "2")[0], s[1]), s, (xt, s[1])])
    bd.trk(GND, W_MOT, [bd.pc(f"R{n + 6}", "2"), (bd.pc(f"R{n + 6}", "2")[0], ygb)])
    # gate network
    g, rpd1 = bd.pc(f"Q{n}", "1"), bd.pc(f"R{n + 6}", "1")
    bd.trk(gate, W_SIG, [g, (g[0], rpd1[1]), rpd1])
    bd.trk(gate, W_SIG, [bd.pc(f"R{n}", "2"), (bd.pc(f"R{n}", "2")[0], rpd1[1]), rpd1])
    bd.trk(pwm, W_SIG, [bd.pc(f"R{n}", "1"), bd.pc(jp)])
    return dict(rail_x=[bd.pc(cs, "1")[0], bd.pc(f"D{n}", "1")[0], bd.pc(cd, "1")[0]],
                rail_pads=[bd.pc(cs, "1"), bd.pc(f"D{n}", "1"), bd.pc(cd, "1")],
                tooth=(xt, s[1]), gnd_x=[bd.pc(f"R{n + 6}", "2")[0], xt])


def reg_block(bd, yr, u, cin, cbulk, cout, cp, jplus, rnet, corner):
    """HT7833 block at the left end of a row, tab facing the SW strip on the left edge.
    Returns SW-strip tap y, rail-bus tap x, GND-bus tap x."""
    ygb = yr + CELL_GB
    if corner:  # power corner, bottom-left of the board (ESP+ sits between PWR+ and GND)
        bd.place("J1", 2.0, yr + 12.19, 0)            # SWITCHED_SYSTEM+ input
        bd.place("J3", 4.9, yr + 11.04, 0)            # ESP_SW_BAT+
        bd.place("J2", 7.8, yr + 12.19, 0)            # GND input
        bd.place("J4", 10.7, yr + 12.44, 0)           # ESP GND
    bd.place(u, 3.5, yr + 2.45, 180)                  # tab touches the SW strip; pin 3 touches the rail bus
    bd.place(cin, 3.05, yr + 6.19, 0)                 # pin 1 (SW) on the strip
    bd.place(cbulk, 3.05, yr + 8.58, 0)
    bd.place(cout, 7.6, yr + 2.45, 270)               # between pin 3 (VOUT) and pin 1 (GND)
    bd.place(jplus, 10.15, yr + 3.0, 0)               # MOTOR_A+ / MOTOR_B+ (2.5 x 7.0)
    bd.place(cp, 9.4, yr + 7.95, 180)                 # 220 uF, + pad on the right under the + wire pad
    u3, u1 = bd.pc(u, "3"), bd.pc(u, "1")
    ci, cb = (bd.pc(cin, "1"), bd.pc(cin, "2")), (bd.pc(cbulk, "1"), bd.pc(cbulk, "2"))
    co1, co2, cpp, cpm = bd.pc(cout, "1"), bd.pc(cout, "2"), bd.pc(cp, "1"), bd.pc(cp, "2")
    ytab = bd.pc(u, "2")[1]
    taps = [ci[0][1], cb[0][1], ytab]
    bd.trk(SW, W_SW, [(XSW, ytab), (3.0, ytab)])      # into the tab strip
    bd.trk(SW, W_MOT, [(XSW, ci[0][1]), ci[0]])
    bd.trk(SW, W_MOT, [(XSW, cb[0][1]), cb[0]])
    # output: pin 3 -> cout -> rail bus; pin 1 -> cout -> cp(-) -> GND bus; input caps -> cp(-)
    bd.trk(rnet, W_RAIL, [u3, (co1[0], u3[1])])
    bd.trk(GND, W_MOT, [u1, (co2[0], u1[1])])
    bd.trk(GND, W_MOT, [co2, (co2[0], cpm[1])])
    bd.trk(GND, W_MOT, [ci[1], (ci[1][0], cpm[1]), cb[1]])
    bd.trk(GND, W_MOT, [(ci[1][0], cpm[1]), cpm])
    bd.trk(GND, W_GB, [cpm, (cpm[0], ygb)])
    bd.trk(rnet, W_RAIL, [bd.pc(jplus), (bd.pc(jplus)[0], cpp[1]), cpp])
    gnd = [cpm[0]]
    if corner:
        bd.trk(SW, W_SW, [bd.pc("J1"), bd.pc("J3")])
        for j in ("J2", "J4"):
            bd.trk(GND, W_GB, [(bd.pc(j)[0], ygb), bd.pc(j)])
            gnd.append(bd.pc(j)[0])
    rail_x = [u3[0], co1[0], bd.pc(jplus)[0]]
    for x, pad in zip(rail_x, (u3, co1, bd.pc(jplus))):
        bd.trk(rnet, W_RAIL, [(x, yr), pad])
    return taps, rail_x, gnd


def build_pcb():
    bd = Board()
    rows = ((YR_A, ("U1", "C1", "C3", "C5", "C7", "J11"), RA, False, (1, 2, 3)),
            (YR_B, ("U2", "C2", "C4", "C6", "C8", "J15"), RB, True, (4, 5, 6)))
    sw_taps = []
    for yr, blk, rnet, corner, chs in rows:
        taps, rail_x, gnd_x = reg_block(bd, yr, *blk, rnet, corner)
        sw_taps += taps
        for n in chs:
            c = channel(bd, n, XS[n - 1], yr)
            rail_x += c["rail_x"]
            gnd_x += c["gnd_x"]
            for (x, y) in c["rail_pads"]:
                bd.trk(rnet, W_RAIL, [(x, yr), (x, y)])
            bd.trk(GND, W_TOOTH, [c["tooth"], (c["tooth"][0], yr + CELL_GB)])
        # buses: a vertex at every tap so each stub ends on a segment end
        bd.trk(rnet, W_RAIL, [(x, yr) for x in sorted(set(rail_x))])
        bd.trk(GND, W_GB, [(x, yr + CELL_GB) for x in sorted(set(gnd_x) | {XLINK})])
    # Row A's GND bus joins row B's down the right edge, past the end of the rail B bus.
    bd.trk(GND, W_GB, [(XLINK, YR_A + CELL_GB), (XLINK, YR_B + CELL_GB)])
    bd.trk(SW, W_SW, [bd.pc("J1"), (XSW, bd.pc("J1")[1])] + [(XSW, y) for y in sorted(set(sw_taps), reverse=True)])

    # ---- copper zones ----
    bd.zone(GND, [(0, 0), (W, 0), (W, H), (0, H)], 0, solid=False)
    for yr in (YR_A, YR_B):  # solid SWITCHED_SYSTEM+ heat spreader at each regulator tab
        y0, y1 = yr - W_RAIL / 2, yr + 4.8
        bd.zone(SW, [(EDGE, y0), (4.3, y0), (4.3, y1), (EDGE, y1)], 1, solid=True)

    # Assembly drawing (F.Fab): no copper labels on this board (DEC-006), so every part body shows
    # its reference and a short value, and wire pads show a short function label.
    short = {"100": "100R", "100k": "100k", "1uF": "1u", "10uF": "10u", "10nF": "10n", "100nF": "100n", "220uF": "220u"}
    padlbl = {"J1": "PWR+", "J2": "GND", "J3": "ESP+", "J4": "EGND", "J11": "A+", "J15": "B+"}
    for ref, f in bd.fp.items():
        f.Value().SetVisible(False)
        if ref[0] == "J":
            t = pcbnew.PCB_TEXT(f)
            t.SetText(padlbl.get(ref, f.GetValue()))
            t.SetLayer(pcbnew.F_Fab)
            t.SetTextSize(pcbnew.VECTOR2I(mm(0.6), mm(0.6)))
            t.SetTextThickness(mm(0.1))
            t.SetPosition(f.GetPosition())
            t.SetTextAngleDegrees(90 if ref in ("J11", "J15") else 0)
            f.Add(t)
            continue
        for it in f.GraphicalItems():
            if it.GetClass() == "PCB_TEXT" and it.GetText() == "${REFERENCE}" and it.GetLayer() == pcbnew.F_Fab:
                it.SetText("${REFERENCE}" + (f"\n{short[f.GetValue()]}" if f.GetValue() in short else ""))
                it.SetTextSize(pcbnew.VECTOR2I(mm(0.5), mm(0.5)))
                it.SetTextThickness(mm(0.08))
                it.SetPosition(f.GetPosition())  # stock footprints park this text beside the body
                it.SetTextAngleDegrees(90 if round(f.GetOrientationDegrees()) % 180 else 0)
    legend = pcbnew.PCB_TEXT(bd.b)
    legend.SetText("PWR+ = SWITCHED_SYSTEM+ from the external master switch (never raw BAT)   GND = system ground\n"
                   "ESP+ = ESP_SW_BAT+ switched battery to the XIAO BAT pad (NOT 3.3 V)   EGND = ESP32 ground\n"
                   "A+ = motors 1-3 positive (HAPTIC_3V3_A)   B+ = motors 4-6 positive (HAPTIC_3V3_B)   Mn- = motor n negative\n"
                   "C7/C8: + pad marked   D1-D6: cathode bar towards the 3.3 V rail   R: 100R gate / 100k pulldown")
    legend.SetLayer(pcbnew.F_Fab)
    legend.SetTextSize(pcbnew.VECTOR2I(mm(0.9), mm(0.9)))
    legend.SetTextThickness(mm(0.12))
    legend.SetHorizJustify(pcbnew.GR_TEXT_H_ALIGN_LEFT)
    legend.SetVertJustify(pcbnew.GR_TEXT_V_ALIGN_TOP)
    legend.SetPosition(V(0, H + 2.0))
    bd.b.Add(legend)

    # No pour under part bodies: on an unmasked board a hidden sliver between a part's pads
    # is a solder-bridge risk. Regulators are exempt (their tab copper is the heat spreader).
    for ref, f in bd.fp.items():
        if ref[0] == "J" or ref in ("U1", "U2"):
            continue
        ka = pcbnew.ZONE(bd.b)
        ka.SetIsRuleArea(True)
        ka.SetDoNotAllowZoneFills(True)
        for setter in (ka.SetDoNotAllowTracks, ka.SetDoNotAllowVias, ka.SetDoNotAllowPads, ka.SetDoNotAllowFootprints):
            setter(False)
        ka.SetLayer(pcbnew.F_Cu)
        ka.Outline().Append(f.GetCourtyard(pcbnew.F_CrtYd))
        bd.b.Add(ka)

    # ---- outline ----
    s = pcbnew.PCB_SHAPE(bd.b)
    s.SetShape(pcbnew.SHAPE_T_RECT)
    s.SetStart(V(0, 0))
    s.SetEnd(V(W, H))
    s.SetLayer(pcbnew.Edge_Cuts)
    s.SetWidth(mm(0.1))
    bd.b.Add(s)

    missing = {p["ref"] for p in PARTS} - set(bd.fp)
    assert not missing, f"unplaced parts: {sorted(missing)}"
    pcbnew.SaveBoard(f"{DIR}/{PROJ}.kicad_pcb", bd.b, True)
    print(f"board {W} x {H} mm = {W * H:.0f} mm^2, {len(bd.fp)} footprints")


if __name__ == "__main__":
    write_libs()
    write_schematic()
    build_pcb()
    write_project()  # after SaveBoard so the rules file is ours
