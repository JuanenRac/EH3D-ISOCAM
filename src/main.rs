// EH3D-ISOCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::scene::{from_gerber_with, SceneOptions};
use eh3d_isocam::board::{
    centre_of, drills_job, finish_plot, isolation_plot, mark_rows, mask_job, plot_options,
    route_job,
};
use eh3d_isocam::config::Settings;
use eh3d_isocam::drc;
use eh3d_isocam::drill::{read_drills_with_format, read_outline};
use eh3d_isocam::dxf;
use eh3d_isocam::fiducial::apply_to_passes;
use eh3d_isocam::filetype;
use eh3d_isocam::fill::{rubout_contours, Fill};
use eh3d_isocam::forced::{forced_passes, insert};
use eh3d_isocam::gcode::write_in;
use eh3d_isocam::hpgl::{write_opts, PenBy};
use eh3d_isocam::isolate::{net_scene, plan_with_pads, rubout_lines, Pass};
use eh3d_isocam::optimize::optimize_passes;
use eh3d_isocam::order::{arrange, directions, travel_mm, Cut, Strategy};
use eh3d_isocam::preview;
use eh3d_isocam::problems::{problem_zones, thin_for_tolerance_in};
use eh3d_isocam::tolerance::{close_pieces_scoped, ToleranceScope};
use eh3d_isocam::transform::{lay_out, Board};
use eh3d_isocam::units::{parse_length, Unit};
use eh3d_parser_gcode::emit::{header_with_units, sha256_hex, Header};
use eh3d_parser_gerber::{
    codes, parse_bytes_with, read_file_limited, InputError, InputLimits, ReadError, Severity,
};
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
  eh3d-isocam [gui] [LAYER.gtl]                  the window (a build with --features gui): with no arguments, or with a single layer, it opens; the commands above never open it
  eh3d-isocam gui [LAYER.gtl] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

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
the coordinates with zeros in front (5 for whole mils in five digits).
--excellon-format 2.4 (excellon_format = 2.4) reads and writes the coordinates of a drill file, and writes those of the routing file of route, as two digits before the point and four after, six digits in inches (X012340 is 1.2340 in),
with the zeros as the header says (INCH,TZ or INCH,LZ); a file that states its own format (a FILE_FORMAT comment or INCH,TZ,00.0000) keeps it. The default, auto, is what the file says and whole mils
for inches that say nothing. The plain style of ncd_isocam_style is whole mils only.";

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
    /// The units of a drill file that does not say them, or says them wrong (`--units mm|inch`).
    units: Option<eh3d_parser_excellon::Units>,
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
        units: None,
    };
    // the configuration file first, so that the options on the command line win over it
    if let Some(i) = args.iter().position(|a| a == "--config") {
        let file = args
            .get(i + 1)
            .ok_or_else(|| "--config needs a file".to_string())?;
        let text = fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        cli.settings = Settings::parse(&text).map_err(|e| format!("{file}: {e}"))?;
        cli.max_bytes = (cli.settings.max_input_mb * 1_048_576.0) as usize;
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
            "--units" => {
                let v = it.next().ok_or_else(|| format!("{a} needs mm or inch"))?;
                cli.units = Some(match v.trim().to_ascii_lowercase().as_str() {
                    "mm" | "metric" => eh3d_parser_excellon::Units::Millimetres,
                    "inch" | "in" | "inches" => eh3d_parser_excellon::Units::Inches,
                    _ => return Err(format!("--units '{v}' is not mm or inch")),
                });
            }
            "--fiducials-csv" => {
                // the pairs of marks, one row each: design x, design y, measured x, measured y (a header row and lines that start with # are left out)
                let file = it.next().ok_or_else(|| format!("{a} needs a file"))?;
                let text = read(&cli, &file)?;
                for line in mark_rows(&text) {
                    cli.settings
                        .set("fiducial", line)
                        .map_err(|e| format!("{file}: {e}"))?;
                }
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

/// The language of the person from the variables of the system (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`); English when it is none of them.
fn language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = env::var(var) {
            if v.is_empty() {
                continue;
            }
            let code = v.chars().take(2).collect::<String>().to_ascii_lowercase();
            return ["es", "de", "fr", "it", "ja", "zh", "pt"]
                .into_iter()
                .find(|l| *l == code)
                .unwrap_or("en");
        }
    }
    "en"
}

