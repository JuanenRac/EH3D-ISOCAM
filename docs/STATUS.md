# EH3D-ISOCAM: status

## Current state

EH3D-ISOCAM is a working program on a computer: a library (`eh3d_isocam`), a command line (`eh3d-isocam`) and a window (built with `--features gui`). It isolates, drills and routes the layers of the example boards, checks the design rules and the limits of a machine table, and walks its G-code against a simulated machine. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen and does not cover the commands `check`, `drc`, `info`, `preview` and `rubout-area`; no board has been milled and the files have not been read by other programs.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 45 requirements: 34 implemented, 4 prototype, 7 planned and 0 validated. By progress: 34 done, 4 partial, 4 pending and 3 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the paths should follow the exact curve instead of half a cell, and whether arcs should be fitted; today a curve is many short moves.
* Which of the checks of the command line should come into the window first.
* How the stretches left out for a narrower tool should be decided when the grid is not fine enough.
* Whether a silkscreen layer belongs here or in the viewer.

## What it needs from other repositories

* `EH3D-CORE-LIBS`, `EH3D-GERBERVIEWER` and `EH3D-COMMON`, as sibling folders at the versions named in the workflows.
* The tool crib program for the CSV table, and the autolevel program for the height map.
* A milling machine, a board and a person, for the only validation that counts.

## Known technical debt and omitted tests

* The paths are exact to half a cell, not to the curve itself, and a forced path lets a tool wider than the gap touch the copper on both sides.
* The solder mask follows the pads as drawn and a negative margin does not shrink a region.
* The design rules do not round off the corners of a square pad and do not check the copper against the board edge or the solder mask.
* Drill files are written in whole mils, and the files written were only read back by the readers of this suite.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a machine, a board or a screen are not written: the register names each as `planned:` with the test that will show it.
