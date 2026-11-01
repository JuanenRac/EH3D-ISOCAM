// EH3D-ISOCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::scene::from_gerber;
use eh3d_isocam::config::Settings;
use eh3d_isocam::drc;
use eh3d_isocam::drill::{
    read_drills, read_outline, route_outline, write_drills, write_route_excellon, Hole,
};
use eh3d_isocam::fiducial::{apply_to_holes, apply_to_passes};
use eh3d_isocam::forced::{forced_passes, insert};
use eh3d_isocam::gcode::write_in;
use eh3d_isocam::hpgl::{write_with as hpgl_with, PenBy};
use eh3d_isocam::isolate::{close_pieces, plan, Pass};
use eh3d_isocam::mask;
use eh3d_isocam::optimize::optimize_passes;
use eh3d_isocam::order::{arrange, directions, travel_mm, Cut, Strategy};
use eh3d_isocam::preview;
use eh3d_isocam::transform::{alignment_holes, lay_out, Axis, Board};
use eh3d_isocam::units::Unit;
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-isocam: isolation milling of copper from a Gerber layer

  eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt   the paths of the tools round the copper (G-code, or HP-GL for a .plt)
  eh3d-isocam check LAYER.gtl                    the places where two pieces of copper are closer than the tool is wide
  eh3d-isocam info LAYER.gtl                     the passes: how many paths, which way round, and the travel through the air
  eh3d-isocam preview LAYER.gtl [OUT.svg|OUT.txt] the passes drawn over the copper: an SVG drawing, or text (on the screen when there is no file)
  eh3d-isocam drc LAYER.gtl [--drills F.ncd]     the design rules, with the place of every finding
  eh3d-isocam drills IN.ncd OUT.ncd [--align]    read a drill file and write it back (turned over, repeated on a panel, with alignment holes)
  eh3d-isocam route OUTLINE OUT.plt|OUT.ncd      the path of the tool round the board (OUTLINE is a Gerber outline layer, a .dxf or a .plt)
  eh3d-isocam mask COPPER.gtl OUT.gts [--mask-layer F.gts]  the openings of the solder mask, from the mask layer or from the pads of the copper
  eh3d-isocam config                             every setting with its default, as a configuration file

Every option is a setting of the configuration file: --config FILE reads one, and --fine-tool-mm 0.15 is fine_tool_mm = 0.15 (the old --tool, --passes, --stepover, --res,
--depth, --feed, --plunge, --safe and --rpm still work). A length may carry its unit (--fine-tool-mm 8mil, --depth-mm 0.002in); a bare number is in --input-units (mm, mil or
inch). --output-units inch writes G20 and inches, --report-units changes what the screen shows. Also --outline FILE and --drills FILE name the outline and the holes that go
with the layer, --align adds the holes that hold the board when it is turned over, and --force writes although the tool does not fit between pieces of copper.
An old-style Gerber layer (no format statement) needs --apertures FILE, the list of its apertures (one D code and its size per line), and the format it is written in
(--gerber-format 2.3, --gerber-zeros leading|trailing, --gerber-units inch|mm, --aperture-units mil|mm|inch, --aperture-size-column 1).
isolate refuses to write when the tool does not fit between pieces of copper (run check to see where) unless --force is given; with --forced-isolation yes it lays a path along
every such gap that is not narrower than --forced-min-gap-mm and refuses only for the narrower ones. --optimize-tolerance-mm and --spike-max-mm make the paths lighter;
--scale-x/--scale-y (about the centre of the board) and --fiducial \"DESIGN_X,DESIGN_Y,MEASURED_X,MEASURED_Y\" (two or more times) scale and correct the work for a stretched or turned board.";

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
            "--outline" | "--drills" | "--mask-layer" | "--apertures" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a file"))?;
                match a.as_str() {
                    "--outline" => cli.outline = Some(v),
                    "--drills" => cli.drills = Some(v),
                    "--apertures" => cli.apertures = Some(v),
                    _ => cli.mask_layer = Some(v),
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

fn read(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A Gerber layer read from a file: extended Gerber as it is, an old-style layer with the format and the list of apertures of the settings.
fn read_layer(cli: &Cli, path: &str) -> Result<eh3d_parser_gerber::GerberFile, String> {
    let text = read(path)?;
    let list = match &cli.apertures {
        Some(f) => Some(read(f)?),
        None => None,
    };
    let text = cli
        .settings
        .gerber_text(&text, list.as_deref())
        .map_err(|e| format!("{path}: {e}"))?;
    Ok(eh3d_parser_gerber::parse(&text))
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
        let drills = read_drills(&read(d)?).map_err(|e| format!("{d}: {e}"))?;
        for n in &drills.notes {
            eprintln!("note: {d}: {n}");
        }
        board.holes = drills.holes;
    }
    if let Some(o) = &cli.outline {
        let outline = read_outline(o, &read(o)?).map_err(|e| format!("{o}: {e}"))?;
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

fn is_plot(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".plt")
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
    let close = close_pieces(&board.scene, pl.finest_mm(), pl.res_mm)?;
    let mut passes = plan(&board.scene, &pl)?;
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
        eprintln!(
            "note: the paths have {} points instead of {} ({} spikes taken out, {} paths dropped)",
            st.points_after, st.points_before, st.spikes, st.dropped_paths
        );
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
    let cli = parse_cli(args)?;
    let s = &cli.settings;
    match cmd.as_str() {
        "config" => {
            print!("{}", s.to_text());
            Ok(())
        }
        "check" | "isolate" | "info" | "preview" => {
            let layer = arg(&cli, 0, &format!("{cmd} needs a Gerber layer"))?;
            let (board, whole) = load_board(&cli, Some(layer))?;
            let pl = s.plan()?;
            pl.validate()?;
            if cmd == "check" {
                let close = close_pieces(&board.scene, pl.finest_mm(), pl.res_mm)?;
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
                hpgl_with(&passes, &order, pens)?
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
                holes.extend(alignment_holes(
                    &whole,
                    s.alignment_hole_mm,
                    s.alignment_margin_mm,
                    s.alignment_holes,
                    s.mirror.unwrap_or(Axis::Vertical),
                    tool,
                )?);
            }
            let (t, _) = s.transform(centre_of(&whole))?;
            if !t.is_identity() {
                apply_to_holes(&mut holes, &t)?;
            }
            write_text(out, write_drills(&holes)?)?;
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
            let pass = route_outline(
                &outline,
                s.route_tool_mm,
                s.res_mm.min(s.route_tool_mm / 2.0),
            )?;
            let mut passes = vec![pass];
            let (t, _) = s.transform(centre_of(&whole))?;
            if !t.is_identity() {
                apply_to_passes(&mut passes, &t)?;
            }
            let pass = passes.remove(0);
            let order = s.order();
            let text = if is_plot(out) {
                hpgl_with(std::slice::from_ref(&pass), &order, PenBy::Pass)?
            } else {
                let (cuts, _) = arrange(&pass.cuts(), &order, order.start);
                write_route_excellon(&cuts, s.route_tool_mm, 1)?
            };
            write_text(out, text)?;
            println!(
                "{} paths, {} of cut written to {out}",
                pass.paths.len(),
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
