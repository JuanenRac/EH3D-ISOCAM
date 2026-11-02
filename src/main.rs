// EH3D-ISOCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::scene::from_gerber;
use eh3d_isocam::config::Settings;
use eh3d_isocam::drc;
use eh3d_isocam::drill::{
    add_tabs, read_drills, read_outline, route_outline, write_drills_as, write_route_excellon,
    DrillStyle, Hole, Tabs,
};
use eh3d_isocam::dxf;
use eh3d_isocam::fiducial::{apply_to_holes, apply_to_passes};
use eh3d_isocam::filetype;
use eh3d_isocam::fill::{rubout_contours, Fill};
use eh3d_isocam::forced::{forced_passes, insert};
use eh3d_isocam::gcode::write_in;
use eh3d_isocam::hpgl::{set_pen_speed, write_holes_opts, write_opts, PenBy, PlotOptions};
use eh3d_isocam::isolate::{net_scene, plan_with_pads, rubout_lines, Pass};
use eh3d_isocam::mask;
use eh3d_isocam::optimize::optimize_passes;
use eh3d_isocam::order::{arrange, directions, travel_mm, Cut, Strategy};
use eh3d_isocam::preview;
use eh3d_isocam::problems::{problem_zones, thin_for_tolerance_in};
use eh3d_isocam::tolerance::{close_pieces_scoped, ToleranceScope};
use eh3d_isocam::transform::{alignment_pattern, lay_out, Axis, Board};
use eh3d_isocam::units::{parse_length, Unit};
use eh3d_parser_gerber::{codes, parse_bytes_with, InputError, InputLimits, Severity};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-isocam: isolation milling of copper from a Gerber layer

  eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt|OUT.dxf  the paths of the tools round the copper (G-code, HP-GL for a .plt, a DXF drawing for a .dxf)
  eh3d-isocam check LAYER.gtl                    the places where two pieces of copper are closer than the tool is wide
  eh3d-isocam info LAYER.gtl                     the passes: how many paths, which way round, and the travel through the air
  eh3d-isocam preview LAYER.gtl [OUT.svg|OUT.txt] the passes drawn over the copper: an SVG drawing, or text (on the screen when there is no file)
  eh3d-isocam drc LAYER.gtl [--drills F.ncd]     the design rules, with the place of every finding
  eh3d-isocam drills IN.ncd OUT.ncd|OUT.plt [--align]  read a drill file and write it back (turned over, repeated on a panel, with alignment holes), or as a plot with a pen for each drill (hpgl_pen_map)
  eh3d-isocam route OUTLINE OUT.plt|OUT.ncd      the path of the tool round the board (OUTLINE is a Gerber outline layer, a .dxf or a .plt)
  eh3d-isocam mask COPPER.gtl OUT.gts [--mask-layer F.gts]  the openings of the solder mask, from the mask layer or from the pads of the copper
  eh3d-isocam rubout-area OUT.nc|OUT.plt|OUT.dxf [LAYER.gtl] --area X0,Y0,X1,Y1|--contour FILE  parallel lines of a tool that clear an area or fill closed paths, cut to the copper of LAYER when it is given
  eh3d-isocam kind FILE...                       say what kind of file each one is (Gerber, old-style Gerber, Excellon, HP-GL, DXF)
  eh3d-isocam config                             every setting with its default, as a configuration file

