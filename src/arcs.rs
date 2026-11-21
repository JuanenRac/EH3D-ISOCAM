//! Arcs fitted to the straight pieces of a path. The paths come from the exact distance on a grid and are many short moves; where a run of them lies on a circle, one arc
//! (`G2` or `G3`) replaces the run, so the program is shorter and the tool turns smoothly. A run is replaced only when every point is within half the tolerance of the circle and
//! the farthest point from the circle plus the sagitta of the longest piece (how far the arc bulges from the straight piece it replaces) is within the tolerance, so the arc never
//! leaves the polygon by more than the tolerance. Plain code: nothing here writes a file.

/// A point of a path, in mm.
pub type Pt = (f64, f64);

/// The smallest tolerance that can be asked for, in mm.
pub const MIN_TOLERANCE_MM: f64 = 0.001;
/// The largest tolerance that can be asked for, in mm.
pub const MAX_TOLERANCE_MM: f64 = 0.5;
/// An arc replaces at least this many straight pieces; fewer would not make the program shorter by enough to be worth the arc.
pub const MIN_PIECES: usize = 3;
/// A circle wider than this is a straight line for all that a machine can tell, in mm.
const MAX_RADIUS_MM: f64 = 10_000.0;
/// Two consecutive points of an arc are at most this angle apart round its centre (a quarter turn).
const MAX_STEP_RAD: f64 = std::f64::consts::FRAC_PI_2;
/// An arc turns at most this much (three quarters of a turn), so that its start and its end are never taken for a whole circle.
const MAX_SWEEP_RAD: f64 = 1.5 * std::f64::consts::PI;

/// One move of a fitted path, from the end of the one before.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    /// A straight move to the point.
    Line(Pt),
    /// An arc to the point, round `centre`, counter-clockwise when `ccw` (`G3`) and clockwise otherwise (`G2`).
    Arc { to: Pt, centre: Pt, ccw: bool },
}

impl Seg {
    /// The point the move ends at.
    pub fn end(&self) -> Pt {
        match self {
            Seg::Line(p) => *p,
            Seg::Arc { to, .. } => *to,
        }
    }
}

/// Whether a tolerance can be asked for: 0 (no arcs) or from [`MIN_TOLERANCE_MM`] to [`MAX_TOLERANCE_MM`].
pub fn tolerance_ok(tol_mm: f64) -> bool {
    tol_mm == 0.0 || (tol_mm.is_finite() && (MIN_TOLERANCE_MM..=MAX_TOLERANCE_MM).contains(&tol_mm))
}

/// The circle through three points (centre and radius), or none when they are in a line or too nearly so.
fn circle(a: Pt, b: Pt, c: Pt) -> Option<(Pt, f64)> {
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    if !d.is_finite() || d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (
        a.0 * a.0 + a.1 * a.1,
        b.0 * b.0 + b.1 * b.1,
        c.0 * c.0 + c.1 * c.1,
    );
    let ux = (a2 * (b.1 - c.1) + b2 * (c.1 - a.1) + c2 * (a.1 - b.1)) / d;
    let uy = (a2 * (c.0 - b.0) + b2 * (a.0 - c.0) + c2 * (b.0 - a.0)) / d;
    let r = (a.0 - ux).hypot(a.1 - uy);
    (r.is_finite() && r <= MAX_RADIUS_MM).then_some(((ux, uy), r))
}

/// The angle from `a` to `b` round `centre`, in (-pi, pi]; positive counter-clockwise.
fn turn(centre: Pt, a: Pt, b: Pt) -> f64 {
    let (ax, ay) = (a.0 - centre.0, a.1 - centre.1);
    let (bx, by) = (b.0 - centre.0, b.1 - centre.1);
    (ax * by - ay * bx).atan2(ax * bx + ay * by)
}

/// The arc that replaces a run of points (the first, the last and the ones between), or none when they do not lie on one circle within the tolerance.
fn try_arc(run: &[Pt], tol: f64) -> Option<Seg> {
    let (a, c) = (*run.first()?, *run.last()?);
    let b = run[run.len() / 2];
    let (centre, r) = circle(a, b, c)?;
    let half = tol / 2.0;
    let mut sweep = 0.0;
    let mut sign = 0.0_f64;
    let (mut off, mut bulge) = (0.0_f64, 0.0_f64);
    for w in run.windows(2) {
        // every point is on the circle, and the piece between two points is no farther from the arc than its sagitta
        let miss = ((w[0].0 - centre.0).hypot(w[0].1 - centre.1) - r).abs();
        if miss > half {
            return None;
        }
        off = off.max(miss);
        let step = turn(centre, w[0], w[1]);
        if step == 0.0 || step.abs() > MAX_STEP_RAD || (sign != 0.0 && step.signum() != sign) {
            return None;
        }
        sign = step.signum();
        sweep += step.abs();
        let chord = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        bulge = bulge.max(r - (r * r - chord * chord / 4.0).max(0.0).sqrt());
    }
    off = off.max(((c.0 - centre.0).hypot(c.1 - centre.1) - r).abs());
    if off > half || off + bulge > tol || sweep > MAX_SWEEP_RAD {
        return None;
    }
    Some(Seg::Arc {
        to: c,
        centre,
        ccw: sign > 0.0,
    })
}

