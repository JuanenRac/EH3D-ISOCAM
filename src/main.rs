// EH3D-ISOCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::scene::from_gerber;
use eh3d_isocam::config::Settings;
use eh3d_isocam::drc;
use eh3d_isocam::drill::{
    read_drills, read_outline, route_outline, write_drills, write_route_excellon, Hole,
};
use eh3d_isocam::gcode::write_with;
use eh3d_isocam::hpgl::{write_with as hpgl_with, PenBy};
use eh3d_isocam::isolate::{close_pieces, plan, Pass};
use eh3d_isocam::order::{arrange, directions, travel_mm, Cut, Strategy};
use eh3d_isocam::transform::{alignment_holes, lay_out, Axis, Board};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-isocam: isolation milling of copper from a Gerber layer

  eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt   the paths of the tools round the copper (G-code, or HP-GL for a .plt)
  eh3d-isocam check LAYER.gtl                    the places where two pieces of copper are closer than the tool is wide
  eh3d-isocam info LAYER.gtl                     the passes: how many paths, which way round, and the travel through the air
  eh3d-isocam drc LAYER.gtl [--drills F.ncd]     the design rules, with the place of every finding
  eh3d-isocam drills IN.ncd OUT.ncd [--align]    read a drill file and write it back (turned over, repeated on a panel, with alignment holes)
  eh3d-isocam route OUTLINE OUT.plt|OUT.ncd      the path of the tool round the board (OUTLINE is a Gerber outline layer, a .dxf or a .plt)
  eh3d-isocam config                             every setting with its default, as a configuration file

Every option is a setting of the configuration file: --config FILE reads one, and --fine-tool-mm 0.15 is fine_tool_mm = 0.15 (the old --tool, --passes, --stepover, --res,
--depth, --feed, --plunge, --safe and --rpm still work). Also --outline FILE and --drills FILE name the outline and the holes that go with the layer, --align adds the holes that
hold the board when it is turned over, and --force writes although the tool does not fit between pieces of copper.
isolate refuses to write when the tool does not fit between pieces of copper (run check to see where) unless --force is given.";

#[derive(Clone)]
struct Cli {
    positional: Vec<String>,
    settings: Settings,
    force: bool,
    align: bool,
    outline: Option<String>,
    drills: Option<String>,
}

fn parse_cli(args: Vec<String>) -> Result<Cli, String> {
    let mut cli = Cli {
        positional: Vec::new(),
        settings: Settings::default(),
        force: false,
        align: false,
        outline: None,
        drills: None,
    };
    // the configuration file first, so that the options on the command line win over it
    if let Some(i) = args.iter().position(|a| a == "--config") {
        let file = args
            .get(i + 1)
            .ok_or_else(|| "--config needs a file".to_string())?;
        let text = fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        cli.settings = Settings::parse(&text).map_err(|e| format!("{file}: {e}"))?;
    }
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--force" => cli.force = true,
            "--align" => cli.align = true,
            "--config" => {
                it.next();
            }
            "--outline" | "--drills" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a file"))?;
                if a == "--outline" {
                    cli.outline = Some(v);
                } else {
                    cli.drills = Some(v);
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

fn arg<'a>(cli: &'a Cli, i: usize, what: &str) -> Result<&'a str, String> {
    cli.positional
        .get(i)
        .map(String::as_str)
        .ok_or_else(|| what.to_string())
}

/// The layer, with the holes and the outline that go with it, laid out as the settings say. Returns the box of the whole panel as well.
fn load_board(
    cli: &Cli,
    layer: Option<&str>,
) -> Result<(Board, eh3d_gerberviewer::scene::Bounds), String> {
    let mut board = Board::default();
    if let Some(l) = layer {
        board.scene = from_gerber(&eh3d_parser_gerber::parse(&read(l)?));
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
            "{} {} (tool {} mm, {:.3} mm from the copper): {} closed paths ({} clockwise, {} counterclockwise), {} open runs",
            if p.rubout { "ring" } else { "pass" },
            k + 1,
            p.diameter_mm,
            p.offset_mm,
            cw + ccw,
            cw,
            ccw,
            open
        );
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
        "check" | "isolate" | "info" => {
            let layer = arg(&cli, 0, &format!("{cmd} needs a Gerber layer"))?;
            let (board, _) = load_board(&cli, Some(layer))?;
            let pl = s.plan()?;
            pl.validate()?;
            let close = close_pieces(&board.scene, pl.finest_mm(), pl.res_mm)?;
            if cmd == "check" {
                for c in &close {
                    println!(
                        "{} pieces of copper are closer than {} mm near ({:.3}, {:.3})",
                        c.pieces,
                        pl.finest_mm(),
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
            if cmd == "isolate" && !close.is_empty() && !cli.force {
                return Err(format!("the tool does not fit between pieces of copper in {} places (run check to see them, or --force)", close.len()));
            }
            let passes = plan(&board.scene, &pl)?;
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
                    "travel through the air: {:.0} mm in the order they were traced, {:.0} mm with ordering = {}",
                    travel_mm(&as_traced, order.start),
                    travel_mm(&chosen, order.start),
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
                write_with(&passes, &s.job(), &order)?
            };
            write_text(out, text)?;
            let total: f64 = passes
                .iter()
                .flat_map(Pass::cuts)
                .map(|c| c.length_mm())
                .sum();
            println!(
                "{} passes, {} paths, {:.0} mm of cut written to {out}",
                passes.len(),
                passes.iter().map(|ps| ps.cuts().len()).sum::<usize>(),
                total
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
            let (board, _) = load_board(&cli2, None)?;
            let outline = eh3d_isocam::drill::Outline {
                closed: board.outline,
                open: Vec::new(),
            };
            let pass = route_outline(
                &outline,
                s.route_tool_mm,
                s.res_mm.min(s.route_tool_mm / 2.0),
            )?;
            let order = s.order();
            let text = if is_plot(out) {
                hpgl_with(std::slice::from_ref(&pass), &order, PenBy::Pass)?
            } else {
                let (cuts, _) = arrange(&pass.cuts(), &order, order.start);
                write_route_excellon(&cuts, s.route_tool_mm, 1)?
            };
            write_text(out, text)?;
            println!(
                "{} paths, {:.0} mm of cut written to {out}",
                pass.paths.len(),
                pass.cuts().iter().map(Cut::length_mm).sum::<f64>()
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
