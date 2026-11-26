# EH3D-ISOCAM: status

## Current state

EH3D-ISOCAM is a working program on a computer: a library (`eh3d_isocam`), a command line (`eh3d-isocam`) and a window (built with `--features gui`). It isolates, drills and routes the layers of the example boards, checks the design rules and the limits of a machine table, and walks its G-code against a simulated machine. The suite of tests runs on Windows and Linux in the CI. It also writes arcs where a run of short moves lies on a circle, engraves a silkscreen layer and tells what to look at before a job (`report`). The window has not been looked at on a screen; its logic makes the checks and the report of the job, but the buttons that call them are not written and it does not cover `info`, `rubout-area` and the other commands; no board has been milled and the files have not been read by other programs.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 54 requirements: 44 implemented, 6 prototype, 4 planned and 0 validated. By progress: 44 done, 6 partial, 1 pending and 3 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether `arc_fit_tolerance_mm` should be on by default; today it is 0, which keeps every piece straight, and no controller has run a program with arcs.
* Where the buttons for the checks and the report of the job should go in the window.
* How the stretches left out for a narrower tool should be decided when the grid is not fine enough.
* Whether the stamped pads of a silkscreen layer should be engraved too, and how.

## What it needs from other repositories

* `EH3D-CORE-LIBS`, `EH3D-GERBERVIEWER` and `EH3D-COMMON`, as sibling folders at the versions named in the workflows (EH3D-COMMON at the tag v0.3.3; EH3D-GERBERVIEWER at the tag v0.1.0).
* The tool crib program for the CSV table, and the autolevel program for the height map.
* A milling machine, a board and a person, for the only validation that counts.

## Known technical debt and omitted tests

* The paths are exact to half a cell, not to the curve itself, an arc stays within `arc_fit_tolerance_mm` of those paths, and a forced path lets a tool wider than the gap touch the copper on both sides.
* The solder mask follows the pads as drawn and a negative margin does not shrink a region.
* The design rules do not round off the corners of a square pad and do not check the copper against the board edge or the solder mask.
* Drill files are written in whole mils, and the files written were only read back by the readers of this suite.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a machine, a board or a screen are not written: the register names each as `planned:` with the test that will show it.
