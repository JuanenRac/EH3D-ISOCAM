//! The silkscreen layer as engraving. A milling machine cannot print ink, so the lines of the layer are engraved: the tool follows the centre of every stroke at `silk_depth_mm` and
//! cuts a line as wide as the tool, not as wide as the stroke. Strokes that touch end to end are joined into one run, a region is cut round its edge, and what cannot be engraved
//! (a pad drawn as a flash) is left out and counted. Plain code: nothing here opens a file.

use crate::config::Settings;
use crate::contour::Loop;
use crate::isolate::Pass;
use eh3d_gerberviewer::scene::{Scene, Shape};
use std::collections::BTreeMap;

/// Two ends closer than this, in mm, are the same point.
const JOIN_MM: f64 = 1e-4;

/// The two ends of a stroke.
type Stroke = ((f64, f64), (f64, f64));

/// The engraving of a silkscreen layer and what was said about it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Engraving {
    /// One pass with the closed paths and the open runs, with the tool of `silk_tool_mm`; none when nothing can be engraved.
    pub passes: Vec<Pass>,
    /// The strokes of the layer.
    pub strokes: usize,
    /// The regions of the layer, cut round their edge.
    pub regions: usize,
    /// The flashes (pads and marks drawn with one stamp) that were left out.
    pub flashes_left_out: usize,
    /// Strokes whose width differs from the tool by more than `silk_width_tolerance_mm`.
    pub width_mismatches: usize,
    /// The narrowest and the widest stroke drawn, in mm; 0 when there is none.
    pub drawn_width_mm: (f64, f64),
    /// What the person should look at before the file is used.
    pub warnings: Vec<String>,
}

fn key(p: (f64, f64)) -> (i64, i64) {
    (
        (p.0 / JOIN_MM).round() as i64,
        (p.1 / JOIN_MM).round() as i64,
    )
}

fn same(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= JOIN_MM
}

/// Joins strokes that touch end to end into runs: closed paths (without the first point repeated) and open runs. The order follows the order of the strokes in the layer.
fn chain(segs: &[Stroke]) -> (Vec<Loop>, Vec<Loop>) {
    let mut at: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, s) in segs.iter().enumerate() {
        at.entry(key(s.0)).or_default().push(i);
        at.entry(key(s.1)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let (mut closed, mut open) = (Vec::new(), Vec::new());
    // the next unused stroke that touches the point, as the end it leaves from
    let next = |used: &[bool], p: (f64, f64)| -> Option<usize> {
        // a point near the edge of a cell of the key may sit in the next one
        let (kx, ky) = key(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(v) = at.get(&(kx + dx, ky + dy)) {
                    for &i in v {
                        if !used[i] && (same(segs[i].0, p) || same(segs[i].1, p)) {
                            return Some(i);
                        }
                    }
                }
            }
        }
        None
    };
    for first in 0..segs.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let mut run: Vec<(f64, f64)> = vec![segs[first].0, segs[first].1];
        loop {
            let end = *run.last().unwrap();
            let Some(i) = next(&used, end) else { break };
            used[i] = true;
            run.push(if same(segs[i].0, end) {
                segs[i].1
            } else {
                segs[i].0
            });
        }
        loop {
            let start = run[0];
            let Some(i) = next(&used, start) else { break };
            used[i] = true;
            run.insert(
                0,
                if same(segs[i].0, start) {
                    segs[i].1
                } else {
                    segs[i].0
                },
            );
        }
        if run.len() >= 4 && same(run[0], *run.last().unwrap()) {
            run.pop();
            closed.push(run);
        } else {
            open.push(run);
        }
    }
    (closed, open)
}