/// The message of a file that is over the size limit, in the language of the person.
fn too_large(size: usize, limit: usize) -> String {
    let human = |n: usize| {
        if n >= 1_048_576 {
            format!("{} MiB", n / 1_048_576)
        } else {
            format!("{n} bytes")
        }
    };
    let (s, l) = (human(size), human(limit));
    match language() {
        "es" => format!("el fichero supera el límite ({s} frente a {l}); suba max_input_mb en el fichero de configuración (o --max-bytes) si es correcto"),
        "de" => format!("die Datei überschreitet die Grenze ({s} gegenüber {l}); max_input_mb in der Konfigurationsdatei (oder --max-bytes) erhöhen, wenn das richtig ist"),
        "fr" => format!("le fichier dépasse la limite ({s} contre {l}) ; augmentez max_input_mb dans le fichier de configuration (ou --max-bytes) si c'est voulu"),
        "it" => format!("il file supera il limite ({s} contro {l}); aumentare max_input_mb nel file di configurazione (o --max-bytes) se è corretto"),
        "ja" => format!("ファイルが上限を超えています（{s}、上限 {l}）。正しい場合は設定ファイルの max_input_mb（または --max-bytes）を増やしてください"),
        "zh" => format!("文件超过上限（{s}，上限 {l}）；如无误，请在配置文件中调高 max_input_mb（或使用 --max-bytes）"),
        "pt" => format!("o ficheiro excede o limite ({s} contra {l}); aumente max_input_mb no ficheiro de configuração (ou --max-bytes) se estiver correto"),
        _ => format!("the file is above the limit ({s} against {l}); raise max_input_mb in the configuration file (or --max-bytes) if that is right"),
    }
}

/// The shapes of a Gerber layer, with the arc tolerance of the settings.
fn scene_of(cli: &Cli, file: &eh3d_parser_gerber::GerberFile) -> eh3d_gerberviewer::scene::Scene {
    from_gerber_with(
        file,
        &SceneOptions {
            arc_tolerance_mm: cli.settings.arc_tolerance_mm,
        },
    )
}

