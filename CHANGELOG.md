# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **The G-code starts with the standard header:** the generator and its version, the units, the file the work came from with its SHA-256, and a path with a coordinate that is not a number is refused.
- **An output that is also an input is refused:** the result is never written over the layer, the outline, the holes, the mask or the apertures of the run.
- **A plot beyond the range of the format is refused:** a coordinate over 32767 units (819 mm) fails with a message instead of wrapping round.
- **The units of a drill file can be given:** `--units mm|inch`; a file that does not say its units and whose holes are tiny is refused without it.
- **The outline of separate strokes is joined with an index:** fifty thousand segments take moments.
- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **Limited reading and arc tolerance:** files are read with the size limit of the reader before they are loaded (`max_input_mb`, 64 MiB by default, refusal in eight languages) and `arc_tolerance_mm` sets how closely arcs are followed.
- **One shift for the plots of a board:** with the shift on, the isolation plot and the hole plot are moved by the same amount (the box of the board less `hpgl_margin_mm`), so they no longer end up a fraction of a millimetre apart.
- **Spindle wait:** `spindle_wait_s` writes a `G4` wait after each spindle start in the G-code.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.

## [0.0.7] - CI hardening and shared validator

- **Faster, bounded CI:** the Cargo build is cached, a run that a newer push supersedes is cancelled, every job has a time limit, and the Rust jobs live in their own workflow that is skipped when only documents or images change.
- **A fixed Rust toolchain:** `rust-toolchain.toml` names the compiler version (with rustfmt and clippy) that the CI installs, so a new stable release cannot turn the lints red without a change here.
- **Monthly dependency updates:** Dependabot opens one grouped pull request a month for the GitHub Actions.
- **A line for every change:** a pull request that changes code must add a line under `[Unreleased]` of this file (`tools/changelog_pr_check.py`; the label `no-changelog` skips it).
- **Shared checks updated:** the validator of the suite checks the text of the banner against the manifest and reads the files that git tracks.
- **Tool numbers above 255 are no longer merged:** a drill file with tools numbered above 255 gets them renumbered to free numbers so two tools never become one, and the reader reports a `tool-number-too-large` notice for each.
- **More tests:** the limits of the job, the G-code with tool changes and in inches, the HP-GL plots with pens, speeds and the style of the original program, the DXF drawings, the kind of a file and the reading of drill files.

## [0.0.6] - Library parts moved to the shared crates and reading with limits

- **The pure CAM comes from the shared `eh3d-cam` crate:** the copper on a grid, the passes, the checks, the order and the shortening of paths, the fiducials, the board transforms, the solder mask openings and old-style Gerber moved to the shared libraries and are re-exported with the same public paths, so the library and the command line do not change.
- **Units, distance transform and contour lines come from `eh3d-raster`:** the three modules moved to the shared library of the suite; `eh3d_isocam::units`, `edt` and `contour` still work as before.
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge. The manifest now names its three siblings in `depends_on`.

- **Files are read with a size limit and an encoding check:** `--max-bytes N` refuses a file over the limit (256 MiB by default), and text that is not UTF-8 is read as Latin-1 with a warning that carries the code `input-not-utf8`.
- **The notices of the Gerber reader are shown by their stable code** on the error output, and `--hide CODE[,CODE]` leaves out the notices of those codes (a misspelt code is an error).
- **No note when nothing changed:** the note about the lighter paths and the one about the bridges of the cut are printed only when they changed something.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.

## [0.0.5] - A plot that keeps the origin of the board, the cut round the outline and a plain drill file

- **A plot that keeps the origin of the board** (`hpgl_shift_negative`, yes by default as before): with `no`, the coordinates of a plot (and of a plot of holes) stay as they are, negative ones included, so that a plot and a drill file made from the same design share one origin.
- **The cut round the outline in the plot of `isolate`** (`plot_outline = yes`, `--outline FILE`, `hpgl_outline_pen`, `hpgl_isolation_pen`): the cut of the tool of `route_tool_mm` round the outline comes first, on its own pen, followed by the isolation on its pen; 0 for a pen takes the usual one (the outline after the others).
- **A plain drill file** (`ncd_isocam_style`, `ncd_digits`; both off by default): the drill file is written as the Bungard programs do (`M48` and `%`, tools by number only, a repeated coordinate left out, CR LF, `M00` at the end), and the coordinates can be filled with zeros in front (five digits for whole mils).
- **A drill file with no header line is known:** a `%`, tools by number and the holes are taken for Excellon by `kind` and by the readers.
- **Tests for turning the holes over about the middle of the outline** with `--mirror vertical` (a board milled on its other side finds its holes where they were).