/// The engraving of a silkscreen layer for these settings.
pub fn engrave(scene: &Scene, s: &Settings) -> Result<Engraving, String> {
    if !(s.silk_tool_mm.is_finite() && s.silk_tool_mm > 0.0) {
        return Err("silk_tool_mm must be above 0".into());
    }
    let mut e = Engraving::default();
    let mut segs = Vec::new();
    let mut loops: Vec<Loop> = Vec::new();
    let (mut narrow, mut wide) = (f64::INFINITY, 0.0_f64);
    for shape in &scene.shapes {
        match shape {
            Shape::Stroke { x0, y0, x1, y1, w } => {
                e.strokes += 1;
                narrow = narrow.min(*w);
                wide = wide.max(*w);
                if (w - s.silk_tool_mm).abs() > s.silk_width_tolerance_mm + 1e-9 {
                    e.width_mismatches += 1;
                }
                if (x0 - x1).hypot(y0 - y1) > JOIN_MM {
                    segs.push(((*x0, *y0), (*x1, *y1)));
                }
            }
            Shape::Region { pts } => {
                let mut p = pts.clone();
                if p.len() > 1 && same(p[0], p[p.len() - 1]) {
                    p.pop();
                }
                if p.len() >= 3 {
                    e.regions += 1;
                    loops.push(p);
                }
            }
            Shape::Disc { .. } | Shape::Rect { .. } | Shape::Obround { .. } => {
                e.flashes_left_out += 1
            }
        }
    }
    if e.strokes > 0 {
        e.drawn_width_mm = (narrow, wide);
    }
    let (mut closed, open) = chain(&segs);
    closed.append(&mut loops);
    if closed.is_empty() && open.is_empty() {
        return Err("the layer has no stroke or region to engrave".into());
    }
    e.passes.push(Pass {
        offset_mm: 0.0,
        paths: closed,
        open,
        tool: 0,
        diameter_mm: s.silk_tool_mm,
        rubout: false,
    });
    e.warnings.push(
        "the silkscreen is engraved, not printed: every line is as wide as the tool and is cut into the board"
            .into(),
    );
    if e.width_mismatches > 0 {
        e.warnings.push(format!(
            "{} of {} strokes are drawn {} to {} mm wide and the engraved line is {} mm wide (silk_width_tolerance_mm is {})",
            e.width_mismatches,
            e.strokes,
            e.drawn_width_mm.0,
            e.drawn_width_mm.1,
            s.silk_tool_mm,
            s.silk_width_tolerance_mm
        ));
    }
    if e.flashes_left_out > 0 {
        e.warnings.push(format!(
            "{} flashes (pads and marks drawn with one stamp) are not engraved",
            e.flashes_left_out
        ));
    }
    for n in &scene.notes {
        e.warnings.push(format!("not drawn: {n}"));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Shape {
        Shape::Stroke { x0, y0, x1, y1, w }
    }

    fn scene(shapes: Vec<Shape>) -> Scene {
        Scene {
            shapes,
            notes: Vec::new(),
        }
    }

    #[test]
    fn strokes_that_touch_end_to_end_become_one_run() {
        let s = Settings::default();
        let e = engrave(
            &scene(vec![
                stroke(0.0, 0.0, 5.0, 0.0, 0.2),
                stroke(10.0, 10.0, 12.0, 10.0, 0.2),
                stroke(5.0, 0.0, 5.0, 3.0, 0.2),
            ]),
            &s,
        )
        .unwrap();
        let p = &e.passes[0];
        assert_eq!((p.paths.len(), p.open.len()), (0, 2));
        assert_eq!(p.open[0].len(), 3);
        assert_eq!(p.open[1].len(), 2);
        assert_eq!(e.strokes, 3);
    }

    #[test]
    fn a_square_of_strokes_is_a_closed_path() {
        let s = Settings::default();
        let e = engrave(
            &scene(vec![
                stroke(0.0, 0.0, 4.0, 0.0, 0.2),
                stroke(4.0, 0.0, 4.0, 4.0, 0.2),
                stroke(4.0, 4.0, 0.0, 4.0, 0.2),
                stroke(0.0, 4.0, 0.0, 0.0, 0.2),
            ]),
            &s,
        )
        .unwrap();
        assert_eq!(e.passes[0].paths.len(), 1);
        assert_eq!(e.passes[0].paths[0].len(), 4);
        assert!(e.passes[0].open.is_empty());
    }

    #[test]
    fn a_region_is_cut_round_its_edge_and_a_flash_is_left_out() {
        let s = Settings::default();
        let e = engrave(
            &scene(vec![
                Shape::Region {
                    pts: vec![(0.0, 0.0), (3.0, 0.0), (3.0, 2.0), (0.0, 0.0)],
                },
                Shape::Disc {
                    x: 9.0,
                    y: 9.0,
                    r: 0.5,
                },
            ]),
            &s,
        )
        .unwrap();
        assert_eq!((e.regions, e.flashes_left_out), (1, 1));
        assert_eq!(e.passes[0].paths[0].len(), 3);
        assert!(e.warnings.iter().any(|w| w.contains("1 flashes")));
    }

    #[test]
    fn a_stroke_of_another_width_than_the_tool_is_told() {
        let mut s = Settings::default();
        s.set("silk_tool_mm", "0.2").unwrap();
        let e = engrave(
            &scene(vec![
                stroke(0.0, 0.0, 5.0, 0.0, 0.2),
                stroke(0.0, 3.0, 5.0, 3.0, 0.5),
            ]),
            &s,
        )
        .unwrap();
        assert_eq!(e.width_mismatches, 1);
        assert_eq!(e.drawn_width_mm, (0.2, 0.5));
        assert!(e
            .warnings
            .iter()
            .any(|w| w.contains("1 of 2 strokes") && w.contains("0.2 mm wide")));
        // a wider tolerance accepts it
        s.set("silk_width_tolerance_mm", "0.4").unwrap();
        let e = engrave(&scene(vec![stroke(0.0, 3.0, 5.0, 3.0, 0.5)]), &s).unwrap();
        assert_eq!(e.width_mismatches, 0);
    }

    #[test]
    fn a_layer_with_nothing_to_engrave_is_an_error_and_the_tool_comes_from_the_settings() {
        let s = Settings::default();
        assert!(engrave(&scene(Vec::new()), &s).is_err());
        let only_flash = scene(vec![Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 1.0,
        }]);
        assert!(engrave(&only_flash, &s).is_err());
        let mut t = Settings::default();
        t.set("silk_tool_mm", "0.35").unwrap();
        let e = engrave(&scene(vec![stroke(0.0, 0.0, 1.0, 0.0, 0.35)]), &t).unwrap();
        assert_eq!(e.passes[0].diameter_mm, 0.35);
    }
}
