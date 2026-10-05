#!/usr/bin/env python3
"""Tile the verified board into a toner-transfer sheet with a printed scale check.

Reads EAS_ERM_Driver_tight.kicad_pcb (never writes to it) and produces
  output/EAS_ERM_Driver_tight_PANEL.kicad_pcb              (throwaway, for plotting)
  output/EAS_ERM_Driver_tight_TONER_PANEL_<n>up_MIRRORED_print_1to1.pdf

The PDF is mirrored, so print it at 100 % (no "fit to page") and iron it face down.
The ruler and its labels sit outside the board copies: measure them first, then cut
that strip off before ironing. Label text is stored mirrored so that it reads
correctly after the plot's mirror.

The sheet is built once to measure its extent, then rebuilt centred on the page, and
the finished PDF is rasterized so that the ruler and outline lengths are verified, not
assumed.

Usage: python3 scripts/panel.py [columns] [rows] [gap mm]   (default 3 x 2 = 6 copies, 30 mm apart)
"""
import os
import subprocess
import sys

import pcbnew
from PIL import Image

Image.MAX_IMAGE_PIXELS = None
DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROJ = "EAS_ERM_Driver_tight"
NAME = PROJ.replace("EAS_ERM_Driver", "").strip("_") or "single row"
COLS = int(sys.argv[1]) if len(sys.argv) > 1 else 3
ROWS = int(sys.argv[2]) if len(sys.argv) > 2 else 2
# mm between copies. Half of it ends up as paper on each side of a cut-out copy, to be
# folded around the copper clad while ironing, so this is deliberately wider than a saw kerf.
GAP = float(sys.argv[3]) if len(sys.argv) > 3 else 30.0
RULER = 100.0    # mm reference line
RULER_V = 50.0   # mm reference line, the other axis (printers can scale X and Y differently)
MARGIN_X, RULER_Y, GRID_Y = 18.0, 18.0, 36.0
PAGE_W, PAGE_H = 297.0, 210.0   # A4 landscape, the board's page setting
EDGE_W = 0.1     # outline line width, so ink is wider than the cut dimension

mm = pcbnew.FromMM
PANEL = f"{DIR}/output/{PROJ}_PANEL.kicad_pcb"
PDF = f"{DIR}/output/{PROJ}_TONER_PANEL_{COLS * ROWS}up_MIRRORED_print_1to1.pdf"


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def build(sx, sy):
    """Write the panel board, with everything shifted by (sx, sy) mm. Returns board size."""
    b = pcbnew.LoadBoard(f"{DIR}/{PROJ}.kicad_pcb")
    edge = next(d for d in b.GetDrawings() if d.GetLayer() == pcbnew.Edge_Cuts)
    bb = pcbnew.BOX2I(edge.GetStart(), edge.GetEnd() - edge.GetStart())
    bb.Normalize()
    bw, bh = pcbnew.ToMM(bb.GetWidth()), pcbnew.ToMM(bb.GetHeight())
    x0, y0 = pcbnew.ToMM(bb.GetLeft()), pcbnew.ToMM(bb.GetTop())

    originals = list(b.GetFootprints()) + list(b.GetTracks()) + list(b.Zones()) + list(b.GetDrawings())
    # Move the original copy to the grid origin, then duplicate it into the remaining cells.
    for it in originals:
        it.Move(V(MARGIN_X + sx - x0, GRID_Y + sy - y0))
    for r in range(ROWS):
        for c in range(COLS):
            if not r and not c:
                continue
            off = V(c * (bw + GAP), r * (bh + GAP))
            for it in originals:
                try:
                    d = it.Duplicate(False)   # FOOTPRINT takes addToParentGroup
                except TypeError:
                    d = it.Duplicate()       # other board items take no argument
                d.Move(off)
                b.Add(d)

    # ---- scale check, drawn on the drawings layer (plotted, but outside every board copy) ----
    layer = pcbnew.Dwgs_User

    def line(x1, y1, x2, y2, w=0.25):
        s = pcbnew.PCB_SHAPE(b)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetStart(V(x1 + sx, y1 + sy))
        s.SetEnd(V(x2 + sx, y2 + sy))
        s.SetLayer(layer)
        s.SetWidth(mm(w))
        b.Add(s)

    # Mirrored text extends away from its anchor in the opposite direction, so RIGHT
    # justify here is what keeps a label inside the page and left-aligned once plotted.
    def text(txt, x, y, size=2.2, just=pcbnew.GR_TEXT_H_ALIGN_RIGHT):
        t = pcbnew.PCB_TEXT(b)
        t.SetText(txt)
        t.SetLayer(layer)
        t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
        t.SetTextThickness(mm(size * 0.15))
        t.SetPosition(V(x + sx, y + sy))
        t.SetHorizJustify(just)
        t.SetMirrored(True)  # the plot is mirrored; store mirrored so it prints readable
        b.Add(t)

    line(MARGIN_X, RULER_Y, MARGIN_X + RULER, RULER_Y, 0.3)          # 100 mm reference
    for i in range(11):                                              # ticks every 10 mm
        x = MARGIN_X + i * RULER / 10
        line(x, RULER_Y, x, RULER_Y - (3.0 if i % 5 == 0 else 1.5))
    line(MARGIN_X - 6.0, GRID_Y, MARGIN_X - 6.0, GRID_Y + RULER_V, 0.3)  # 50 mm reference
    for i in range(6):
        y = GRID_Y + i * RULER_V / 5
        line(MARGIN_X - 6.0, y, MARGIN_X - 6.0 + (3.0 if i % 5 == 0 else 1.5), y)

    # End labels are swapped, because the plot mirror puts the far tick on the left.
    text(f"{RULER:.0f} mm", MARGIN_X, RULER_Y + 6.0, 2.0, pcbnew.GR_TEXT_H_ALIGN_CENTER)
    text("0", MARGIN_X + RULER, RULER_Y + 6.0, 2.0, pcbnew.GR_TEXT_H_ALIGN_CENTER)
    text(f"MEASURE TICK 0 TO TICK {RULER:.0f}: exactly {RULER:.1f} mm", MARGIN_X, RULER_Y - 5.0)
    text("Print at 100% - turn OFF 'fit to page'", MARGIN_X, RULER_Y - 9.5, 2.0)
    # Above the vertical ruler: below it, the mirror drops the label onto a board copy.
    text(f"{RULER_V:.0f} mm", MARGIN_X - 6.0, GRID_Y - 3.0, 2.0, pcbnew.GR_TEXT_H_ALIGN_CENTER)
    ybot = GRID_Y + ROWS * bh + (ROWS - 1) * GAP + 6.0
    text(f"EAS ERM driver ({NAME}): board {bw:.2f} x {bh:.2f} mm, {COLS * ROWS} copies, MIRRORED",
         MARGIN_X, ybot, 2.0)
    text("Cut the ruler strip off before ironing.", MARGIN_X, ybot + 4.0, 2.0)

    # Extent of what is actually plotted: the outlines and the ruler layer. Copper lies
    # inside the outlines, and the F.Fab legend text is not plotted, so both are excluded.
    box = pcbnew.BOX2I()
    for d in b.GetDrawings():
        if d.GetLayer() in (pcbnew.Edge_Cuts, layer):
            box.Merge(d.GetBoundingBox())
    pcbnew.SaveBoard(PANEL, b, True)
    return bw, bh, [pcbnew.ToMM(v) for v in (box.GetLeft(), box.GetTop(), box.GetRight(), box.GetBottom())]


