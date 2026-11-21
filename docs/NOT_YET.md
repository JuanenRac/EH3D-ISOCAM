# EH3D-ISOCAM: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A Voronoi strategy for the paths | The paths come from the exact distance on a grid and are exact to half a cell; a Voronoi strategy is a second way to trace them | the grid method has been compared with milled boards |
| Fitting arcs to the paths | A curve is many short moves by decision, which keeps the program simple and the check exact | a controller needs fewer lines or a test shows an arc fit is exact within the tolerance |
| A silkscreen layer | There is no layer for it in the job; the solder mask is the only extra layer written | a person asks for it |
| The checks of the command line in the window | The window shows the parameters and the dry run; `check`, `drc`, `info`, `preview` and `rubout-area` stay on the command line | the window has been looked at on a screen and finished |
| The tools of the crib read from its JSON file | Only the CSV table of the crib program is read; the window does not use the crib | the crib program and this one agree on one exchange file |
| The correction from reference points compared with a camera | The correction moves the paths and keeps the size of the tool offset; it has not been compared with measurements | the vision program of the suite measures marks on a board |
| Boards that were really milled | No board has been milled with these paths, so no tolerance has been confirmed | a person mills a board and measures it |
| Design rules against the board edge and the solder mask | The rules check copper and holes only, and the ring of a hole takes the pad as drawn | the rules of a maker are given to check against |
