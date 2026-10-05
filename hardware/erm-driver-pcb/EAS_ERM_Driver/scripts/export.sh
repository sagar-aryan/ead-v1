#!/usr/bin/env bash
# Rebuild the project from scripts/build.py, verify it, and write every deliverable to output/.
# Stops at the first failing step (ERC error, DRC/parity violation, or check.py failure).
# Usage: bash scripts/export.sh
set -euo pipefail
cd "$(dirname "$0")/.."
P=EAS_ERM_Driver
O=output
rm -rf "$O" && mkdir -p "$O/gerbers"
quiet() { "$@" 2>&1 | grep -v "property.h" || true; }   # pcbnew prints a harmless enum assert

quiet python3 scripts/build.py
kicad-cli sch erc --severity-all --exit-code-violations -o "$O/erc.rpt" "$P.kicad_sch"
# --save-board keeps the refilled zones, so every plot below contains the GND pour.
quiet kicad-cli pcb drc --schematic-parity --refill-zones --save-board --severity-all \
    --exit-code-violations -o "$O/drc.rpt" "$P.kicad_pcb"
grep -q "Found 0 DRC violations" "$O/drc.rpt" && grep -q "Found 0 unconnected pads" "$O/drc.rpt" \
    && grep -q "Found 0 Footprint errors" "$O/drc.rpt" || { echo "DRC report not clean"; exit 1; }
python3 scripts/check.py 2>/dev/null > "$O/check.txt" || { cat "$O/check.txt"; exit 1; }

kicad-cli sch export pdf -o "$O/${P}_schematic.pdf" "$P.kicad_sch"
kicad-cli sch export bom --fields "Reference,Value,Footprint,Description,QUANTITY" \
    --labels "Refs,Value,Footprint,Description,Qty" --group-by "Value,Footprint" \
    -o "$O/${P}_bom.csv" "$P.kicad_sch"
quiet kicad-cli pcb export gerbers --layers F.Cu,Edge.Cuts -o "$O/gerbers/" "$P.kicad_pcb"
# Toner transfer: print at 100 % (no "fit to page"); the image is mirrored so it reads correctly
# after being ironed face-down onto the copper.
quiet kicad-cli pcb export pdf --layers F.Cu,Edge.Cuts --mode-single --black-and-white --mirror \
    -o "$O/${P}_TONER_F.Cu_MIRRORED_print_1to1.pdf" "$P.kicad_pcb"
quiet kicad-cli pcb export pdf --layers F.Cu,Edge.Cuts --mode-single --black-and-white \
    -o "$O/${P}_F.Cu_top_view_fitcheck_1to1.pdf" "$P.kicad_pcb"
for s in 1 3; do
    quiet kicad-cli pcb export pdf --layers F.Fab,Edge.Cuts --mode-single --black-and-white \
        --sketch-pads-on-fab-layers --scale $s -o "$O/${P}_assembly_${s}to1.pdf" "$P.kicad_pcb"
done

# PNG previews (cropped to the drawing) for quick review in the docs.
python3 - "$O" "$P" <<'EOF'
import subprocess, sys
from PIL import Image, ImageOps
o, p = sys.argv[1:3]
for pdf, png in ((f"{p}_F.Cu_top_view_fitcheck_1to1.pdf", "preview_copper.png"),
                 (f"{p}_assembly_3to1.pdf", "preview_assembly.png"), (f"{p}_schematic.pdf", "preview_schematic.png")):
    subprocess.run(["pdftoppm", "-r", "600" if "copper" in png else "200", "-png", "-singlefile",
                    f"{o}/{pdf}", f"{o}/tmp"], check=True)
    im = Image.open(f"{o}/tmp.png").convert("L")
    box = ImageOps.invert(im).getbbox()
    im.crop((box[0] - 20, box[1] - 20, box[2] + 20, box[3] + 20)).save(f"{o}/{png}")
subprocess.run(["rm", f"{o}/tmp.png"])
EOF
ls -1 "$O" "$O/gerbers"
