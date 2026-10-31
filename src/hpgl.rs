//! The passes as an HP-GL plot file, the format the routing programs of the Bungard machines read: 40 units to the millimetre, one pen for each pass (or for each tool: the pen
//! is the tool of the machine), the tool up while it travels and down along a path.

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
