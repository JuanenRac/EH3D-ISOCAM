# EH3D-ISOCAM: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A Voronoi strategy for the paths | The paths come from the exact distance on a grid and are exact to half a cell; a Voronoi strategy is a second way to trace them | the grid method has been compared with milled boards |
| Arcs in the plots and the drawings | Arcs are fitted in the G-code only; a plot and a DXF drawing stay in straight pieces, which every reader takes | a machine that reads a plot needs fewer points |
| Arcs by default | `arc_fit_tolerance_mm` is 0 so that the programs stay as they were; no controller has run a program with arcs | a machine has run one and the tolerance is known |
| Stamped pads in the silkscreen | Only strokes and regions are engraved; a pad drawn with one stamp is left out and counted | a person says how a stamped mark should be engraved |
| Buttons for the checks and the report in the window | The window makes the checks and the report of the job without a screen, but nobody has looked at the window, so no button was added blind | the window has been looked at on a screen and finished |
| The tools of the crib read from its JSON file | Only the CSV table of the crib program is read; the window does not use the crib | the crib program and this one agree on one exchange file |
| The correction from reference points compared with a camera | The correction moves the paths and keeps the size of the tool offset; it has not been compared with measurements | the vision program of the suite measures marks on a board |
| Boards that were really milled | No board has been milled with these paths, so no tolerance has been confirmed | a person mills a board and measures it |
| Design rules against the board edge and the solder mask | The rules check copper and holes only, and the ring of a hole takes the pad as drawn | the rules of a maker are given to check against |