## [0.0.4] - Plots as the original program wrote them, tolerance for pads, filling by contours and DXF output

- **Plots as the original program wrote them** (`hpgl_isocam_style`, on by default; off keeps the compact form): every point of a path is its own `PA`, lines are ended with CR LF after about 75 columns, `IN`, `VS` and `SP` stand on lines of their own, and the plot ends with `PU;` and `PA0,0;SP;`.
- **Tolerance only for pads** (`conflict_tolerance_scope = pads`; `all` by default, as before): a gap narrower than the tool by up to the tolerance is accepted only when a pad is beside it, every other gap is judged against the whole tool, and the thin-copper warning looks at pads alone.
- **Filling by contours** (`rubout-area OUT [LAYER.gtl] --contour FILE`, or `--area`; `rubout_fill_margin_mm`): the closed paths of a file are filled even-odd (an inner path is a hole) with back-and-forth lines of `rubout_lines_tool_mm`; with a layer the lines are cut to the copper, the whole tool lying on copper plus the margin.
- **The kind of an input file** (`kind FILE...`): Gerber with a format statement, old-style Gerber, Excellon, HP-GL, DXF or unknown; a layer that is really a drill file, a plot or a drawing (or a drill file that is a Gerber layer) is refused with a plain message that names what it is and what to do.
- **DXF output:** `isolate` and `rubout-area` write a DXF drawing (polylines in millimetres, one layer for each pass) when the output name ends in `.dxf`.
- **Plot of the holes with pens** (`drills IN OUT.plt`, `hpgl_pen_map = tool:pen, ...`) and **pen speed** (`hpgl_pen_speed_cm_s`, 0 by default = the plot is as before and the machine uses its own speed): each drill tool is a pen, the tools the map leaves out take the lowest free pens, and a non-zero speed adds `VS` to any plot.
- **Bridges on the cut of the board** (`tab_count`, `tab_width_mm`, `tab_min_length_mm`; 0 bridges by default): `route` opens each closed path into runs with a gap at every bridge, so the board stays held in the sheet; the cut stops half a bridge and half a tool short of the middle of the bridge, a path too short for a bridge is left closed and said so, and a bridge of less than 0.1 mm or pieces of less than two tools are refused.
- **One net by itself** (`isolate --net X,Y`, `net_tool_mm`): only the copper joined to the point (up to half a millimetre from it) is isolated, with its own tool.
- **Clearing an area with parallel lines** (`rubout-area OUT --area X0,Y0,X1,Y1`, `rubout_lines_tool_mm`, `rubout_lines_direction`, and `rubout_overlap`): back-and-forth lines whose tool reaches the edge of the area.
- **Six alignment holes and a special pattern** (`alignment_holes` 2, 3 or 6; `alignment_pattern` normal or special; `alignment_special_a_mm` 40 and `alignment_special_b_mm` 60 by default): six is the pair and the four corners outside the board; special puts the pair about the middle of the board, and six adds four more level with them.
- **A warning for a tolerance that is high against thin copper** (`tolerance_warning_margin_mm`, 0.05 by default): with a conflict tolerance, `check` and `isolate` list the tracks and rectangular pads thinner than twice the tolerance plus the margin.
- **Better guess of the format of an old-style Gerber layer:** it uses the units the file states and how big the board is (a short coordinate in inch is 2.3, not 1.3), and a list of apertures may give both sides in one cell (`60x40`).
- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Rectangular pens in the viewer:** a track drawn with a rectangular or obround aperture is now the swept rectangle (the segment and the pen, a polygon), not a round-ended line as wide as the narrow side of the pen.
- **Problem zones in `check`:** inner corners sharper than `problem_corner_angle_deg` and tracks that run between two pads nearer than `problem_track_gap_factor` times the tool are listed as warnings; both criteria are settings.
- **Wide tools only round pads** (`around_pads_min_mm`, 0 = off, as before): with a value above 0 every tool except the narrowest cuts only the rings round pads whose smaller side is at least that wide, and the narrowest tool does the rest (tracks and small pads), leaving out what the wider ones have cut when `trim_fine` is on.
- **Tolerance for small conflicts** (`conflict_tolerance_mm`, 0 by default: as before): gaps narrower than the tool by at most that much are accepted by `check` and `isolate`.
- **The distance of the alignment holes** is documented as the setting `alignment_margin_mm` (5 mm by default; 10 mm is what some other programs use).
- **Copper that is not drawn stops `isolate`:** a layer with macro apertures, clear polarity or step and repeat, which the reader cannot draw, is refused with the reasons (the cutter would go over pads that are not there); `--force` writes anyway.

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
