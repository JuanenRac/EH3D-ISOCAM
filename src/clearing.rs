//! Filling an area with rings: closed paths that follow the edge of the area inwards, one tool width apart less the overlap, so that the tool cuts round and round instead of back and
//! forth. The area is a set of closed contours (an even-odd fill, so a contour inside another one is a hole). The order of the rings (the outside first or the middle first) and
//! the overlap are chosen; the rings are the lines where the distance to the edge equals the radius of the tool plus a ring's step, found on the same grid and with the same exact
//! distance as the isolation passes.

use crate::bitmap::Bitmap;
use crate::contour::{loops, simplify, Grid, Loop};
use crate::edt::distance_mm;
use crate::isolate::Pass;
use eh3d_gerberviewer::scene::Bounds;

/// The most rings one fill may have.
pub const MAX_RINGS: usize = 2000;

/// How an area is cleared: with parallel lines back and forth, or with rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Lines,
    Rings,
}

impl Style {
    /// The value of the setting `rubout_style`: `lines` or `rings`.
    pub fn parse(v: &str) -> Result<Style, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "lines" | "line" => Ok(Style::Lines),
            "rings" | "ring" => Ok(Style::Rings),
            other => Err(format!("rubout_style '{other}': it must be lines or rings")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Lines => "lines",
            Style::Rings => "rings",
        }
    }
}

/// The order the rings are cut in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RingOrder {
    /// The ring at the edge first, then the ones inside it.
    #[default]
    OutsideIn,
    /// The innermost ring first, then the ones round it.
    InsideOut,
}

impl RingOrder {
    /// The value of the setting `rubout_ring_order`: `outside-in` or `inside-out`.
    pub fn parse(v: &str) -> Result<RingOrder, String> {
        match v.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "outside-in" | "outsidein" => Ok(RingOrder::OutsideIn),
            "inside-out" | "insideout" => Ok(RingOrder::InsideOut),
            other => Err(format!(
                "rubout_ring_order '{other}': it must be outside-in or inside-out"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RingOrder::OutsideIn => "outside-in",
            RingOrder::InsideOut => "inside-out",
        }
    }
}

/// What the rings are like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rings {
    pub tool_mm: f64,
    /// The share of the width of the tool that neighbouring rings have in common, 0 to 0.9.
    pub overlap: f64,
    pub order: RingOrder,
    /// The size of the cell the area is drawn on.
    pub res_mm: f64,
}

/// The even-odd crossings of the line `y = yc` with the contours, sorted.
fn crossings(contours: &[Loop], yc: f64) -> Vec<f64> {
    let mut xs = Vec::new();
    for c in contours {
        let n = c.len();
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            if (a.1 > yc) != (b.1 > yc) {
                xs.push(a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1));
            }
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs
}

