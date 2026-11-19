//! The path of the tool of a G-code program as lines on the plane, for the picture of the window: the rapid moves through the air, the moves down into the material and the cutting
//! moves, each cutting stretch with the depth it is cut at, so that the picture can colour them by type and by depth. The program is walked with the interpreter of the suite, so the
//! picture shows what the machine would do (arcs included) and not what the settings meant.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion};

/// What a stroke of the picture is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A rapid move across the plane.
    Rapid,
    /// A move straight down (or up) at the feed: one point on the plane.
    Plunge,
    /// A cutting move.
    Cut,
}

/// A line on the plane (a single point for a plunge) and the depth it is at below Z 0, as a positive number (0 for a rapid move).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub kind: Kind,
    pub points: Vec<(f64, f64)>,
    pub depth_mm: f64,
}

/// The strokes of a program in the order the tool makes them, and the deepest the tool goes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trace {
    pub strokes: Vec<Stroke>,
    pub deepest_mm: f64,
}

/// The longest step of an arc drawn as a line, in radians (5 degrees).
const ARC_STEP: f64 = 5.0 * std::f64::consts::PI / 180.0;

/// The points of an arc from `from` to `to` about `center` (a start that is the end is a whole circle), the first and the last included.
fn arc_points(from: (f64, f64), to: (f64, f64), center: (f64, f64), cw: bool) -> Vec<(f64, f64)> {
    let (ax, ay) = (from.0 - center.0, from.1 - center.1);
    let (bx, by) = (to.0 - center.0, to.1 - center.1);
    let radius = ax.hypot(ay);
    let mut sweep = (ax * by - ay * bx).atan2(ax * bx + ay * by);
    if cw {
        sweep = -sweep;
    }
    if sweep <= 1e-12 {
        sweep += std::f64::consts::TAU;
    }
    let start = ay.atan2(ax);
    let steps = ((sweep / ARC_STEP).ceil() as usize).max(2);
    let mut points = Vec::with_capacity(steps + 1);
    points.push(from);
    for k in 1..steps {
        let a = if cw {
            start - sweep * k as f64 / steps as f64
        } else {
            start + sweep * k as f64 / steps as f64
        };
        points.push((center.0 + radius * a.cos(), center.1 + radius * a.sin()));
    }
    points.push(to);
    points
}

/// Walks the program and gives its strokes. Cutting moves that follow each other at the same depth are one stroke.
pub fn trace(text: &str) -> Trace {
    let moves = interpret(&parse(text, Dialect::Grbl));
    let mut out = Trace::default();
    for m in &moves.segments {
        let from = (m.from.x, m.from.y);
        let to = (m.to.x, m.to.y);
        let flat = (to.0 - from.0).hypot(to.1 - from.1);
        let depth = (-m.from.z.min(m.to.z)).max(0.0);
        let (kind, points) = match m.motion {
            Motion::Rapid => {
                if flat < 1e-9 {
                    continue;
                }
                (Kind::Rapid, vec![from, to])
            }
            Motion::ArcCw { center } => (Kind::Cut, arc_points(from, to, center, true)),
            Motion::ArcCcw { center } => (Kind::Cut, arc_points(from, to, center, false)),
            Motion::Linear if flat < 1e-9 => (Kind::Plunge, vec![from]),
            Motion::Linear => (Kind::Cut, vec![from, to]),
        };
        if kind != Kind::Rapid {
            out.deepest_mm = out.deepest_mm.max(depth);
        }
        let depth = if kind == Kind::Rapid { 0.0 } else { depth };
        if let Some(last) = out.strokes.last_mut() {
            let joins = kind == Kind::Cut
                && last.kind == Kind::Cut
                && (last.depth_mm - depth).abs() < 1e-9
                && last.points.last().is_some_and(|p| {
                    (p.0 - points[0].0).abs() < 1e-9 && (p.1 - points[0].1).abs() < 1e-9
                });
            if joins {
                last.points.extend(points.into_iter().skip(1));
                continue;
            }
        }
        out.strokes.push(Stroke {
            kind,
            points,
            depth_mm: depth,
        });
    }
    out
}