Every option is a setting of the configuration file: --config FILE reads one, and --fine-tool-mm 0.15 is fine_tool_mm = 0.15 (the old --tool, --passes, --stepover, --res,
--depth, --feed, --plunge, --safe and --rpm still work). A length may carry its unit (--fine-tool-mm 8mil, --depth-mm 0.002in); a bare number is in --input-units (mm, mil or
inch). --output-units inch writes G20 and inches, --report-units changes what the screen shows. Also --outline FILE and --drills FILE name the outline and the holes that go
with the layer, --align adds the holes that hold the board when it is turned over, and --force writes although the tool does not fit between pieces of copper.
An old-style Gerber layer (no format statement) needs --apertures FILE, the list of its apertures (one D code and its size per line), and the format it is written in
(--gerber-format 2.3, --gerber-zeros leading|trailing, --gerber-units inch|mm, --aperture-units mil|mm|inch, --aperture-size-column 1).
isolate refuses to write when the tool does not fit between pieces of copper (run check to see where) unless --force is given; with --forced-isolation yes it lays a path along
every such gap that is not narrower than --forced-min-gap-mm and refuses only for the narrower ones. Files are read with a size limit (--max-bytes N, 268435456 by default); text that
is not UTF-8 is read as Latin-1 with a warning, and the notices of the Gerber reader are shown with their code: --hide CODE[,CODE] leaves out the notices of those codes. --optimize-tolerance-mm and --spike-max-mm make the paths lighter;
--scale-x/--scale-y (about the centre of the board) and --fiducial \"DESIGN_X,DESIGN_Y,MEASURED_X,MEASURED_Y\" (two or more times) scale and correct the work for a stretched or turned board.
isolate --net X,Y isolates only the net that holds the point (up to half a millimetre from its copper) with the tool of net_tool_mm. route puts bridges on the cut with tab_count,
tab_width_mm and tab_min_length_mm. With a conflict tolerance, check and isolate warn about the tracks and pads that are thin for it (tolerance_warning_margin_mm). The alignment
holes are 2, 3 or 6, outside the board or in a special pattern (alignment_pattern, alignment_special_a_mm and alignment_special_b_mm).
conflict_tolerance_scope = pads keeps the tolerance for the gaps beside a pad. A layer that is really a drill file, a plot or a drawing (and the other way round) is refused by name; kind says
what a file is. hpgl_pen_map and hpgl_pen_speed_cm_s (0 = the machine's own) set the pens of a plot of holes and the speed of a plot. hpgl_isocam_style (on) writes a plot with a PA for each point and lines of about 75 columns, as the original program did.
hpgl_shift_negative = no keeps the coordinates of the board in a plot, negative ones included, so that a plot and a drill file share one origin. isolate with plot_outline = yes and
--outline FILE puts the cut round the outline (the tool of route_tool_mm) first in the plot, on hpgl_outline_pen (0 = the pen after the others), the isolation on hpgl_isolation_pen
(0 = as usual). ncd_isocam_style = yes writes a drill file as the Bungard programs do (M48 and %, tools by number, a repeated coordinate left out, CR LF, M00), and ncd_digits fills
the coordinates with zeros in front (5 for whole mils in five digits).";

#[derive(Clone)]
struct Cli {
    positional: Vec<String>,
    settings: Settings,
    force: bool,
    align: bool,
    outline: Option<String>,
    drills: Option<String>,
    mask_layer: Option<String>,
    apertures: Option<String>,
    /// A file whose closed paths are filled with lines (`--contour FILE`).
    contour: Option<String>,
    /// A point of the net to isolate by itself (`--net X,Y`).
    net: Option<(f64, f64)>,
    /// The area to clear with lines (`--area X0,Y0,X1,Y1`).
    area: Option<(f64, f64, f64, f64)>,
    /// The largest file that is read, in bytes (`--max-bytes N`).
    max_bytes: usize,
    /// The codes of the notices of the Gerber reader that are not shown (`--hide CODE,CODE`).
    hide: Vec<String>,
}

/// Numbers separated by commas, each one a length that may carry its unit.
fn lengths(v: &str, unit: Unit, n: usize, what: &str) -> Result<Vec<f64>, String> {
    let parts: Vec<&str> = v.split(',').collect();
    if parts.len() != n {
        return Err(format!(
            "{what} needs {n} numbers separated by commas, like 1,2"
        ));
    }
    parts
        .iter()
        .map(|p| parse_length(p, unit).map_err(|e| format!("{what}: {e}")))
        .collect()
}

fn parse_cli(args: Vec<String>) -> Result<Cli, String> {
    let mut cli = Cli {
        positional: Vec::new(),
        settings: Settings::default(),
        force: false,
        align: false,
        outline: None,
        drills: None,
        mask_layer: None,
        apertures: None,
        contour: None,
        net: None,
        area: None,
        max_bytes: InputLimits::default().max_bytes,
        hide: Vec::new(),
    };
    // the configuration file first, so that the options on the command line win over it
    if let Some(i) = args.iter().position(|a| a == "--config") {
        let file = args
            .get(i + 1)
            .ok_or_else(|| "--config needs a file".to_string())?;
        let text = fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        cli.settings = Settings::parse(&text).map_err(|e| format!("{file}: {e}"))?;
    }
    // the unit of the bare numbers counts wherever it is on the line
    if let Some(i) = args.iter().position(|a| a == "--input-units") {
        let v = args
            .get(i + 1)
            .ok_or_else(|| "--input-units needs a value".to_string())?;
        cli.settings.set("input_units", v)?;
    }
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--force" => cli.force = true,
            "--align" => cli.align = true,
            "--config" | "--input-units" => {
                it.next();
            }
            "--outline" | "--drills" | "--mask-layer" | "--apertures" | "--contour" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a file"))?;
                match a.as_str() {
                    "--outline" => cli.outline = Some(v),
                    "--drills" => cli.drills = Some(v),
                    "--apertures" => cli.apertures = Some(v),
                    "--contour" => cli.contour = Some(v),
                    _ => cli.mask_layer = Some(v),
                }
            }
            "--max-bytes" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                cli.max_bytes = v
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| format!("--max-bytes '{v}' is not a whole number"))?;
            }
            "--hide" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                for c in v.split(',').map(str::trim).filter(|c| !c.is_empty()) {
                    if !codes::ALL.contains(&c) {
                        return Err(format!("--hide: '{c}' is not a code of the reader"));
                    }
                    cli.hide.push(c.to_string());
                }
            }
            "--net" | "--area" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                let unit = cli.settings.input_units;
                if a == "--net" {
                    let n = lengths(&v, unit, 2, "--net")?;
                    cli.net = Some((n[0], n[1]));
                } else {
                    let n = lengths(&v, unit, 4, "--area")?;
                    cli.area = Some((n[0], n[1], n[2], n[3]));
                }
            }
            o if o.starts_with("--") => {
                let v = it.next().ok_or_else(|| format!("{o} needs a value"))?;
                cli.settings.set(o, &v)?;
            }
            _ => cli.positional.push(a),
        }
    }
    cli.settings.validate()?;
    Ok(cli)
}

