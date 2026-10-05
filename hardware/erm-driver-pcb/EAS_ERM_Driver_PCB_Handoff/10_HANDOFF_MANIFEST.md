# Handoff Manifest

This folder is intentionally self-contained for the PCB-design agent.

Start with:
1. `00_README.md`
2. `05_KICAD_AGENT_TASK.md`

Then use:
- `01_SYSTEM_ARCHITECTURE.md` for top-level power/blocks.
- `02_SCHEMATIC_AND_NETLIST.md` for exact electrical connectivity.
- `03_COMPONENTS_AND_FOOTPRINTS.md` for footprints and purchased parts.
- `04_LAYOUT_AND_EMI_RULES.md` for one-layer routing and thermal/EMI constraints.
- `06_BOM.md` for the populated BOM and pad map.
- `07_VALIDATION_AND_TEST.md` for bring-up.
- `08_DECISION_LOG.md` for locked decisions.
- `09_SOURCE_NOTES.md` for manufacturer references and inventory.

## Agent completion criteria
The task is complete only when the agent has produced a KiCad project that is:
- electrically connected according to the locked netlist;
- single-layer copper;
- drill-free;
- connector-free;
- compact;
- hand-solderable;
- validated by ERC/DRC;
- documented with final pad mapping and power rails.

Do not reopen already-set architectural questions unless a physical manufacturing or safety contradiction is discovered. If that happens, report the exact contradiction and identify the minimum required change.