/// The moves of an open path that starts at `points[0]`, with arcs where a run of at least [`MIN_PIECES`] straight pieces lies on a circle within `tol_mm`. A tolerance of 0, or a
/// tolerance that is not valid, keeps every piece straight. The moves end where the points end, in the same order.
pub fn fit(points: &[Pt], tol_mm: f64) -> Vec<Seg> {
    let n = points.len();
    let straight = |from: usize| points[from..].iter().skip(1).map(|p| Seg::Line(*p));
    if n < 2
        || !(tol_mm > 0.0 && tolerance_ok(tol_mm))
        || points.iter().any(|p| !(p.0.is_finite() && p.1.is_finite()))
    {
        return straight(0).collect();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < n {
        let mut best: Option<(usize, Seg)> = None;
        let mut j = i + MIN_PIECES;
        while j < n {
            match try_arc(&points[i..=j], tol_mm) {
                Some(seg) => best = Some((j, seg)),
                None => break,
            }
            j += 1;
        }
        match best {
            Some((end, seg)) => {
                out.push(seg);
                i = end;
            }
            None => {
                out.push(Seg::Line(points[i + 1]));
                i += 1;
            }
        }
    }
    out
}

/// The points of the moves with every arc drawn as short straight pieces no farther than 0.0005 mm from it, from `start`.
pub fn sample(start: Pt, segs: &[Seg]) -> Vec<Pt> {
    let mut pts = vec![start];
    let mut at = start;
    for s in segs {
        match *s {
            Seg::Line(p) => pts.push(p),
            Seg::Arc { to, centre, ccw } => {
                let r = (at.0 - centre.0).hypot(at.1 - centre.1);
                let mut angle = turn(centre, at, to);
                if ccw && angle <= 0.0 {
                    angle += std::f64::consts::TAU;
                }
                if !ccw && angle >= 0.0 {
                    angle -= std::f64::consts::TAU;
                }
                // pieces whose sagitta stays under the limit
                let step = 2.0 * (1.0 - 0.0005 / r.max(0.001)).clamp(-1.0, 1.0).acos();
                let n = ((angle.abs() / step.max(1e-4)).ceil() as usize).clamp(1, 100_000);
                let a0 = (at.1 - centre.1).atan2(at.0 - centre.0);
                for k in 1..n {
                    let a = a0 + angle * k as f64 / n as f64;
                    pts.push((centre.0 + r * a.cos(), centre.1 + r * a.sin()));
                }
                pts.push(to);
            }
        }
        at = s.end();
    }
    pts
}

/// The distance from a point to the nearest point of a polyline.
fn distance_to(p: Pt, line: &[Pt]) -> f64 {
    if line.len() == 1 {
        return (p.0 - line[0].0).hypot(p.1 - line[0].1);
    }
    line.windows(2)
        .map(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (p.0 - (w[0].0 + t * dx)).hypot(p.1 - (w[0].1 + t * dy))
        })
        .fold(f64::INFINITY, f64::min)
}

/// How far the fitted path is from the polygon it came from: the largest distance of a point of one to the other, both ways round, in mm. It is what the tolerance bounds.
pub fn deviation_mm(points: &[Pt], segs: &[Seg]) -> f64 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    let fitted = sample(*first, segs);
    let a = points
        .iter()
        .map(|p| distance_to(*p, &fitted))
        .fold(0.0, f64::max);
    let b = fitted
        .iter()
        .map(|p| distance_to(*p, points))
        .fold(0.0, f64::max);
    a.max(b)
}

/// How many of the moves are arcs.
pub fn arcs_in(segs: &[Seg]) -> usize {
    segs.iter().filter(|s| matches!(s, Seg::Arc { .. })).count()
}