/// The text of a file, read the way the readers of the suite read bytes: a file over the size limit is refused, UTF-8 (with or without a byte order mark) is taken as it is, and
/// anything else is read as Latin-1 with a warning that carries the stable code of the reader.
fn read(cli: &Cli, path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if bytes.len() > cli.max_bytes {
        let e = InputError::TooLarge {
            size: bytes.len(),
            limit: cli.max_bytes,
        };
        return Err(format!("{path}: {e}"));
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text
            .strip_prefix('\u{feff}')
            .map(str::to_string)
            .unwrap_or(text)),
        Err(e) => {
            if !cli.hide.iter().any(|c| c == codes::INPUT_NOT_UTF8) {
                eprintln!(
                    "warning: {path}: the text is not UTF-8 and was read as Latin-1 [{}]",
                    codes::INPUT_NOT_UTF8
                );
            }
            Ok(e.into_bytes().iter().map(|&b| char::from(b)).collect())
        }
    }
}

/// A Gerber layer read from a file: extended Gerber as it is, an old-style layer with the format and the list of apertures of the settings.
fn read_layer(cli: &Cli, path: &str) -> Result<eh3d_parser_gerber::GerberFile, String> {
    let text = read(cli, path)?;
    filetype::expect_gerber(path, filetype::detect(&text))?;
    let list = match &cli.apertures {
        Some(f) => Some(read(cli, f)?),
        None => None,
    };
    let text = cli
        .settings
        .gerber_text(&text, list.as_deref())
        .map_err(|e| format!("{path}: {e}"))?;
    let limits = InputLimits {
        max_bytes: cli.max_bytes,
    };
    let file = parse_bytes_with(text.as_bytes(), &limits).map_err(|e| format!("{path}: {e}"))?;
    // what the reader says, by its stable code: a notice can be hidden with --hide
    for d in &file.diagnostics {
        if d.severity != Severity::Info && !cli.hide.iter().any(|c| c.as_str() == d.code) {
            let kind = if d.severity == Severity::Error {
                "error"
            } else {
                "warning"
            };
            eprintln!(
                "{kind}: {path}: line {}: {} [{}]",
                d.line, d.message, d.code
            );
        }
    }
    Ok(file)
}

fn arg<'a>(cli: &'a Cli, i: usize, what: &str) -> Result<&'a str, String> {
    cli.positional
        .get(i)
        .map(String::as_str)
        .ok_or_else(|| what.to_string())
}

/// A length for the screen, in the unit chosen by `report_units`.
fn len(cli: &Cli, mm: f64) -> String {
    let u = cli.settings.report_units;
    match u {
        Unit::Mm => format!("{mm:.3} mm"),
        Unit::Mil => format!("{:.1} mil", u.from_mm(mm)),
        Unit::Inch => format!("{:.4} in", u.from_mm(mm)),
    }
}

