// EH3D-ISOCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::scene::from_gerber;
use eh3d_isocam::gcode::{length, write, Job};
use eh3d_isocam::isolate::{close_pieces, isolate, Params};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-isocam: isolation milling of copper from a Gerber layer

  eh3d-isocam isolate LAYER.gtl OUT.nc [--tool 0.2] [--passes 1] [--stepover 0.1] [--res 0.02] [--depth 0.06] [--feed 300] [--plunge 60] [--safe 2] [--rpm 24000] [--force]
  eh3d-isocam check LAYER.gtl [--tool 0.2] [--res 0.02]     the places where two pieces of copper are closer than the tool is wide

isolate refuses to write when the tool does not fit between pieces of copper (run check to see where) unless --force is given.
The paths are the lines at the radius of the tool (plus one stepover for each further pass) from the edge of the copper, within --res of the exact ones.";

fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<f64>, String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        let v = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?
            .clone();
        args.drain(i..=i + 1);
        return v
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{name} '{v}' is not a number"));
    }
    Ok(None)
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    let force = args
        .iter()
        .position(|a| a == "--force")
        .map(|i| args.remove(i))
        .is_some();
    let mut p = Params::default();
    let mut job = Job::default();
    if let Some(v) = flag(&mut args, "--tool")? {
        p.tool_diameter_mm = v;
    }
    if let Some(v) = flag(&mut args, "--passes")? {
        p.passes = v as usize;
    }
    if let Some(v) = flag(&mut args, "--stepover")? {
        p.stepover_mm = v;
    }
    if let Some(v) = flag(&mut args, "--res")? {
        p.res_mm = v;
    }
    if let Some(v) = flag(&mut args, "--depth")? {
        job.depth_mm = v;
    }
    if let Some(v) = flag(&mut args, "--feed")? {
        job.feed_mm_min = v;
    }
    if let Some(v) = flag(&mut args, "--plunge")? {
        job.plunge_mm_min = v;
    }
    if let Some(v) = flag(&mut args, "--safe")? {
        job.safe_z_mm = v;
    }
    if let Some(v) = flag(&mut args, "--rpm")? {
        job.spindle_rpm = v;
    }
    let layer = args
        .first()
        .ok_or_else(|| format!("{cmd} needs a Gerber layer"))?
        .clone();
    let bytes = fs::read(&layer).map_err(|e| format!("{layer}: {e}"))?;
    let scene = from_gerber(&eh3d_parser_gerber::parse(&String::from_utf8_lossy(&bytes)));
    for n in &scene.notes {
        eprintln!("note: {n}");
    }
    p.validate()?;
    let close = close_pieces(&scene, p.tool_diameter_mm, p.res_mm)?;
    match cmd.as_str() {
        "check" => {
            for c in &close {
                println!(
                    "{} pieces of copper are closer than {} mm near ({:.3}, {:.3})",
                    c.pieces, p.tool_diameter_mm, c.x, c.y
                );
            }
            if close.is_empty() {
                println!("the tool fits between every piece of copper");
                Ok(())
            } else {
                Err(format!(
                    "{} places where the tool does not fit",
                    close.len()
                ))
            }
        }
        "isolate" => {
            let out = args.get(1).ok_or("isolate needs an output file")?;
            if !close.is_empty() && !force {
                return Err(format!("the tool does not fit between pieces of copper in {} places (run check to see them, or --force)", close.len()));
            }
            let passes = isolate(&scene, &p)?;
            let text = write(&passes, &job)?;
            fs::write(out, text).map_err(|e| format!("{out}: {e}"))?;
            let total: f64 = passes.iter().map(|ps| length(&ps.paths)).sum();
            println!(
                "{} passes, {} paths, {:.0} mm of cut written to {out}",
                passes.len(),
                passes.iter().map(|ps| ps.paths.len()).sum::<usize>(),
                total
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