/// The rings that fill the contours, as the closed paths of one pass. The tool stays wholly inside the area: the first ring is half a tool from the edge, the next ones a step of
/// `tool * (1 - overlap)` further in, and a last ring along the middle is added when the rings would leave a strip in it that no ring reaches. An area narrower than the tool is
/// refused.
pub fn fill_rings(contours: &[Loop], f: &Rings) -> Result<Pass, String> {
    if !(f.tool_mm.is_finite() && (0.1..=10.0).contains(&f.tool_mm)) {
        return Err(format!(
            "the tool of the rings is {} mm: it must be from 0.1 to 10",
            f.tool_mm
        ));
    }
    if !(f.overlap.is_finite() && (0.0..=0.9).contains(&f.overlap)) {
        return Err(format!(
            "the overlap is {}: it must be from 0 to 0.9",
            f.overlap
        ));
    }
    if !(f.res_mm.is_finite() && f.res_mm > 0.0) {
        return Err(format!("the cell is {} mm: it must be above 0", f.res_mm));
    }
    let closed: Vec<Loop> = contours.iter().filter(|c| c.len() >= 3).cloned().collect();
    if closed.is_empty() {
        return Err("there is no closed contour to fill".into());
    }
    let mut bounds = Bounds::empty();
    for p in closed.iter().flatten() {
        if !(p.0.is_finite() && p.1.is_finite()) {
            return Err("a contour has a coordinate that is not a number".into());
        }
        bounds.include(p.0, p.1);
    }
    let res = f.res_mm;
    let mut bm = Bitmap::new(&bounds, 2.0 * res, res)?;
    // the cells inside the contours, row by row (a cell is inside when its centre is)
    for iy in 0..bm.h {
        let (_, yc) = bm.centre(0.0, iy as f64);
        let xs = crossings(&closed, yc);
        let (pairs, _) = xs.as_chunks::<2>();
        for pair in pairs {
            let first = ((pair[0] - bm.x0) / res - 0.5).ceil().max(0.0) as usize;
            let last = ((pair[1] - bm.x0) / res - 0.5).floor();
            if last < 0.0 {
                continue;
            }
            for ix in first..=(last as usize).min(bm.w - 1) {
                bm.on[iy * bm.w + ix] = true;
            }
        }
    }
    // how deep inside every cell is: its distance to the nearest cell that is outside
    let outside: Vec<bool> = bm.on.iter().map(|c| !*c).collect();
    let depth = distance_mm(&outside, bm.w, bm.h, res);
    let deepest = depth
        .iter()
        .zip(&bm.on)
        .filter(|(_, inside)| **inside)
        .map(|(d, _)| f64::from(*d))
        .fold(0.0, f64::max);
    let r = f.tool_mm / 2.0;
    // the distance of a cell to the nearest cell outside is half a cell more than its distance to the edge
    let first = r + res / 2.0;
    if deepest < first {
        return Err(format!(
            "the contours are narrower than the tool of {} mm: no ring fits",
            f.tool_mm
        ));
    }
    let pitch = f.tool_mm * (1.0 - f.overlap);
    let mut levels = Vec::new();
    let mut level = first;
    while level <= deepest {
        if levels.len() >= MAX_RINGS {
            return Err(format!(
                "more than {MAX_RINGS} rings: use a wider tool or a smaller area"
            ));
        }
        levels.push(level);
        level += pitch;
    }
    // a strip along the middle that no ring reaches gets one more ring
    let last = levels[levels.len() - 1];
    if deepest > last + r + res && levels.len() < MAX_RINGS {
        levels.push(deepest - res / 4.0);
    }
    let grid = Grid {
        dist: &depth,
        w: bm.w,
        h: bm.h,
        res,
        x0: bm.x0,
        y0: bm.y0,
    };
    let mut rings: Vec<Loop> = Vec::new();
    for lv in levels {
        for ring in loops(&grid, lv) {
            let ring = simplify(&ring, res / 2.0);
            if ring.len() >= 3 {
                rings.push(ring);
            }
        }
    }
    if rings.is_empty() {
        return Err("the contours are narrower than the tool: no ring fits".into());
    }
    if f.order == RingOrder::InsideOut {
        rings.reverse();
    }
    Ok(Pass {
        offset_mm: 0.0,
        paths: rings,
        diameter_mm: f.tool_mm,
        rubout: true,
        ..Pass::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
        vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    }

    fn rings(tool: f64, overlap: f64, order: RingOrder) -> Rings {
        Rings {
            tool_mm: tool,
            overlap,
            order,
            res_mm: 0.05,
        }
    }

    fn bounds_of(path: &Loop) -> (f64, f64, f64, f64) {
        let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in path {
            b = (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1));
        }
        b
    }

    #[test]
    fn the_rings_of_a_rectangle_are_nested_rectangles_a_step_apart() {
        let area = vec![rect(0.0, 0.0, 20.0, 10.0)];
        let pass = fill_rings(&area, &rings(2.0, 0.0, RingOrder::OutsideIn)).unwrap();
        assert!(pass.rubout && pass.diameter_mm == 2.0);
        // half a tool in from the edge, then a step of 2 mm each time: 1, 3 (the middle is 5 deep, so a ring along it is added at 5)
        let boxes: Vec<(f64, f64, f64, f64)> = pass.paths.iter().map(bounds_of).collect();
        assert!(boxes.len() >= 3, "{boxes:?}");
        let near = |a: f64, b: f64| (a - b).abs() < 0.12;
        assert!(
            near(boxes[0].0, 1.0) && near(boxes[0].1, 1.0),
            "{:?}",
            boxes[0]
        );
        assert!(
            near(boxes[0].2, 19.0) && near(boxes[0].3, 9.0),
            "{:?}",
            boxes[0]
        );
        assert!(
            near(boxes[1].0, 3.0) && near(boxes[1].3, 7.0),
            "{:?}",
            boxes[1]
        );
        // each ring is inside the one before
        for w in boxes.windows(2) {
            assert!(w[1].0 > w[0].0 && w[1].1 > w[0].1 && w[1].2 < w[0].2 && w[1].3 < w[0].3);
        }
    }

    #[test]
    fn the_order_turns_the_rings_round_and_a_smaller_overlap_means_more_rings() {
        let area = vec![rect(0.0, 0.0, 30.0, 16.0)];
        let outside_in = fill_rings(&area, &rings(2.0, 0.0, RingOrder::OutsideIn)).unwrap();
        let inside_out = fill_rings(&area, &rings(2.0, 0.0, RingOrder::InsideOut)).unwrap();
        let mut back = inside_out.paths.clone();
        back.reverse();
        assert_eq!(back, outside_in.paths);
        let box_of = |p: &Pass, i: usize| bounds_of(&p.paths[i]);
        assert!(box_of(&outside_in, 0).0 < box_of(&outside_in, 1).0);
        assert!(box_of(&inside_out, 0).0 > box_of(&inside_out, 1).0);
        // 50 % overlap puts the rings 1 mm apart: about twice as many
        let overlapped = fill_rings(&area, &rings(2.0, 0.5, RingOrder::OutsideIn)).unwrap();
        assert!(
            overlapped.paths.len() >= 2 * outside_in.paths.len() - 2,
            "{} and {}",
            overlapped.paths.len(),
            outside_in.paths.len()
        );
        let gap = |p: &Pass| bounds_of(&p.paths[1]).0 - bounds_of(&p.paths[0]).0;
        assert!(
            (gap(&overlapped) - 1.0).abs() < 0.12,
            "{}",
            gap(&overlapped)
        );
        assert!(
            (gap(&outside_in) - 2.0).abs() < 0.12,
            "{}",
            gap(&outside_in)
        );
    }

    #[test]
    fn a_contour_inside_another_is_a_hole_the_rings_go_round() {
        let area = vec![rect(0.0, 0.0, 30.0, 30.0), rect(10.0, 10.0, 20.0, 20.0)];
        let pass = fill_rings(&area, &rings(2.0, 0.0, RingOrder::OutsideIn)).unwrap();
        // no ring runs through the hole: every point is outside the hole less half a tool
        for p in pass.paths.iter().flatten() {
            let inside_hole =
                p.0 > 10.0 + 0.9 && p.0 < 20.0 - 0.9 && p.1 > 10.0 + 0.9 && p.1 < 20.0 - 0.9;
            assert!(!inside_hole, "{p:?}");
        }
        // there are rings round the hole as well as round the edge: some are small closed paths near it
        let round_hole = pass
            .paths
            .iter()
            .filter(|r| {
                let b = bounds_of(r);
                b.0 > 5.0 && b.2 < 25.0
            })
            .count();
        assert!(round_hole >= 1, "{} rings", pass.paths.len());
    }

    #[test]
    fn what_cannot_be_filled_is_refused_with_the_reason() {
        let area = vec![rect(0.0, 0.0, 20.0, 10.0)];
        let ok = rings(2.0, 0.2, RingOrder::OutsideIn);
        for (bad, part) in [
            (Rings { tool_mm: 0.0, ..ok }, "tool of the rings"),
            (
                Rings {
                    tool_mm: f64::NAN,
                    ..ok
                },
                "tool of the rings",
            ),
            (
                Rings {
                    overlap: 0.95,
                    ..ok
                },
                "overlap",
            ),
            (
                Rings {
                    overlap: -0.1,
                    ..ok
                },
                "overlap",
            ),
            (Rings { res_mm: 0.0, ..ok }, "cell"),
        ] {
            let e = fill_rings(&area, &bad).unwrap_err();
            assert!(e.contains(part), "{e}");
        }
        assert!(fill_rings(&[], &ok)
            .unwrap_err()
            .contains("no closed contour"));
        assert!(fill_rings(&[vec![(0.0, 0.0), (1.0, 1.0)]], &ok).is_err());
        assert!(
            fill_rings(&[vec![(0.0, 0.0), (f64::NAN, 1.0), (1.0, 0.0)]], &ok)
                .unwrap_err()
                .contains("not a number")
        );
        // narrower than the tool
        let thin = vec![rect(0.0, 0.0, 20.0, 1.0)];
        assert!(fill_rings(&thin, &ok).unwrap_err().contains("narrower"));
        // more rings than the limit
        let big = vec![rect(0.0, 0.0, 400.0, 400.0)];
        let fine = Rings {
            tool_mm: 0.1,
            overlap: 0.9,
            res_mm: 0.5,
            order: RingOrder::OutsideIn,
        };
        assert!(fill_rings(&big, &fine).is_err());
    }

    #[test]
    fn the_style_and_the_order_are_read_by_their_names() {
        assert_eq!(Style::parse(" Rings "), Ok(Style::Rings));
        assert_eq!(Style::parse("lines"), Ok(Style::Lines));
        assert!(Style::parse("spiral")
            .unwrap_err()
            .contains("lines or rings"));
        assert_eq!(RingOrder::parse("Inside_Out"), Ok(RingOrder::InsideOut));
        assert_eq!(RingOrder::parse("outside-in"), Ok(RingOrder::OutsideIn));
        assert!(RingOrder::parse("up").is_err());
        for s in [Style::Lines, Style::Rings] {
            assert_eq!(Style::parse(s.name()), Ok(s));
        }
        for o in [RingOrder::OutsideIn, RingOrder::InsideOut] {
            assert_eq!(RingOrder::parse(o.name()), Ok(o));
        }
    }
}
