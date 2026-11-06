//! The G-code of the passes: the tool goes up to the safe height, travels, goes down to the depth of the cut and follows the path. The order, the direction and the starting points come from the settings; by default the nearest path is cut next.

use crate::contour::Loop;
use crate::isolate::Pass;
use crate::order::{arrange, Order};
use crate::units::Unit;
use eh3d_parser_gcode::emit::number;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Job {
    /// The height at which the tool travels, above the board.
    pub safe_z_mm: f64,
    /// The height the tool goes up to when it has to be changed by hand: high enough to take it out and put the new one in.
    pub tool_change_z_mm: f64,
    /// How deep the tool goes into the copper (a positive number; the Z of the cut is its negative).
    pub depth_mm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub spindle_rpm: f64,
    /// Seconds to wait (`G4`) after each `M3` for the spindle to reach its speed; 0 writes no wait.
    pub spindle_wait_s: f64,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            safe_z_mm: 2.0,
            tool_change_z_mm: 30.0,
            depth_mm: 0.06,
            feed_mm_min: 300.0,
            plunge_mm_min: 60.0,
            spindle_rpm: 24000.0,
            spindle_wait_s: 0.0,
        }
    }
}

impl Job {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("safe_z_mm", self.safe_z_mm, 0.5, 50.0)?;
        range(
            "tool_change_z_mm",
            self.tool_change_z_mm,
            self.safe_z_mm,
            200.0,
        )?;
        range("depth_mm", self.depth_mm, 0.005, 2.0)?;
        range("feed_mm_min", self.feed_mm_min, 1.0, 10_000.0)?;
        range("plunge_mm_min", self.plunge_mm_min, 1.0, 5_000.0)?;
        range("spindle_rpm", self.spindle_rpm, 1000.0, 200_000.0)?;
        range("spindle_wait_s", self.spindle_wait_s, 0.0, 60.0)
    }
}

/// A number of the file, written the way the whole suite writes it. [`write_in`] checks beforehand that every number it writes is finite, so the error cannot happen here.
fn n(v: f64) -> String {
    number(v).unwrap_or_default()
}