/// The polygon against the arcs, as numbers a person can read: the moves each makes, how many arcs there are and how far the one is from the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comparison {
    /// The straight moves of the polygon.
    pub polygon_moves: usize,
    /// The moves once arcs are fitted (arcs and the straight moves left).
    pub fitted_moves: usize,
    pub arcs: usize,
    /// The largest distance between the two paths, in mm.
    pub deviation_mm: f64,
}

/// Compares a path with the arcs fitted to it.
pub fn compare(points: &[Pt], tol_mm: f64) -> Comparison {
    let segs = fit(points, tol_mm);
    Comparison {
        polygon_moves: points.len().saturating_sub(1),
        fitted_moves: segs.len(),
        arcs: arcs_in(&segs),
        deviation_mm: deviation_mm(points, &segs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(r: f64, n: usize) -> Vec<Pt> {
        (0..=n)
            .map(|k| {
                let a = std::f64::consts::TAU * k as f64 / n as f64;
                (r * a.cos(), r * a.sin())
            })
            .collect()
    }

    #[test]
    fn a_circle_drawn_in_many_pieces_becomes_a_few_arcs() {
        let pts = ring(5.0, 64);
        let c = compare(&pts, 0.01);
        assert_eq!(c.polygon_moves, 64);
        assert!(c.arcs >= 1 && c.fitted_moves <= 8, "{c:?}");
        assert!(c.deviation_mm <= 0.01 + 1e-9, "{c:?}");
    }

    #[test]
    fn a_straight_line_and_a_zero_tolerance_stay_straight() {
        let line: Vec<Pt> = (0..10).map(|k| (k as f64, 0.5 * k as f64)).collect();
        assert_eq!(arcs_in(&fit(&line, 0.01)), 0);
        assert_eq!(fit(&line, 0.01).len(), 9);
        let pts = ring(5.0, 64);
        assert_eq!(arcs_in(&fit(&pts, 0.0)), 0);
        assert_eq!(fit(&pts, 0.0).len(), 64);
        assert_eq!(fit(&pts, f64::NAN).len(), 64);
        assert_eq!(fit(&pts, 5.0).len(), 64);
    }

    #[test]
    fn the_direction_of_the_turn_is_kept() {
        let ccw = fit(&ring(5.0, 64), 0.01);
        assert!(ccw
            .iter()
            .all(|s| matches!(s, Seg::Arc { ccw: true, .. } | Seg::Line(_))));
        let mut cw_pts = ring(5.0, 64);
        cw_pts.reverse();
        let cw = fit(&cw_pts, 0.01);
        assert!(arcs_in(&cw) >= 1);
        assert!(cw
            .iter()
            .all(|s| matches!(s, Seg::Arc { ccw: false, .. } | Seg::Line(_))));
    }

    #[test]
    fn the_path_ends_where_the_points_end() {
        let pts = ring(3.0, 40);
        let segs = fit(&pts, 0.02);
        assert_eq!(segs.last().unwrap().end(), *pts.last().unwrap());
    }

    #[test]
    fn an_s_bend_gets_one_arc_each_way_and_stays_within_the_tolerance() {
        let mut pts: Vec<Pt> = (0..=16)
            .map(|k| {
                let a = std::f64::consts::FRAC_PI_2 * k as f64 / 16.0;
                (4.0 * a.cos(), 4.0 * a.sin())
            })
            .collect();
        pts.extend((1..=16).map(|k| {
            let t = std::f64::consts::FRAC_PI_2 * k as f64 / 16.0;
            (-4.0 * t.sin(), 8.0 - 4.0 * t.cos())
        }));
        let segs = fit(&pts, 0.01);
        assert!(segs.iter().any(|s| matches!(s, Seg::Arc { ccw: true, .. })));
        assert!(segs
            .iter()
            .any(|s| matches!(s, Seg::Arc { ccw: false, .. })));
        assert!(deviation_mm(&pts, &segs) <= 0.01 + 1e-9);
    }

    #[test]
    fn a_tolerance_outside_its_range_is_refused() {
        assert!(tolerance_ok(0.0) && tolerance_ok(0.001) && tolerance_ok(0.5));
        assert!(!tolerance_ok(0.0005) && !tolerance_ok(0.6) && !tolerance_ok(-0.01));
        assert!(!tolerance_ok(f64::NAN) && !tolerance_ok(f64::INFINITY));
    }

    #[test]
    fn a_point_that_is_not_a_number_keeps_the_path_straight() {
        let mut pts = ring(5.0, 64);
        pts[10].0 = f64::NAN;
        assert_eq!(arcs_in(&fit(&pts, 0.01)), 0);
    }
}