/// The layer, with the holes and the outline that go with it, laid out as the settings say. Returns the box of the whole panel as well.
fn load_board(
    cli: &Cli,
    layer: Option<&str>,
) -> Result<(Board, eh3d_gerberviewer::scene::Bounds), String> {
    let mut board = Board::default();
    if let Some(l) = layer {
        board.scene = from_gerber(&read_layer(cli, l)?);
        for n in &board.scene.notes {
            eprintln!("note: {n}");
        }
    }
    if let Some(d) = &cli.drills {
        let text = read(cli, d)?;
        filetype::expect_drills(d, filetype::detect(&text))?;
        let drills = read_drills(&text).map_err(|e| format!("{d}: {e}"))?;
        for n in &drills.notes {
            eprintln!("note: {d}: {n}");
        }
        board.holes = drills.holes;
    }
    if let Some(o) = &cli.outline {
        let outline = read_outline(o, &read(cli, o)?).map_err(|e| format!("{o}: {e}"))?;
        if !outline.open.is_empty() {
            eprintln!(
                "note: {o}: {} runs of the outline do not close and are left out",
                outline.open.len()
            );
        }
        board.outline = outline.closed;
    }
    lay_out(&board, &cli.settings.layout())
}

fn write_text(out: &str, text: String) -> Result<(), String> {
    fs::write(out, text).map_err(|e| format!("{out}: {e}"))
}

/// The places where the finest tool does not fit, with the conflict tolerance applied as the settings say (to every gap, or only beside pads).
fn close_scoped(
    s: &Settings,
    scene: &eh3d_gerberviewer::scene::Scene,
    pl: &eh3d_isocam::isolate::Plan,
) -> Result<Vec<eh3d_isocam::isolate::Closeness>, String> {
    close_pieces_scoped(
        scene,
        pl.finest_mm(),
        s.conflict_tolerance_mm,
        s.conflict_tolerance_scope,
        pl.res_mm,
    )
}

/// A plot file with the pen speed of the settings (none when it is 0).
fn plot(cli: &Cli, text: Result<String, String>) -> Result<String, String> {
    let text = set_pen_speed(&text?, cli.settings.hpgl_pen_speed_cm_s)?;
    Ok(if cli.settings.hpgl_isocam_style {
        eh3d_isocam::hpgl::isocam_style(&text)
    } else {
        text
    })
}

/// How a plot is written, from the settings.
fn plot_options(cli: &Cli) -> PlotOptions {
    PlotOptions {
        keep_negative: !cli.settings.hpgl_shift_negative,
        pen_of_pass: Vec::new(),
    }
}

fn is_dxf(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".dxf")
}

fn is_plot(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".plt")
}

/// The warning about copper that is thin for the tolerance, when there is a tolerance (the first few, then how many more).
fn warn_thin(
    cli: &Cli,
    scene: &eh3d_gerberviewer::scene::Scene,
    out: &mut dyn FnMut(String),
) -> Result<(), String> {
    let s = &cli.settings;
    let thin = thin_for_tolerance_in(
        scene,
        s.conflict_tolerance_mm,
        s.tolerance_warning_margin_mm,
        s.conflict_tolerance_scope == ToleranceScope::Pads,
    )?;
    for c in thin.iter().take(5) {
        out(format!(
            "the tolerance is high against a track or pad {} wide near ({:.3}, {:.3}): below {} (twice the tolerance plus {}) it may be eaten away",
            len(cli, c.size_mm),
            c.x,
            c.y,
            len(cli, 2.0 * s.conflict_tolerance_mm + s.tolerance_warning_margin_mm),
            len(cli, s.tolerance_warning_margin_mm)
        ));
    }
    if thin.len() > 5 {
        out(format!(
            "{} more tracks or pads are as thin",
            thin.len() - 5
        ));
    }
    Ok(())
}

fn describe(passes: &[Pass], cli: &Cli) {
    let order = cli.settings.order();
    for (k, p) in passes.iter().enumerate() {
        let cuts = p.cuts();
        let (arranged, _) = arrange(&cuts, &order, order.start);
        let (cw, ccw, open) = directions(&arranged);
        println!(
            "{} {} (tool {}, {} from the copper): {} closed paths ({} clockwise, {} counterclockwise), {} open runs",
            if p.rubout { "ring" } else { "pass" },
            k + 1,
            len(cli, p.diameter_mm),
            len(cli, p.offset_mm),
            cw + ccw,
            cw,
            ccw,
            open
        );
    }
}