/// The colour of a stroke (red, green, blue): grey for a rapid move, red for a move down, and for a cutting move a blend from blue (just under the surface) to amber (the deepest cut
/// of the program). Without a depth to compare with, every cut is amber.
pub fn colour(kind: Kind, depth_mm: f64, deepest_mm: f64) -> (u8, u8, u8) {
    match kind {
        Kind::Rapid => (0x8a, 0x8f, 0x9c),
        Kind::Plunge => (0xff, 0x54, 0x70),
        Kind::Cut => {
            let t = if deepest_mm > 0.0 && depth_mm.is_finite() {
                (depth_mm / deepest_mm).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let mix =
                |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
            (mix(0x2e, 0xff), mix(0x9e, 0xb0), mix(0xff, 0x00))
        }
    }
}

impl Trace {
    /// How many strokes of a kind there are.
    pub fn count(&self, kind: Kind) -> usize {
        self.strokes.iter().filter(|s| s.kind == kind).count()
    }

    /// The box of everything on the plane: the smallest and the largest corner; none for an empty trace.
    pub fn bounds(&self) -> Option<((f64, f64), (f64, f64))> {
        let mut pts = self.strokes.iter().flat_map(|s| s.points.iter());
        let first = *pts.next()?;
        Some(pts.fold((first, first), |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_DEPTHS: &str = "G21\nG90\nG0 Z2\nM3 S24000\nG0 X10 Y10\nG1 Z-0.03 F60\nG1 X20 Y10 F300\nG1 X20 Y20\nG1 Z-0.06 F60\nG1 X10 Y20 F300\nG0 Z2\nG0 X0 Y0\nM5\nM2\n";

    #[test]
    fn rapid_moves_plunges_and_cuts_are_told_apart_with_their_depths() {
        let t = trace(TWO_DEPTHS);
        assert_eq!(t.count(Kind::Rapid), 2, "{:?}", t.strokes);
        assert_eq!(t.count(Kind::Plunge), 2);
        // the two cutting moves at 0.03 are one stroke, the one at 0.06 is another
        let cuts: Vec<&Stroke> = t.strokes.iter().filter(|s| s.kind == Kind::Cut).collect();
        assert_eq!(cuts.len(), 2);
        assert_eq!(cuts[0].points, [(10.0, 10.0), (20.0, 10.0), (20.0, 20.0)]);
        assert!((cuts[0].depth_mm - 0.03).abs() < 1e-9);
        assert!((cuts[1].depth_mm - 0.06).abs() < 1e-9);
        assert!((t.deepest_mm - 0.06).abs() < 1e-9);
        // the order is the order of the program
        assert_eq!(t.strokes[0].kind, Kind::Rapid);
        assert_eq!(t.strokes[0].depth_mm, 0.0);
        assert_eq!(t.bounds(), Some(((0.0, 0.0), (20.0, 20.0))));
    }

    #[test]
    fn an_arc_is_drawn_as_points_on_the_circle_and_a_whole_circle_closes() {
        let half = trace("G21\nG90\nG0 X0 Y0\nM3 S1000\nG1 Z-0.1 F60\nG2 X20 Y0 I10 J0 F600\nM2\n");
        let arc = half
            .strokes
            .iter()
            .find(|s| s.kind == Kind::Cut)
            .expect("a cut");
        assert!(arc.points.len() > 20);
        for p in &arc.points {
            assert!(((p.0 - 10.0).hypot(p.1) - 10.0).abs() < 1e-9);
            assert!(p.1 >= -1e-9, "clockwise from the left goes over the top");
        }
        assert_eq!(arc.points.first(), Some(&(0.0, 0.0)));
        let end = arc.points.last().unwrap();
        assert!((end.0 - 20.0).abs() < 1e-9 && end.1.abs() < 1e-9);
        // counterclockwise goes under
        let under =
            trace("G21\nG90\nG0 X0 Y0\nM3 S1000\nG1 Z-0.1 F60\nG3 X20 Y0 I10 J0 F600\nM2\n");
        let arc = under.strokes.iter().find(|s| s.kind == Kind::Cut).unwrap();
        assert!(arc.points.iter().all(|p| p.1 <= 1e-9));
        // a start that is the end is a whole circle
        let full =
            trace("G21\nG90\nG0 X10 Y0\nM3 S1000\nG1 Z-0.1 F60\nG2 X10 Y0 I-10 J0 F600\nM2\n");
        let ring = full.strokes.iter().find(|s| s.kind == Kind::Cut).unwrap();
        assert!(ring.points.len() > 60, "{}", ring.points.len());
        let (lo, hi) = full.bounds().unwrap();
        assert!(
            lo.0 < -9.9 && hi.0 > 9.9 && lo.1 < -9.9 && hi.1 > 9.9,
            "{lo:?} {hi:?}"
        );
    }

    #[test]
    fn an_empty_program_and_moves_without_a_distance_have_nothing_to_draw() {
        assert_eq!(trace(""), Trace::default());
        assert_eq!(Trace::default().bounds(), None);
        let t = trace("G21\nG90\nG0 Z5\nG0 Z8\nM2\n");
        assert!(t.strokes.is_empty(), "{:?}", t.strokes);
    }

    #[test]
    fn the_colours_go_from_blue_to_amber_with_the_depth_and_the_other_kinds_have_their_own() {
        assert_eq!(colour(Kind::Cut, 0.0, 0.06), (0x2e, 0x9e, 0xff));
        assert_eq!(colour(Kind::Cut, 0.06, 0.06), (0xff, 0xb0, 0x00));
        let mid = colour(Kind::Cut, 0.03, 0.06);
        assert!(
            mid.0 > 0x2e && mid.0 < 0xff && mid.2 > 0 && mid.2 < 0xff,
            "{mid:?}"
        );
        // a depth beyond the deepest, or a program with no depth, is the deepest colour
        assert_eq!(colour(Kind::Cut, 1.0, 0.06), (0xff, 0xb0, 0x00));
        assert_eq!(colour(Kind::Cut, 0.0, 0.0), (0xff, 0xb0, 0x00));
        assert_eq!(colour(Kind::Cut, f64::NAN, 0.06), (0xff, 0xb0, 0x00));
        let kinds = [
            colour(Kind::Rapid, 0.0, 0.06),
            colour(Kind::Plunge, 0.06, 0.06),
            colour(Kind::Cut, 0.06, 0.06),
        ];
        assert!(kinds[0] != kinds[1] && kinds[1] != kinds[2] && kinds[0] != kinds[2]);
    }
}
