# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.3] - Forced isolation, optimization, scale and fiducials, solder mask, units and a preview

- **Forced isolation** (off by default, `forced_isolation = yes`): a path is also laid along the middle of every gap that the finest tool does not fit in, as long as the gap is not narrower than `forced_min_gap_mm`; every place is reported with its coordinates and its gap, the narrower gaps are listed and left alone, and `isolate` still refuses to write for those unless `--force` is given. With the setting off, the places are only reported, as before. The tool is wider than such a gap, so it touches the copper on both sides there; that is only warned about, with the place.
- **Lighter paths** (`optimize_tolerance_mm` = 0.01 and `spike_max_mm` = 0.02 by default, 0 switches each off; `spike_angle_deg`): spikes where the path turns back on itself are taken out, repeated points are joined and points on a straight line within the tolerance become one move; arcs are not fitted, by decision: the paths stay straight moves.
- **Scale and correction of the board:** `scale_x` and `scale_y` scale the work about the centre of the board (the centre stays where it is, and a scale of 1 changes nothing), and pairs of reference points, given relative to that centre (`--fiducial "DESIGN_X,DESIGN_Y,MEASURED_X,MEASURED_Y"`, two or more times) give a similarity (two pairs) or a least-squares affine fit (three or more) applied to every path, hole and outline; a fit that turns the board over, collapses it, scales an axis more than `fiducial_max_scale_deviation` away from 1 or leaves a mark more than `fiducial_max_residual_mm` from where it should be is refused with the reason.
- **Solder mask:** `mask COPPER.gtl OUT.gts` writes the openings as a Gerber layer, from the mask layer of the design (`--mask-layer`) or from the flashed pads of the copper, each grown by `mask_margin_mm` (negative shrinks); tracks stay covered. Silkscreen is not produced.
- **Units:** a length in the settings file or on the command line may carry `mm`, `mil` or `in` (`--fine-tool-mm 8mil`), a bare number is in `input_units`, the G-code can be written in inches (`output_units = inch`, `G20`) and the lengths printed on the screen follow `report_units`.
- **Preview without a window:** `preview LAYER.gtl [OUT.svg|OUT.txt]` draws the copper and every pass (as wide as its tool) as an SVG drawing, or as text with a key (`preview_cols`).
- **Fixed:** a layer with a coordinate that is not a number is refused with the number of the shape instead of being drawn.
- **91 tests** run on a computer, with damaged input; none of this has run on a machine, and the correction has not been compared with real measurements.

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