/// The text of a file, read the way the readers of the suite read bytes: a file over the size limit is refused, UTF-8 (with or without a byte order mark) is taken as it is, and
/// anything else is read as Latin-1 with a warning that carries the stable code of the reader.
fn read(cli: &Cli, path: &str) -> Result<String, String> {
    let limits = InputLimits {
        max_bytes: cli.max_bytes,
    };
    let bytes = read_file_limited(std::path::Path::new(path), &limits).map_err(|e| match e {
        ReadError::TooLarge(InputError::TooLarge { size, limit }) => {
            format!("{path}: {}", too_large(size, limit))
        }
        other => format!("{path}: {other}"),
    })?;
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
        board.scene = scene_of(cli, &read_layer(cli, l)?);
        for n in &board.scene.notes {
            eprintln!("note: {n}");
        }
    }
    if let Some(d) = &cli.drills {
        let text = read(cli, d)?;
        filetype::expect_drills(d, filetype::detect(&text))?;
        let drills = read_drills_with_format(&text, cli.units, cli.settings.excellon_format)
            .map_err(|e| format!("{d}: {e}"))?;
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

/// The standard header of a G-code file: the generator and its version, the units, the file the work came from with its SHA-256 and the tools.
fn gcode_header(cli: &Cli, source: Option<&str>) -> String {
    let named = source.and_then(|p| fs::read(p).ok().map(|bytes| (p, sha256_hex(&bytes))));
    let units = if cli.settings.output_units == Unit::Inch {
        "inch (G20)"
    } else {
        "mm (G21)"
    };
    header_with_units(
        &Header {
            generator: "eh3d-isocam",
            version: env!("CARGO_PKG_VERSION"),
            source: named.as_ref().map(|(p, hash)| {
                (
                    std::path::Path::new(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("input"),
                    hash.clone(),
                )
            }),
            tools: "one tool per pass, see the comment of each pass",
        },
        units,
    )
}

/// Writes an output file. The output may not be a file the run reads (the layer, the outline, the holes, the mask or the apertures): that would destroy the input. `out_index` is the place
/// of the output among the positional arguments, so that it is not compared with itself.
fn write_text(cli: &Cli, out_index: usize, out: &str, text: String) -> Result<(), String> {
    let same = |other: &str| match (fs::canonicalize(other), fs::canonicalize(out)) {
        (Ok(a), Ok(b)) => a == b,
        _ => other == out,
    };
    let extras = [
        &cli.outline,
        &cli.drills,
        &cli.mask_layer,
        &cli.apertures,
        &cli.contour,
    ];
    let reads = cli
        .positional
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != out_index)
        .map(|(_, p)| p)
        .chain(extras.into_iter().flatten())
        .any(|p| same(p));
    if reads {
        return Err(format!(
            "{out}: the output file is also an input of this run; write the result to a new file"
        ));
    }
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

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
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
                        write_text(&cli, 1, o, text)?;
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
                isolation_plot(&passes, s, &whole, &board.outline)?
            } else if is_dxf(out) {
                dxf::write(&passes)?
            } else {
                let g = write_in(&passes, &s.job(), &order, s.output_units)?;
                format!(
                    "{}{g}",
                    gcode_header(&cli, cli.positional.first().map(String::as_str))
                )
            };
            write_text(&cli, 1, out, text)?;
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
                    Some(l) => Some(scene_of(&cli, &read_layer(&cli, l)?)),
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
                finish_plot(
                    s,
                    write_opts(
                        std::slice::from_ref(&pass),
                        &order,
                        PenBy::Pass,
                        &plot_options(s, &eh3d_gerberviewer::scene::Bounds::empty()),
                    ),
                )?
            } else if is_dxf(out) {
                dxf::write(std::slice::from_ref(&pass))?
            } else {
                let g = write_in(
                    std::slice::from_ref(&pass),
                    &s.job(),
                    &order,
                    s.output_units,
                )?;
                format!("{}{g}", gcode_header(&cli, cli.contour.as_deref()))
            };
            write_text(&cli, 0, out, text)?;
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
            let copper = scene_of(&cli, &read_layer(&cli, layer)?);
            let mask_scene = match &cli.mask_layer {
                Some(m) => Some(scene_of(&cli, &read_layer(&cli, m)?)),
                None => None,
            };
            let job = mask_job(&copper, mask_scene.as_ref(), s.mask_margin_mm)?;
            for n in &job.notes {
                eprintln!("note: {n}");
            }
            write_text(&cli, 1, out, job.text)?;
            println!(
                "{} openings, {} beyond each pad, written to {out}",
                job.openings,
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
            let job = drills_job(board.holes, &whole, s, cli.align, is_plot(out))?;
            write_text(&cli, 1, out, job.text)?;
            println!("{} holes written to {out}", job.holes);
            Ok(())
        }
        "route" => {
            let file = arg(&cli, 0, "route needs an outline")?;
            let out = arg(&cli, 1, "route needs an output file")?;
            let mut cli2 = cli.clone();
            cli2.outline = Some(file.to_string());
            cli2.drills = None;
            let (board, whole) = load_board(&cli2, None)?;
            let r = route_job(&board.outline, &whole, s, is_plot(out))?;
            if r.tabs.bridges > 0 {
                eprintln!(
                    "note: {} bridges of {} on {} paths",
                    r.tabs.bridges,
                    len(&cli, s.tab_width_mm),
                    r.tabs.paths_cut
                );
            }
            if r.tabs.paths_left_closed > 0 {
                eprintln!(
                    "warning: {} paths are too short for a bridge ({} between bridges) and are cut right round: the piece will not be held",
                    r.tabs.paths_left_closed,
                    len(&cli, s.tab_min_length_mm)
                );
            }
            write_text(&cli, 1, out, r.text)?;
            println!(
                "{} paths, {} of cut written to {out}",
                r.paths,
                len(&cli, r.cut_mm)
            );
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

/// Whether the program opens its window instead of running a command, and the layer to open in it: no arguments, `gui` (with a layer or without), or a single layer file given by the
/// system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [a] if a == "gui" => Some(None),
        [a, f] if a == "gui" => Some(Some(f.clone())),
        [f] if !f.starts_with('-')
            && std::path::Path::new(f).extension().is_some()
            && std::path::Path::new(f).is_file() =>
        {
            Some(Some(f.clone()))
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(layer: Option<String>) -> ExitCode {
    let lang = eh3d_isocam::lang::Lang::from_env();
    match eh3d_isocam::gui::run(layer.map(std::path::PathBuf::from), lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_isocam::shot::parse_window_args(args, 1) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_isocam::lang::Lang::from_env);
    match eh3d_isocam::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_isocam::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(layer) = window_request(&args) {
        return open_window(layer);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
