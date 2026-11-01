//! The passes as an HP-GL plot file, the format the routing programs of the Bungard machines read: 40 units to the millimetre, one pen for each pass (or for each tool: the pen
//! is the tool of the machine), the tool up while it travels and down along a path.

use crate::drill::Hole;
use crate::isolate::Pass;
use crate::order::{arrange, Order};

/// Units of HP-GL to the millimetre.
pub const UNITS_PER_MM: f64 = 40.0;
/// The most pens a plot file may use.
pub const MAX_PENS: usize = 8;

/// What a pen stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenBy {
    /// A new pen for each pass.
    Pass,
    /// A pen for each tool: the passes of one tool share it.
    Tool,
}

fn units(v: f64) -> i64 {
    (v * UNITS_PER_MM).round() as i64
}

/// The passes as HP-GL, one pen for each pass, in the default order (the path whose first point is nearest next). Coordinates may not be negative in a plot file, so the paths
/// are moved when they are.
pub fn write(passes: &[Pass]) -> Result<String, String> {
    let order = Order {
        loop_start: crate::order::LoopStart::Original,
        ..Order::default()
    };
    write_with(passes, &order, PenBy::Pass)
}

/// The passes as HP-GL with the paths in the order, direction and starting points that `order` says.
pub fn write_with(passes: &[Pass], order: &Order, pens: PenBy) -> Result<String, String> {
    let pen_of = |k: usize, p: &Pass| match pens {
        PenBy::Pass => k + 1,
        PenBy::Tool => p.tool + 1,
    };
    let most = passes
        .iter()
        .enumerate()
        .map(|(k, p)| pen_of(k, p))
        .max()
        .unwrap_or(0);
    if most > MAX_PENS {
        return Err(format!(
            "{most} pens needed: a plot file has at most {MAX_PENS}"
        ));
    }
    if !(order.start.0.is_finite() && order.start.1.is_finite()) {
        return Err("the starting point of the tool is not a number".into());
    }
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    for c in passes.iter().flat_map(|p| p.cuts()) {
        for q in &c.pts {
            min_x = min_x.min(q.0);
            min_y = min_y.min(q.1);
        }
    }
    let (dx, dy) = (
        if min_x < 0.0 { -min_x } else { 0.0 },
        if min_y < 0.0 { -min_y } else { 0.0 },
    );
    let mut s = String::from("IN;\n");
    let mut at = order.start;
    let mut pen = 0;
    for (k, pass) in passes.iter().enumerate() {
        let (cuts, end) = arrange(&pass.cuts(), order, at);
        if cuts.is_empty() {
            continue;
        }
        if pen_of(k, pass) != pen {
            pen = pen_of(k, pass);
            s.push_str(&format!("SP{pen};\n"));
        }
        for cut in &cuts {
            s.push_str(&format!(
                "PU{},{};\nPD",
                units(cut.pts[0].0 + dx),
                units(cut.pts[0].1 + dy)
            ));
            let mut pts: Vec<String> = cut
                .pts
                .iter()
                .skip(1)
                .map(|q| format!("{},{}", units(q.0 + dx), units(q.1 + dy)))
                .collect();
            if cut.closed {
                pts.push(format!(
                    "{},{}",
                    units(cut.pts[0].0 + dx),
                    units(cut.pts[0].1 + dy)
                ));
            }
            s.push_str(&pts.join(","));
            s.push_str(";\n");
        }
        at = end;
    }
    s.push_str("PU0,0;SP0;\n");
    Ok(s)
}

/// The pen speed of a plot file in centimetres a second: `0` leaves the plot as it is (the machine's own speed), otherwise `VS` is put after the initialisation. It is a setting
/// (`hpgl_pen_speed_cm_s`) because the right value belongs to the machine.
pub fn set_pen_speed(plot: &str, cm_s: f64) -> Result<String, String> {
    if cm_s == 0.0 {
        return Ok(plot.to_string());
    }
    if !(cm_s.is_finite() && (0.1..=100.0).contains(&cm_s)) {
        return Err(format!(
            "the pen speed is {cm_s} cm/s: it must be 0 (off) or from 0.1 to 100"
        ));
    }
    let v = format!("{cm_s:.2}");
    let v = v.trim_end_matches('0').trim_end_matches('.');
    Ok(plot.replacen("IN;\n", &format!("IN;\nVS{v};\n"), 1))
}

