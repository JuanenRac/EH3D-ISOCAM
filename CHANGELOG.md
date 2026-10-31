# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.2] - Excellon, several tools, direction and order of the cuts, alignment holes and design rules

- **The order of the paths is optimized by default** (`ordering = optimized`: nearest first and then shortened by turning stretches round, with the limit of `optimize_max_paths`); `ordering = nearest` and `ordering = original` still choose the others.

- **Holes and board outline:** drill files are read (Bungard files in mils with the reader of the Bungard formats, metric and decimal files with the reader of the suite) and written back in whole mils; a board outline is read from a Gerber outline layer, a DXF drawing or an HP-GL plot file, strokes drawn one by one are joined into closed paths, and the path of the tool round the board and inside its cutouts is written as HP-GL or as a routing Excellon file (`drills` and `route`).
- **Several tools and the leftover copper:** up to four tools, the widest cutting first; a stretch of a narrower tool that a wider one has already cut along is left out and the rest is kept as open runs; rings of a wider tool, each overlapping the one before, clear the copper that is left beyond the passes; the G-code stops for a tool change when the width changes and the plot file gives each tool its pen.
- **Direction, start and order of the cuts:** the paths are traced with the copper on the right of the tool; the direction can be climb, conventional, clockwise or counterclockwise, each closed path can start at its nearest point, its lowest point or where it was traced, the tool starts from a chosen point, and the paths are put in the order they were traced, nearest first, or shortened further by turning stretches of the order round; `info` shows how many paths go each way and the travel through the air.
- **Both sides, panels and alignment holes:** the layer, the holes and the outline are turned over, turned by quarter turns and repeated on a panel together; alignment holes are put where a turned-over board finds them again.
- **Design rules:** copper narrower than the rule, pieces of copper closer than the rule or than the tool is wide, and holes with too little copper round them, each with its place (`drc`).
- **One settings file:** every tuned value is a setting with a default, read from a `key = value` file (`--config`, `examples/isocam.cfg`, printed by `config`) or given as an option with the same name.
- **Fixed:** a region with fewer than three points no longer stops the drawing of a layer.
- **60 tests** run on a computer, with damaged input; the Rust version needed is now 1.88 (the Bungard formats).

## [0.0.1] - Isolation milling of copper from a Gerber layer: exact offsets, several passes and gap checks

- **The copper on a grid:** pads, tracks and regions drawn on cells of the chosen size, with a limit on the memory it may take.
- **The exact distance:** the Euclidean distance from every cell to the copper (checked against a brute force), and the paths of the passes traced where it equals the radius of the tool, one stepover farther for each further pass.
- **The gap check:** pieces of copper that are closer than the tool is wide are listed; `isolate` refuses to write unless `--force` is given.
- **The G-code:** lifts between paths, plunges to the depth chosen and cuts the nearest path next; an output file that ends in `.plt` gets an HP-GL plot file with one pen for each pass, for the routing program of the Bungard machines.
- **The command line** `eh3d-isocam` with `isolate` and `check`, and a sample layer.
- **17 tests** run on a computer, with damaged input.
