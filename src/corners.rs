//! The shape of the outline before the tool goes round it: corners rounded with an arc of a radius, and small cutouts (the holes of the drawing of the board) left out. Both are
//! off by default; they work on the closed paths of the outline, in millimetres, and touch nothing else.

use crate::contour::Loop;

/// What `round_outline` did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rounded {
    /// The closed paths with their corners rounded.
    pub paths: Vec<Loop>,
    /// The corners that were rounded with the radius asked for or with a smaller one.
    pub rounded: usize,
    /// Of those, the corners that got a smaller radius because the sides next to them were too short for the one asked for.
    pub limited: usize,
}

/// The least a path may turn at a corner (radians) for the corner to be rounded: a nearly straight join is left as it is, and so is a path that turns back on itself.
const MIN_TURN: f64 = 1e-3;

/// Rounds every corner of a closed path with an arc of `radius_mm`, the arc drawn in straight pieces that are never further than `tol_mm` from it. A corner whose sides are too
/// short for the radius (an arc takes at most half of each side) gets the biggest radius that fits, and is counted in `limited`. Returns the path and how many corners were rounded
/// and limited. A radius of 0 or less, or a path of fewer than three points, comes back as it was.
pub fn round_corners(path: &[(f64, f64)], radius_mm: f64, tol_mm: f64) -> (Loop, usize, usize) {
    let n = path.len();
    if n < 3 || !(radius_mm.is_finite() && radius_mm > 0.0) {
        return (path.to_vec(), 0, 0);
    }
    let tol = if tol_mm.is_finite() && tol_mm > 0.0 {
        tol_mm
    } else {
        0.01
    };
    let mut out: Loop = Vec::with_capacity(n * 2);
    let (mut rounded, mut limited) = (0, 0);
    for i in 0..n {
        let (a, b, c) = (path[(i + n - 1) % n], path[i], path[(i + 1) % n]);
        let (dx1, dy1) = (b.0 - a.0, b.1 - a.1);
        let (dx2, dy2) = (c.0 - b.0, c.1 - b.1);
        let (l1, l2) = (dx1.hypot(dy1), dx2.hypot(dy2));
        if l1 < 1e-9 || l2 < 1e-9 {
            out.push(b);
            continue;
        }
        let (ux, uy) = (dx1 / l1, dy1 / l1);
        let (vx, vy) = (dx2 / l2, dy2 / l2);
        let cross = ux * vy - uy * vx;
        let dot = ux * vx + uy * vy;
        let turn = cross.abs().atan2(dot);
        if !(MIN_TURN..=std::f64::consts::PI - MIN_TURN).contains(&turn) {
            out.push(b);
            continue;
        }
        let tan_half = (turn / 2.0).tan();
        let room = 0.5 * l1.min(l2);
        let mut r = radius_mm;
        if r * tan_half > room {
            r = room / tan_half;
            limited += 1;
        }
        let t = r * tan_half;
        if r < 1e-6 || t < 1e-9 {
            out.push(b);
            continue;
        }
        let side = cross.signum();
        // the arc starts a distance t before the corner and ends t after it; its centre is r to the inside of the turn
        let start = (b.0 - ux * t, b.1 - uy * t);
        let centre = (start.0 - uy * side * r, start.1 + ux * side * r);
        let a0 = (start.1 - centre.1).atan2(start.0 - centre.0);
        let step = 2.0 * (1.0 - (tol / r).min(0.5)).acos();
        let pieces = ((turn / step).ceil() as usize).max(1);
        for j in 0..=pieces {
            let ang = a0 + side * turn * j as f64 / pieces as f64;
            out.push((centre.0 + r * ang.cos(), centre.1 + r * ang.sin()));
        }
        rounded += 1;
    }
    (out, rounded, limited)
}

/// Rounds the corners of every closed path of an outline (see `round_corners`). A radius of 0 gives the paths back unchanged.
pub fn round_outline(closed: &[Loop], radius_mm: f64, tol_mm: f64) -> Rounded {
    let mut result = Rounded::default();
    for path in closed {
        let (p, r, l) = round_corners(path, radius_mm, tol_mm);
        result.paths.push(p);
        result.rounded += r;
        result.limited += l;
    }
    result
}