/// Reads a map of drill tools to pens: `tool:pen` pairs separated by commas (`1:2, 3:1`). Empty is no map.
pub fn parse_pen_map(v: &str) -> Result<Vec<(u8, u8)>, String> {
    let mut out: Vec<(u8, u8)> = Vec::new();
    for part in v.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (t, p) = part
            .split_once(':')
            .ok_or_else(|| format!("'{part}' is not tool:pen"))?;
        let tool: u8 = t
            .trim()
            .parse()
            .map_err(|_| format!("'{t}' is not a tool number from 1 to 255"))?;
        let pen: u8 = p
            .trim()
            .parse()
            .map_err(|_| format!("'{p}' is not a pen number"))?;
        if tool == 0 || pen == 0 || usize::from(pen) > MAX_PENS {
            return Err(format!(
                "'{part}': the tool is from 1 to 255 and the pen from 1 to {MAX_PENS}"
            ));
        }
        if out.iter().any(|(t0, _)| *t0 == tool) {
            return Err(format!("tool {tool} is mapped twice"));
        }
        out.push((tool, pen));
    }
    Ok(out)
}

/// The text of a map of tools to pens, as `parse_pen_map` reads it.
pub fn pen_map_text(map: &[(u8, u8)]) -> String {
    map.iter()
        .map(|(t, p)| format!("{t}:{p}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The pen of each tool of a list of holes: the tools in the map get their pen, the others the lowest pens that are free, in the order of their tool numbers.
pub fn pens_for(tools: &[u8], map: &[(u8, u8)]) -> Result<Vec<(u8, u8)>, String> {
    let mut sorted: Vec<u8> = tools.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut out: Vec<(u8, u8)> = Vec::new();
    for t in &sorted {
        if let Some((_, p)) = map.iter().find(|(m, _)| m == t) {
            out.push((*t, *p));
        }
    }
    let mut next = 1u8;
    for t in &sorted {
        if out.iter().any(|(o, _)| o == t) {
            continue;
        }
        while out.iter().any(|(_, p)| *p == next) || map.iter().any(|(_, p)| *p == next) {
            next += 1;
        }
        if usize::from(next) > MAX_PENS {
            return Err(format!(
                "more than {MAX_PENS} pens are needed for the drill tools: map them in hpgl_pen_map"
            ));
        }
        out.push((*t, next));
    }
    Ok(out)
}

/// The holes as a plot file: one pen for each drill tool (`map` gives the pen of a tool, the others take the lowest free pens), and at each hole the pen is put down and lifted.
pub fn write_holes(holes: &[Hole], map: &[(u8, u8)]) -> Result<String, String> {
    if holes.is_empty() {
        return Err("there are no holes to write".into());
    }
    if holes
        .iter()
        .any(|h| !(h.x_mm.is_finite() && h.y_mm.is_finite()))
    {
        return Err("a hole has a coordinate that is not a number".into());
    }
    let tools: Vec<u8> = holes.iter().map(|h| h.tool).collect();
    let pens = pens_for(&tools, map)?;
    let min_x = holes.iter().map(|h| h.x_mm).fold(f64::INFINITY, f64::min);
    let min_y = holes.iter().map(|h| h.y_mm).fold(f64::INFINITY, f64::min);
    let (dx, dy) = (
        if min_x < 0.0 { -min_x } else { 0.0 },
        if min_y < 0.0 { -min_y } else { 0.0 },
    );
    let mut order: Vec<(u8, u8)> = pens.clone();
    order.sort_by_key(|(_, p)| *p);
    let mut s = String::from("IN;\n");
    for (tool, pen) in order {
        s.push_str(&format!("SP{pen};\n"));
        for h in holes.iter().filter(|h| h.tool == tool) {
            s.push_str(&format!(
                "PU{},{};PD;PU;\n",
                units(h.x_mm + dx),
                units(h.y_mm + dy)
            ));
        }
    }
    s.push_str("PU0,0;SP0;\n");
    Ok(s)
}
