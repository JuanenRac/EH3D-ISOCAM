//! The passes as an HP-GL plot file, the format the routing programs of the Bungard machines read: 40 units to the millimetre, one pen for each pass (the pen is the
//! tool of the machine), the tool up while it travels and down along a closed path.

use crate::contour::Loop;
use crate::isolate::Pass;

/// Units of HP-GL to the millimetre.
pub const UNITS_PER_MM: f64 = 40.0;
/// The most pens a plotter file may use.
pub const MAX_PENS: usize = 8;

fn units(v: f64) -> i64 {
    (v * UNITS_PER_MM).round() as i64
}

/// The passes as HP-GL: pass `k` is drawn with pen `k + 1`. Coordinates may not be negative in a plot file, so the paths are moved by `(dx, dy)` mm when they are.
pub fn write(passes: &[Pass]) -> Result<String, String> {
    if passes.len() > MAX_PENS {
        return Err(format!(
            "{} passes: a plot file has at most {MAX_PENS} pens",
            passes.len()
        ));
    }
    let all: Vec<&Loop> = passes.iter().flat_map(|p| p.paths.iter()).collect();
    let min_x = all
        .iter()
        .flat_map(|p| p.iter().map(|q| q.0))
        .fold(f64::INFINITY, f64::min);
    let min_y = all
        .iter()
        .flat_map(|p| p.iter().map(|q| q.1))
        .fold(f64::INFINITY, f64::min);
    let (dx, dy) = (
        if min_x < 0.0 { -min_x } else { 0.0 },
        if min_y < 0.0 { -min_y } else { 0.0 },
    );
    let mut s = String::from("IN;\n");
    for (k, pass) in passes.iter().enumerate() {
        if pass.paths.iter().all(|p| p.len() < 3) {
            continue;
        }
        s.push_str(&format!("SP{};\n", k + 1));
        let mut at = (0.0, 0.0);
        let mut left: Vec<&Loop> = pass.paths.iter().filter(|p| p.len() >= 3).collect();
        while !left.is_empty() {
            let d = |p: &Loop| (p[0].0 - at.0).powi(2) + (p[0].1 - at.1).powi(2);
            let i = left
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    d(a.1)
                        .partial_cmp(&d(b.1))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            let path = left.remove(i);
            s.push_str(&format!(
                "PU{},{};\nPD",
                units(path[0].0 + dx),
                units(path[0].1 + dy)
            ));
            let pts: Vec<String> = path
                .iter()
                .skip(1)
                .chain(std::iter::once(&path[0]))
                .map(|q| format!("{},{}", units(q.0 + dx), units(q.1 + dy)))
                .collect();
            s.push_str(&pts.join(","));
            s.push_str(";\n");
            at = path[0];
        }
    }
    s.push_str("PU0,0;SP0;\n");
    Ok(s)
}
