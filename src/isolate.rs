//! The passes of the tool around the copper, and the places where two pieces of copper are too close for the tool to go between them.

use crate::bitmap::from_scene;
use crate::contour::{loops, simplify, Grid, Loop};
use crate::edt::distance_mm;
use eh3d_gerberviewer::scene::Scene;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// The width of the tool at the depth of the cut.
    pub tool_diameter_mm: f64,
    /// How many passes, each one a stepover farther from the copper than the one before.
    pub passes: usize,
    pub stepover_mm: f64,
    /// The size of a cell of the grid: the paths are within this of the exact ones.
    pub res_mm: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            tool_diameter_mm: 0.2,
            passes: 1,
            stepover_mm: 0.1,
            res_mm: 0.02,
        }
    }
}

impl Params {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("tool_diameter_mm", self.tool_diameter_mm, 0.02, 10.0)?;
        range("stepover_mm", self.stepover_mm, 0.01, 10.0)?;
        range("res_mm", self.res_mm, 0.002, 1.0)?;
        if !(1..=10).contains(&self.passes) {
            return Err(format!(
                "passes is {}: it must be from 1 to 10",
                self.passes
            ));
        }
        if self.res_mm > self.tool_diameter_mm / 2.0 {
            return Err(format!("the cell of {} mm is too coarse for a tool of {} mm: it must be at most half of it", self.res_mm, self.tool_diameter_mm));
        }
        Ok(())
    }

    /// The farthest the tool goes from the copper, centre of the tool to the edge of the copper.
    pub fn reach_mm(&self) -> f64 {
        self.tool_diameter_mm / 2.0 + (self.passes - 1) as f64 * self.stepover_mm
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    /// The distance from the edge of the copper to the centre of the tool.
    pub offset_mm: f64,
    pub paths: Vec<Loop>,
}

/// The passes, from the first one (the tool touching the copper) outwards.
pub fn isolate(scene: &Scene, p: &Params) -> Result<Vec<Pass>, String> {
    p.validate()?;
    let bm = from_scene(scene, p.reach_mm() + 4.0 * p.res_mm, p.res_mm)?;
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    let grid = Grid {
        dist: &dist,
        w: bm.w,
        h: bm.h,
        res: bm.res,
        x0: bm.x0,
        y0: bm.y0,
    };
    Ok((0..p.passes)
        .map(|k| {
            let offset = p.tool_diameter_mm / 2.0 + k as f64 * p.stepover_mm;
            // the distance is measured between centres of cells, which puts the edge of the copper half a cell away
            let paths = loops(&grid, offset + p.res_mm / 2.0)
                .into_iter()
                .map(|l| simplify(&l, p.res_mm / 2.0))
                .filter(|l| l.len() >= 3)
                .collect();
            Pass {
                offset_mm: offset,
                paths,
            }
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Closeness {
    /// Where the pieces come close, in mm.
    pub x: f64,
    pub y: f64,
    /// How many separate pieces of copper are closer than the limit there.
    pub pieces: usize,
}

/// Labels the connected cells of a grid (8 neighbours): the number of each cell's piece (0 for no copper) and how many pieces there are.
fn label(on: &[bool], w: usize, h: usize) -> (Vec<u32>, u32) {
    let (mut id, mut n, mut stack) = (vec![0u32; w * h], 0u32, Vec::new());
    for s in 0..w * h {
        if !on[s] || id[s] != 0 {
            continue;
        }
        n += 1;
        id[s] = n;
        stack.push(s);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    let j = ny as usize * w + nx as usize;
                    if on[j] && id[j] == 0 {
                        id[j] = n;
                        stack.push(j);
                    }
                }
            }
        }
    }
    (id, n)
}

/// The places where separate pieces of copper are closer than `min_gap_mm`: a tool as wide as that gap cannot go between them, so the isolation would cut into the copper or leave a bridge.
pub fn close_pieces(scene: &Scene, min_gap_mm: f64, res_mm: f64) -> Result<Vec<Closeness>, String> {
    if !(min_gap_mm.is_finite() && min_gap_mm > 0.0) {
        return Err(format!(
            "min_gap_mm is {min_gap_mm}: it must be more than 0"
        ));
    }
    let bm = from_scene(scene, min_gap_mm + 4.0 * res_mm, res_mm)?;
    let (pieces, count) = label(&bm.on, bm.w, bm.h);
    if count < 2 {
        return Ok(Vec::new());
    }
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    // every cell within half of the gap of the copper joins the pieces that are closer than the gap
    let reach = min_gap_mm / 2.0 + res_mm / 2.0;
    let grown: Vec<bool> = dist.iter().map(|d| f64::from(*d) <= reach).collect();
    let (merged, merged_count) = label(&grown, bm.w, bm.h);
    let mut out = Vec::new();
    for m in 1..=merged_count {
        let mut originals: Vec<u32> = (0..bm.w * bm.h)
            .filter(|&i| merged[i] == m && pieces[i] != 0)
            .map(|i| pieces[i])
            .collect();
        originals.sort_unstable();
        originals.dedup();
        if originals.len() >= 2 {
            let cells: Vec<usize> = (0..bm.w * bm.h).filter(|&i| merged[i] == m).collect();
            let (sx, sy) = cells.iter().fold((0.0, 0.0), |a, &i| {
                (a.0 + (i % bm.w) as f64, a.1 + (i / bm.w) as f64)
            });
            let (x, y) = bm.centre(sx / cells.len() as f64, sy / cells.len() as f64);
            out.push(Closeness {
                x,
                y,
                pieces: originals.len(),
            });
        }
    }
    Ok(out)
}