/// The passes of a layer: the paths of the tools, the forced paths when they are switched on, the lighter paths. With `moved` the scale and the correction from the reference
/// points are applied as well. Returns the passes and the number of gaps where the tool does not fit and nothing was laid.
fn build_passes(
    cli: &Cli,
    board: &Board,
    centre: (f64, f64),
    moved: bool,
) -> Result<(Vec<Pass>, usize), String> {
    let s = &cli.settings;
    let pl = s.plan()?;
    warn_thin(cli, &board.scene, &mut |m| eprintln!("warning: {m}"))?;
    let close = close_scoped(s, &board.scene, &pl)?;
    let mut passes = plan_with_pads(&board.scene, &pl, s.around_pads())?;
    let mut unfixed = close.len();
    if let Some(f) = s.forced() {
        let r = forced_passes(&board.scene, &pl, &f)?;
        for site in &r.forced {
            eprintln!(
                "warning: forced isolation near ({:.3}, {:.3}): the gap is {} and the tool is {}; {} of path laid along the middle of it",
                site.x,
                site.y,
                len(cli, site.gap_mm),
                len(cli, pl.finest_mm()),
                len(cli, site.length_mm)
            );
        }
        for site in &r.too_narrow {
            eprintln!(
                "warning: near ({:.3}, {:.3}) the gap is {}, narrower than forced_min_gap_mm ({}): nothing is laid there",
                site.x,
                site.y,
                len(cli, site.gap_mm),
                len(cli, f.min_gap_mm)
            );
        }
        unfixed = r.too_narrow.len();
        insert(&mut passes, r.passes);
    } else {
        for c in &close {
            eprintln!(
                "warning: {} pieces of copper are closer than {} near ({:.3}, {:.3})",
                c.pieces,
                len(cli, pl.finest_mm()),
                c.x,
                c.y
            );
        }
    }
    let o = s.optimize();
    if !o.is_off() {
        let st = optimize_passes(&mut passes, &o)?;
        // nothing to say when the paths came out as they went in
        if st.points_after != st.points_before || st.spikes > 0 || st.dropped_paths > 0 {
            eprintln!(
                "note: the paths have {} points instead of {} ({} spikes taken out, {} paths dropped)",
                st.points_after, st.points_before, st.spikes, st.dropped_paths
            );
        }
    }
    if moved {
        let (t, fit) = s.transform(centre)?;
        if let Some(f) = &fit {
            eprintln!(
                "note: the reference points give a scale of {:.5} x {:.5} and a turn of {:.4} degrees; the farthest mark is {} from where the fit puts it",
                f.scale_x,
                f.scale_y,
                f.rotation_deg,
                len(cli, f.residuals_mm.iter().copied().fold(0.0, f64::max))
            );
        }
        if !t.is_identity() {
            apply_to_passes(&mut passes, &t)?;
        }
    }
    Ok((passes, unfixed))
}