/// Whether the point is inside the closed path (the crossing-number test; a point on the edge may fall either way).
fn inside(path: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = path.len();
    let mut yes = false;
    for (i, &a) in path.iter().enumerate() {
        let b = path[(i + n - 1) % n];
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            yes = !yes;
        }
    }
    yes
}

/// The size of the box round a path: its longer side.
fn longer_side(path: &[(f64, f64)]) -> f64 {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in path {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x1 - x0).max(y1 - y0)
}

/// Leaves out the cutouts of the board (a closed path inside an odd number of others) whose box has no side longer than `min_mm`, such as the holes a drawing of the board has for
/// screws or marks that the tool of the cut cannot or should not mill. The outer paths and the bigger cutouts stay, in their order. Returns the paths kept and how many were left
/// out. A `min_mm` of 0 or less keeps everything.
pub fn drop_small_cutouts(closed: &[Loop], min_mm: f64) -> (Vec<Loop>, usize) {
    if !(min_mm.is_finite() && min_mm > 0.0) {
        return (closed.to_vec(), 0);
    }
    let mut kept = Vec::with_capacity(closed.len());
    let mut dropped = 0;
    for (i, path) in closed.iter().enumerate() {
        let depth = match path.first() {
            Some(&p) => closed
                .iter()
                .enumerate()
                .filter(|(j, other)| *j != i && inside(other, p))
                .count(),
            None => 0,
        };
        if depth % 2 == 1 && longer_side(path) < min_mm {
            dropped += 1;
        } else {
            kept.push(path.clone());
        }
    }
    (kept, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, side: f64) -> Loop {
        vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
    }

    fn area(p: &[(f64, f64)]) -> f64 {
        let n = p.len();
        (0..n)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % n]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn a_radius_of_zero_leaves_the_path_as_it_was() {
        let sq = square(0.0, 0.0, 10.0);
        assert_eq!(round_corners(&sq, 0.0, 0.01), (sq.clone(), 0, 0));
        assert_eq!(round_corners(&sq, -1.0, 0.01).0, sq);
        assert_eq!(round_corners(&sq, f64::NAN, 0.01).0, sq);
        let two = vec![(0.0, 0.0), (1.0, 1.0)];
        assert_eq!(round_corners(&two, 2.0, 0.01).0, two);
    }

    #[test]
    fn a_square_gets_four_arcs_that_start_and_end_on_its_sides() {
        let sq = square(0.0, 0.0, 10.0);
        let (p, rounded, limited) = round_corners(&sq, 2.0, 0.005);
        assert_eq!((rounded, limited), (4, 0));
        assert!(p.len() > 4 * 6, "{}", p.len());
        // every point is inside the square and at least one point of each arc is at the radius from its centre
        for &(x, y) in &p {
            assert!((-1e-9..=10.0 + 1e-9).contains(&x) && (-1e-9..=10.0 + 1e-9).contains(&y));
        }
        // the first arc, at the corner (10, 0), has the centre (8, 2): its points are 2 mm from it
        let near_corner: Vec<_> = p
            .iter()
            .filter(|q| q.0 > 8.0 + 1e-9 && q.1 < 2.0 - 1e-9)
            .collect();
        assert!(near_corner.len() >= 5);
        for q in near_corner {
            let d = (q.0 - 8.0).hypot(q.1 - 2.0);
            assert!((d - 2.0).abs() < 1e-9, "{d}");
        }
        // a quarter circle is cut off each corner: the area is smaller by (4 - pi) r^2
        let lost = 100.0 - area(&p).abs();
        let exact = (4.0 - std::f64::consts::PI) * 4.0;
        // the straight pieces lie inside the arc, so a little more is lost than with the true arc
        assert!(
            lost > exact * 0.999 && lost < exact * 1.03,
            "{lost} against {exact}"
        );
    }

    #[test]
    fn the_turn_of_the_path_is_kept_whichever_way_it_goes_round() {
        let sq = square(0.0, 0.0, 10.0);
        let mut back = sq.clone();
        back.reverse();
        let (a, ra, _) = round_corners(&sq, 1.5, 0.01);
        let (b, rb, _) = round_corners(&back, 1.5, 0.01);
        assert_eq!((ra, rb), (4, 4));
        assert!(area(&a) > 0.0 && area(&b) < 0.0);
        assert!((area(&a).abs() - area(&b).abs()).abs() < 1e-9);
    }

    #[test]
    fn a_radius_too_big_for_the_sides_is_cut_down_and_counted() {
        // a 4 x 10 strip: an arc may take at most half of the short side (2 mm), so a radius of 5 is cut down to 2
        let strip = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)];
        let (p, rounded, limited) = round_corners(&strip, 5.0, 0.01);
        assert_eq!((rounded, limited), (4, 4));
        // the arcs meet at the middle of the short sides: the points (10, 2) and (0, 2) are on the path
        assert!(p
            .iter()
            .any(|q| (q.0 - 10.0).abs() < 1e-9 && (q.1 - 2.0).abs() < 1e-9));
        assert!(p
            .iter()
            .any(|q| q.0.abs() < 1e-9 && (q.1 - 2.0).abs() < 1e-9));
        // a radius that fits is not counted as limited
        assert_eq!(round_corners(&strip, 2.0, 0.01).2, 0);
    }

    #[test]
    fn a_straight_join_and_a_repeated_point_are_left_alone() {
        let line = vec![
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 10.0),
        ];
        let (p, rounded, _) = round_corners(&line, 1.0, 0.01);
        // the points on the straight side and the repeated one stay; the corners at (10, 0) is a turn, (10, 10) and (0, 10) are turns
        assert!(p.contains(&(5.0, 0.0)));
        assert!(p.iter().filter(|q| **q == (0.0, 10.0)).count() >= 1);
        assert!(rounded >= 2, "{rounded}");
    }

    #[test]
    fn a_corner_with_a_turn_of_a_few_degrees_is_rounded_with_a_long_gentle_arc() {
        // 170 degrees inside: the turn is 10 degrees and the arc takes r * tan(5 degrees)
        let a = 10.0_f64.to_radians();
        let tri = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0 + 100.0 * a.cos(), 100.0 * a.sin()),
            (50.0, 60.0),
        ];
        let (p, rounded, limited) = round_corners(&tri, 20.0, 0.01);
        assert!(rounded >= 3 && limited <= rounded);
        assert!(p.len() > tri.len());
    }

    #[test]
    fn rounding_an_outline_counts_every_corner_of_every_path() {
        let outline = vec![square(0.0, 0.0, 30.0), square(10.0, 10.0, 6.0)];
        let r = round_outline(&outline, 1.0, 0.01);
        assert_eq!((r.paths.len(), r.rounded, r.limited), (2, 8, 0));
        let same = round_outline(&outline, 0.0, 0.01);
        assert_eq!(same.paths, outline);
        assert_eq!(same.rounded, 0);
        // the small square has sides of 6: a radius of 4 is limited to 3 there and fits the big one
        let big = round_outline(&outline, 4.0, 0.01);
        assert_eq!((big.rounded, big.limited), (8, 4));
    }

    #[test]
    fn small_cutouts_go_and_the_outer_path_and_big_cutouts_stay() {
        let outer = square(0.0, 0.0, 100.0);
        let screw = square(10.0, 10.0, 3.0);
        let slot = square(40.0, 40.0, 20.0);
        let outline = vec![outer.clone(), screw, slot.clone()];
        let (kept, dropped) = drop_small_cutouts(&outline, 5.0);
        assert_eq!(dropped, 1);
        assert_eq!(kept, vec![outer.clone(), slot.clone()]);
        // the limit is on the longer side: 3 mm is not below 3
        assert_eq!(drop_small_cutouts(&outline, 3.0).1, 0);
        assert_eq!(drop_small_cutouts(&outline, 3.001).1, 1);
        // 0 keeps everything
        assert_eq!(drop_small_cutouts(&outline, 0.0), (outline.clone(), 0));
    }

    #[test]
    fn a_small_outer_path_is_a_board_and_an_island_inside_a_cutout_is_one_too() {
        // a board of 4 x 4 on its own stays however big the limit; a path inside a cutout (depth 2) is board again and stays
        let tiny = vec![square(0.0, 0.0, 4.0)];
        assert_eq!(drop_small_cutouts(&tiny, 50.0), (tiny.clone(), 0));
        let nested = vec![
            square(0.0, 0.0, 100.0),
            square(20.0, 20.0, 60.0),
            square(40.0, 40.0, 4.0),
        ];
        let (kept, dropped) = drop_small_cutouts(&nested, 10.0);
        assert_eq!((kept.len(), dropped), (3, 0));
        assert_eq!(drop_small_cutouts(&[], 5.0), (Vec::new(), 0));
    }
}