/// A length for the file: three decimals in mm, five in inches.
fn nn(v: f64, unit: Unit) -> String {
    if unit != Unit::Inch {
        return n(v);
    }
    let s = format!("{v:.5}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// The time a job takes, estimated from the speeds of the settings (not from a machine): the seconds of cutting, of going down into the material and of the rapid moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeEstimate {
    pub cutting_s: f64,
    pub plunging_s: f64,
    pub rapid_s: f64,
}

impl TimeEstimate {
    pub fn total_s(&self) -> f64 {
        self.cutting_s + self.plunging_s + self.rapid_s
    }
}

/// The time of cutting `cut_mm` of path in `paths` runs with `travel_mm` of travel through the air between them. Every run goes down from the safe height to the depth of the cut at the
/// plunge feed and comes back up at the rapid speed; spindle waits and tool changes by hand are not counted. A rapid speed that is not above zero is refused.
pub fn estimate_time(
    job: &Job,
    rapid_mm_min: f64,
    cut_mm: f64,
    travel_mm: f64,
    paths: usize,
) -> Result<TimeEstimate, String> {
    let ok = |v: f64| v.is_finite() && v >= 0.0;
    if !(rapid_mm_min.is_finite() && rapid_mm_min > 0.0) {
        return Err("rapid_mm_min must be above 0 to estimate the time".into());
    }
    if !(ok(cut_mm) && ok(travel_mm)) {
        return Err("the lengths of the job must be finite and not negative".into());
    }
    if !(job.feed_mm_min > 0.0 && job.plunge_mm_min > 0.0) {
        return Err("the feed and the plunge feed must be above 0 to estimate the time".into());
    }
    let down_mm = job.safe_z_mm + job.depth_mm;
    let runs = paths as f64;
    Ok(TimeEstimate {
        cutting_s: cut_mm / job.feed_mm_min * 60.0,
        plunging_s: runs * down_mm / job.plunge_mm_min * 60.0,
        rapid_s: (travel_mm + runs * down_mm) / rapid_mm_min * 60.0,
    })
}

/// A time for a person: `45 s`, `2 min 27 s`, `1 h 2 min` (the seconds are rounded first, and the seconds are left out from an hour on).
pub fn duration_text(seconds: f64) -> String {
    let total = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    if h > 0 {
        format!("{h} h {m} min")
    } else if m > 0 {
        format!("{m} min {s} s")
    } else {
        format!("{s} s")
    }
}

/// The total length of the closed paths, in mm.
pub fn length(paths: &[Loop]) -> f64 {
    paths
        .iter()
        .map(|p| {
            let mut sum: f64 = p
                .windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum();
            if let (Some(a), Some(b)) = (p.first(), p.last()) {
                sum += (a.0 - b.0).hypot(a.1 - b.1);
            }
            sum
        })
        .sum()
}

/// The passes as G-code in the default order: the nearest path next, each starting at its point nearest to where the tool is, going the way they were traced.
pub fn write(passes: &[Pass], job: &Job) -> Result<String, String> {
    write_with(passes, job, &Order::default())
}

/// The passes as G-code, a pass after the other, with the paths in the order, direction and starting points that `order` says. When the width of the tool changes from one pass to
/// the next the program stops (`M0`) for the tool to be changed by hand.
pub fn write_with(passes: &[Pass], job: &Job, order: &Order) -> Result<String, String> {
    write_in(passes, job, order, Unit::Mm)
}

/// The same, in millimetres (`G21`) or in inches (`G20`: every length and the feeds are converted). G-code has no mils.
pub fn write_in(passes: &[Pass], job: &Job, order: &Order, unit: Unit) -> Result<String, String> {
    job.validate()?;
    // a number that is not finite would be written as `NaN` or `inf`, which a controller chokes on: nothing is written
    if passes.iter().any(|p| {
        !(p.offset_mm.is_finite() && p.diameter_mm.is_finite())
            || p.cuts()
                .iter()
                .any(|c| c.pts.iter().any(|q| !(q.0.is_finite() && q.1.is_finite())))
    }) {
        return Err("a path has a coordinate that is not a number: nothing was written".into());
    }
    if unit == Unit::Mil {
        return Err("G-code is written in mm or inch, not in mils".into());
    }
    let c = |v: f64| unit.from_mm(v);
    if !(order.start.0.is_finite() && order.start.1.is_finite()) {
        return Err("the starting point of the tool is not a number".into());
    }
    // the spindle needs time to reach its speed before the tool goes down (some controllers do not wait after M3)
    let wait = if job.spindle_wait_s > 0.0 {
        format!("G4 P{}\n", n(job.spindle_wait_s))
    } else {
        String::new()
    };
    let mut g = format!(
        "{}\nG90\nG0 Z{}\nM3 S{}\n{wait}",
        if unit == Unit::Inch { "G20" } else { "G21" },
        nn(c(job.safe_z_mm), unit),
        n(job.spindle_rpm)
    );
    let mut at = order.start;
    let mut tool: Option<f64> = None;
    for (k, pass) in passes.iter().enumerate() {
        let (cuts, end) = arrange(&pass.cuts(), order, at);
        if cuts.is_empty() {
            continue;
        }
        if pass.diameter_mm > 0.0 {
            if tool.is_some_and(|t| (t - pass.diameter_mm).abs() > 1e-9) {
                g.push_str(&format!(
                    "M5\nG0 Z{}\n(change to a tool of {} {}; set the Z zero again with the new tool before pressing start)\nM0\nM3 S{}\n{wait}G0 Z{}\n",
                    nn(c(job.tool_change_z_mm), unit),
                    nn(c(pass.diameter_mm), unit),
                    if unit == Unit::Inch { "in" } else { "mm" },
                    n(job.spindle_rpm),
                    nn(c(job.safe_z_mm), unit)
                ));
            }
            tool = Some(pass.diameter_mm);
        }
        g.push_str(&format!(
            "({} {} at {} {} from the copper, {} paths)\n",
            if pass.rubout { "ring" } else { "pass" },
            k + 1,
            nn(c(pass.offset_mm), unit),
            if unit == Unit::Inch { "in" } else { "mm" },
            cuts.len()
        ));
        for cut in &cuts {
            let start = cut.pts[0];
            g.push_str(&format!(
                "G0 X{} Y{}\nG1 Z{} F{}\n",
                nn(c(start.0), unit),
                nn(c(start.1), unit),
                nn(c(-job.depth_mm), unit),
                nn(c(job.plunge_mm_min), unit)
            ));
            let count = if cut.closed {
                cut.pts.len()
            } else {
                cut.pts.len() - 1
            };
            for j in 1..=count {
                let p = cut.pts[j % cut.pts.len()];
                if j == 1 {
                    g.push_str(&format!(
                        "G1 X{} Y{} F{}\n",
                        nn(c(p.0), unit),
                        nn(c(p.1), unit),
                        nn(c(job.feed_mm_min), unit)
                    ));
                } else {
                    g.push_str(&format!("G1 X{} Y{}\n", nn(c(p.0), unit), nn(c(p.1), unit)));
                }
            }
            g.push_str(&format!("G0 Z{}\n", nn(c(job.safe_z_mm), unit)));
        }
        at = end;
    }
    g.push_str("M5\nM2\n");
    Ok(g)
}
