//! The G-code of the passes: the tool goes up to the safe height, travels, goes down to the depth of the cut and follows the closed path. The nearest path is cut next.

use crate::contour::Loop;
use crate::isolate::Pass;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Job {
    /// The height at which the tool travels, above the board.
    pub safe_z_mm: f64,
    /// How deep the tool goes into the copper (a positive number; the Z of the cut is its negative).
    pub depth_mm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub spindle_rpm: f64,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            safe_z_mm: 2.0,
            depth_mm: 0.06,
            feed_mm_min: 300.0,
            plunge_mm_min: 60.0,
            spindle_rpm: 24000.0,
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
        range("depth_mm", self.depth_mm, 0.005, 2.0)?;
        range("feed_mm_min", self.feed_mm_min, 1.0, 10_000.0)?;
        range("plunge_mm_min", self.plunge_mm_min, 1.0, 5_000.0)?;
        range("spindle_rpm", self.spindle_rpm, 1000.0, 200_000.0)
    }
}

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
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

/// The passes as G-code, a pass after the other, each path starting at its point nearest to where the tool is.
pub fn write(passes: &[Pass], job: &Job) -> Result<String, String> {
    job.validate()?;
    let mut g = format!(
        "G21\nG90\nG0 Z{}\nM3 S{}\n",
        n(job.safe_z_mm),
        n(job.spindle_rpm)
    );
    let mut at = (0.0, 0.0);
    for (k, pass) in passes.iter().enumerate() {
        g.push_str(&format!(
            "(pass {} at {} mm from the copper, {} paths)\n",
            k + 1,
            n(pass.offset_mm),
            pass.paths.len()
        ));
        let mut left: Vec<&Loop> = pass.paths.iter().filter(|p| p.len() >= 3).collect();
        while !left.is_empty() {
            let d = |p: &Loop| {
                p.iter()
                    .map(|q| (q.0 - at.0).powi(2) + (q.1 - at.1).powi(2))
                    .fold(f64::INFINITY, f64::min)
            };
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
            let s = (0..path.len())
                .min_by(|&a, &b| {
                    ((path[a].0 - at.0).powi(2) + (path[a].1 - at.1).powi(2))
                        .partial_cmp(&((path[b].0 - at.0).powi(2) + (path[b].1 - at.1).powi(2)))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            let start = path[s];
            g.push_str(&format!(
                "G0 X{} Y{}\nG1 Z{} F{}\n",
                n(start.0),
                n(start.1),
                n(-job.depth_mm),
                n(job.plunge_mm_min)
            ));
            for j in 1..=path.len() {
                let p = path[(s + j) % path.len()];
                if j == 1 {
                    g.push_str(&format!(
                        "G1 X{} Y{} F{}\n",
                        n(p.0),
                        n(p.1),
                        n(job.feed_mm_min)
                    ));
                } else {
                    g.push_str(&format!("G1 X{} Y{}\n", n(p.0), n(p.1)));
                }
            }
            g.push_str(&format!("G0 Z{}\n", n(job.safe_z_mm)));
            at = start;
        }
    }
    g.push_str("M5\nM2\n");
    Ok(g)
}