def plot(pdf):
    subprocess.run(["kicad-cli", "pcb", "export", "pdf", "--layers",
                    f"F.Cu,Edge.Cuts,{pcbnew.BOARD.GetStandardLayerName(pcbnew.Dwgs_User)}",
                    "--mode-single", "--black-and-white", "--mirror", "--scale", "1",
                    "--check-zones", "-o", pdf, PANEL], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def measure(pdf, dpi=600):
    """Returns (ink bbox mm, longest horizontal ink run mm, set of 30-40 mm runs)."""
    png = f"{DIR}/output/.panel_measure"
    subprocess.run(["pdftoppm", "-r", str(dpi), "-png", "-singlefile", pdf, png], check=True)
    im = Image.open(png + ".png").convert("L")
    px, (w, h), k = im.load(), im.size, 25.4 / dpi
    cols = [x for x in range(w) if any(px[x, y] < 128 for y in range(0, h, 2))]
    rows = [y for y in range(h) if any(px[x, y] < 128 for x in range(0, w, 2))]
    longest, edges = 0.0, set()
    for y in range(h):
        run = 0
        for x in range(w):
            if px[x, y] < 128:
                run += 1
            else:
                longest = max(longest, run * k)
                if 30 < run * k < 40:
                    edges.add(round(run * k, 2))
                run = 0
    im.close()
    os.remove(png + ".png")
    return (cols[0] * k, rows[0] * k, cols[-1] * k, rows[-1] * k), longest, edges


def main():
    bw, bh, (u0, v0, u1, v1) = build(0.0, 0.0)
    # The plot mirrors about the page centre, so centring the layout centres the print.
    sx = (PAGE_W - (u1 - u0)) / 2 - u0
    sy = (PAGE_H - (v1 - v0)) / 2 - v0
    build(sx, sy)
    plot(PDF)
    (x0, y0, x1, y1), longest, edges = measure(PDF)
    edge = min(edges, key=lambda e: abs(e - (bw + EDGE_W))) if edges else 0.0
    print(f"{COLS} x {ROWS} = {COLS * ROWS} copies of {bw:.2f} x {bh:.2f} mm, {GAP:.0f} mm apart ({GAP / 2:.0f} mm of paper per side)")
    print(f"printed area  x [{x0:.1f}, {x1:.1f}], y [{y0:.1f}, {y1:.1f}] mm "
          f"on {PAGE_W:.0f} x {PAGE_H:.0f} mm")
    print(f"ruler line    {longest:.2f} mm of ink = {RULER:.1f} mm tick to tick + 0.3 mm end caps")
    print(f"board outline {edge:.2f} mm of ink = {bw:.2f} mm cut line + {EDGE_W} mm line width")
    ok = (abs(longest - (RULER + 0.3)) < 0.15 and abs(edge - (bw + EDGE_W)) < 0.15
          and x0 > 5 and y0 > 5 and x1 < PAGE_W - 5 and y1 < PAGE_H - 5)
    print("SCALE OK (1:1), nothing clipped" if ok else "CHECK FAILED")
    print(f"wrote {PDF}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