/// The centre of the box of the whole board (or panel); the origin when it is empty.
fn centre_of(b: &eh3d_gerberviewer::scene::Bounds) -> (f64, f64) {
    if b.is_empty() {
        (0.0, 0.0)
    } else {
        ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0)
    }
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    let mut cli = parse_cli(args)?;
    // one net by itself: the tool of the net is the only tool, whatever the plan said
    if cli.net.is_some() && matches!(cmd.as_str(), "isolate" | "info" | "preview") {
        let s = &mut cli.settings;
        s.fine_tool_mm = s.net_tool_mm;
        s.fine_passes = 1;
        s.coarse_tool_mm = 0.0;
        s.rubout_tool_mm = 0.0;
        s.around_pads_min_mm = 0.0;
        s.res_mm = s.res_mm.min(s.net_tool_mm / 2.0);
        s.conflict_tolerance_mm = s.conflict_tolerance_mm.min(s.net_tool_mm / 2.0);
        s.validate()?;
    }
    let s = &cli.settings;
    match cmd.as_str() {
        "kind" => {
            if cli.positional.is_empty() {
                return Err("kind needs a file".into());
            }
            for f in &cli.positional {
                println!("{f}: {}", filetype::detect(&read(&cli, f)?).name());
            }
            Ok(())
        }
        "config" => {
            print!("{}", s.to_text());
            Ok(())
        }
        "check" | "isolate" | "info" | "preview" => {
            let layer = arg(&cli, 0, &format!("{cmd} needs a Gerber layer"))?;
            let (mut board, whole) = load_board(&cli, Some(layer))?;
            if let (Some(at), true) = (cli.net, cmd != "check") {
                board.scene = net_scene(&board.scene, at, s.res_mm)?;
                eprintln!(
                    "note: the net at ({}, {}) has {} shapes; it is isolated with a tool of {}",
                    at.0,
                    at.1,
                    board.scene.shapes.len(),
                    len(&cli, s.net_tool_mm)
                );
            }
            let pl = s.plan()?;
            pl.validate()?;
            if cmd == "check" {
                let close = close_scoped(s, &board.scene, &pl)?;
                for z in problem_zones(&board.scene, &s.problem_rules(), pl.res_mm)? {
                    println!("warning: {}", z.describe());
                }
                warn_thin(&cli, &board.scene, &mut |m| println!("warning: {m}"))?;
                for c in &close {
                    println!(
                        "{} pieces of copper are closer than {} near ({:.3}, {:.3})",
                        c.pieces,
                        len(&cli, pl.finest_mm()),
                        c.x,
                        c.y
                    );
                }
                return if close.is_empty() {
                    println!("the tool fits between every piece of copper");
                    Ok(())
                } else {
                    Err(format!(
                        "{} places where the tool does not fit",
                        close.len()
                    ))
                };
            }
            let (passes, unfixed) =
                build_passes(&cli, &board, centre_of(&whole), cmd == "isolate")?;
            // copper that the reader cannot draw (macro apertures, clear polarity, step and repeat) is not isolated: the cutter would go over it
            if cmd == "isolate" && !board.scene.notes.is_empty() && !cli.force {
                return Err(format!(
                    "the layer has copper that is not drawn, so the paths would be wrong ({}); --force writes anyway",
                    board.scene.notes.join("; ")
                ));
            }
            if cmd == "isolate" && unfixed > 0 && !cli.force {
                return Err(format!("the tool does not fit between pieces of copper in {unfixed} places (run check to see them, or --force{})", if s.forced_isolation { "" } else { "; forced_isolation = yes lays a path along the wider ones" }));
            }
            if cmd == "preview" {
                let out = cli.positional.get(1);
                let svg = out.is_some_and(|o| o.to_ascii_lowercase().ends_with(".svg"));
                let text = if svg {
                    preview::svg(&board.scene, &passes)?
                } else {
                    preview::text(&board.scene, &passes, s.preview_cols)?
                };
                match out {
                    Some(o) => {
                        write_text(o, text)?;
                        println!("the preview of {} passes written to {o}", passes.len());
                    }
                    None => print!("{text}"),
                }
                return Ok(());
            }
            if cmd == "info" {
                describe(&passes, &cli);
                let all: Vec<Cut> = passes.iter().flat_map(Pass::cuts).collect();
                let order = s.order();
                let original = eh3d_isocam::order::Order {
                    strategy: Strategy::Original,
                    ..order
                };
                let (as_traced, _) = arrange(&all, &original, order.start);
                let (chosen, _) = arrange(&all, &order, order.start);
                println!(
                    "travel through the air: {} in the order they were traced, {} with ordering = {}",
                    len(&cli, travel_mm(&as_traced, order.start)),
                    len(&cli, travel_mm(&chosen, order.start)),
                    order.strategy.name()
                );
                return Ok(());
            }
            let out = arg(&cli, 1, "isolate needs an output file")?;
            let order = s.order();
            let text = if is_plot(out) {
                let pens = if pl.tools.len() > 1 || pl.rubout.is_some() {
                    PenBy::Tool
                } else {
                    PenBy::Pass
                };
                let mut all = passes.clone();
                let mut opts = plot_options(&cli);
                if s.plot_outline {
                    if board.outline.is_empty() {
                        return Err(
                            "plot_outline needs the outline of the board: give --outline FILE"
                                .into(),
                        );
                    }
                    let outline = eh3d_isocam::drill::Outline {
                        closed: board.outline.clone(),
                        open: Vec::new(),
                    };
                    let mut cut = route_outline(
                        &outline,
                        s.route_tool_mm,
                        s.res_mm.min(s.route_tool_mm / 2.0),
                    )?;
                    let (t, _) = s.transform(centre_of(&whole))?;
                    if !t.is_identity() {
                        let mut one = vec![cut];
                        apply_to_passes(&mut one, &t)?;
                        cut = one.remove(0);
                    }
                    // the pens of the passes as they would be, then the cut in front of them
                    let usual = |k: usize, p: &Pass| match pens {
                        PenBy::Pass => k + 1,
                        PenBy::Tool => p.tool + 1,
                    };
                    let mut pen_of: Vec<usize> = all
                        .iter()
                        .enumerate()
                        .map(|(k, p)| match s.hpgl_isolation_pen {
                            0 => usual(k, p),
                            n => usize::from(n),
                        })
                        .collect();
                    let outline_pen = match s.hpgl_outline_pen {
                        0 => pen_of.iter().copied().max().unwrap_or(0) + 1,
                        n => usize::from(n),
                    };
                    pen_of.insert(0, outline_pen);
                    all.insert(0, cut);
                    opts.pen_of_pass = pen_of;
                }
                plot(&cli, write_opts(&all, &order, pens, &opts))?
            } else if is_dxf(out) {
                dxf::write(&passes)?
            } else {
                write_in(&passes, &s.job(), &order, s.output_units)?
            };
            write_text(out, text)?;
            let total: f64 = passes
                .iter()
                .flat_map(Pass::cuts)
                .map(|c| c.length_mm())
                .sum();
            println!(
                "{} passes, {} paths, {} of cut written to {out}",
                passes.len(),
                passes.iter().map(|ps| ps.cuts().len()).sum::<usize>(),
                len(&cli, total)
            );
            Ok(())
        }
        "rubout-area" => {
            let out = arg(&cli, 0, "rubout-area needs an output file")?;
            let layer = cli.positional.get(1).map(String::as_str);
            let pass = if cli.contour.is_some() || layer.is_some() {
                // contours (a file, or the box of --area), cut to the copper of the layer when one is given
                let contours =
                    match (&cli.contour, cli.area) {
                        (Some(f), _) => {
                            let o = read_outline(f, &read(&cli, f)?)
                                .map_err(|e| format!("{f}: {e}"))?;
                            o.closed
                        }
                        (None, Some((x0, y0, x1, y1))) => {
                            vec![vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]]
                        }
                        (None, None) => return Err(
                            "rubout-area with a layer needs --contour FILE or --area X0,Y0,X1,Y1"
                                .into(),
                        ),
                    };
                let copper = match layer {
                    Some(l) => Some(from_gerber(&read_layer(&cli, l)?)),
                    None => None,
                };
                rubout_contours(
                    &contours,
                    copper.as_ref(),
                    &Fill {
                        tool_mm: s.rubout_lines_tool_mm,
                        overlap: s.rubout_overlap,
                        direction: s.rubout_lines_direction,
                        margin_mm: s.rubout_fill_margin_mm,
                        res_mm: s.res_mm.min(s.rubout_lines_tool_mm / 2.0),
                    },
                )?
            } else {
                let area = cli.area.ok_or(
                    "rubout-area needs --area X0,Y0,X1,Y1 (the corners of the area) or --contour FILE",
                )?;
                rubout_lines(
                    area,
                    s.rubout_lines_tool_mm,
                    s.rubout_overlap,
                    s.rubout_lines_direction,
                )?
            };
            let order = s.order();
            let text = if is_plot(out) {
                plot(
                    &cli,
                    write_opts(
                        std::slice::from_ref(&pass),
                        &order,
                        PenBy::Pass,
                        &plot_options(&cli),
                    ),
                )?
            } else if is_dxf(out) {
                dxf::write(std::slice::from_ref(&pass))?
            } else {
                write_in(
                    std::slice::from_ref(&pass),
                    &s.job(),
                    &order,
                    s.output_units,
                )?
            };
            write_text(out, text)?;
            println!(
                "{} lines of {} written to {out}",
                pass.open.len(),
                len(&cli, s.rubout_lines_tool_mm)
            );
            Ok(())
        }
        "mask" => {
            let layer = arg(&cli, 0, "mask needs a copper layer")?;
            let out = arg(&cli, 1, "mask needs an output file")?;
            let copper = from_gerber(&read_layer(&cli, layer)?);
            let mask_scene = match &cli.mask_layer {
                Some(m) => Some(from_gerber(&read_layer(&cli, m)?)),
                None => None,
            };
            let open = mask::openings(&copper, mask_scene.as_ref(), s.mask_margin_mm)?;
            for n in &open.notes {
                eprintln!("note: {n}");
            }
            write_text(out, mask::write_gerber(&open)?)?;
            println!(
                "{} openings, {} beyond each pad, written to {out}",
                open.shapes.len(),
                len(&cli, s.mask_margin_mm)
            );
            Ok(())
        }
        "drc" => {
            let layer = arg(&cli, 0, "drc needs a Gerber layer")?;
            let (board, _) = load_board(&cli, Some(layer))?;
            let found = drc::check(&board.scene, &board.holes, &s.rules(), s.res_mm)?;
            for v in &found {
                println!("{}", v.describe());
            }
            if found.is_empty() {
                println!("the layer meets the rules");
                Ok(())
            } else {
                Err(format!("{} findings", found.len()))
            }
        }
        "drills" => {
            let input = arg(&cli, 0, "drills needs a drill file")?;
            let out = arg(&cli, 1, "drills needs an output file")?;
            let mut cli2 = cli.clone();
            cli2.drills = Some(input.to_string());
            let (board, whole) = load_board(&cli2, None)?;
            let mut holes: Vec<Hole> = board.holes;
            if cli.align {
                let clash = holes.iter().any(|h| {
                    h.tool == s.alignment_tool
                        && (h.diameter_mm - s.alignment_hole_mm).abs() > 0.005
                });
                let tool = if clash {
                    holes
                        .iter()
                        .map(|h| h.tool)
                        .max()
                        .and_then(|m| m.checked_add(1))
                        .ok_or("no free tool number for the alignment holes")?
                } else {
                    s.alignment_tool
                };
                holes.extend(alignment_pattern(
                    &whole,
                    s.alignment_hole_mm,
                    s.alignment_margin_mm,
                    s.alignment_holes,
                    s.mirror.unwrap_or(Axis::Vertical),
                    tool,
                    s.alignment_pattern,
                    (s.alignment_special_a_mm, s.alignment_special_b_mm),
                )?);
            }
            let (t, _) = s.transform(centre_of(&whole))?;
            if !t.is_identity() {
                apply_to_holes(&mut holes, &t)?;
            }
            let text = if is_plot(out) {
                plot(
                    &cli,
                    write_holes_opts(&holes, &s.hpgl_pen_map, !s.hpgl_shift_negative),
                )?
            } else {
                write_drills_as(
                    &holes,
                    &DrillStyle {
                        plain: s.ncd_isocam_style,
                        digits: s.ncd_digits,
                    },
                )?
            };
            write_text(out, text)?;
            println!("{} holes written to {out}", holes.len());
            Ok(())
        }
        "route" => {
            let file = arg(&cli, 0, "route needs an outline")?;
            let out = arg(&cli, 1, "route needs an output file")?;
            let mut cli2 = cli.clone();
            cli2.outline = Some(file.to_string());
            cli2.drills = None;
            let (board, whole) = load_board(&cli2, None)?;
            let outline = eh3d_isocam::drill::Outline {
                closed: board.outline,
                open: Vec::new(),
            };
            let mut pass = route_outline(
                &outline,
                s.route_tool_mm,
                s.res_mm.min(s.route_tool_mm / 2.0),
            )?;
            if s.tab_count > 0 {
                let r = add_tabs(
                    &mut pass,
                    s.route_tool_mm,
                    &Tabs {
                        count: s.tab_count,
                        width_mm: s.tab_width_mm,
                        min_length_mm: s.tab_min_length_mm,
                    },
                )?;
                if r.bridges > 0 {
                    eprintln!(
                        "note: {} bridges of {} on {} paths",
                        r.bridges,
                        len(&cli, s.tab_width_mm),
                        r.paths_cut
                    );
                }
                if r.paths_left_closed > 0 {
                    eprintln!(
                        "warning: {} paths are too short for a bridge ({} between bridges) and are cut right round: the piece will not be held",
                        r.paths_left_closed,
                        len(&cli, s.tab_min_length_mm)
                    );
                }
            }
            let mut passes = vec![pass];
            let (t, _) = s.transform(centre_of(&whole))?;
            if !t.is_identity() {
                apply_to_passes(&mut passes, &t)?;
            }
            let pass = passes.remove(0);
            let order = s.order();
            let text = if is_plot(out) {
                plot(
                    &cli,
                    write_opts(
                        std::slice::from_ref(&pass),
                        &order,
                        PenBy::Pass,
                        &plot_options(&cli),
                    ),
                )?
            } else {
                let (cuts, _) = arrange(&pass.cuts(), &order, order.start);
                write_route_excellon(&cuts, s.route_tool_mm, 1)?
            };
            write_text(out, text)?;
            println!(
                "{} paths, {} of cut written to {out}",
                pass.cuts().len(),
                len(&cli, pass.cuts().iter().map(Cut::length_mm).sum::<f64>())
            );
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
