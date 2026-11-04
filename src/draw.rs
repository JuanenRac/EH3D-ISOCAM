//! What the window draws for the shapes of a layer: the shapes themselves and, for each filled region, the triangles that cover it (the screen fills triangles, and a region of copper
//! is often not convex). Nothing here needs a screen.

use eh3d_gerberviewer::scene::Shape;

/// Three corners, as places in the list of points of a region.
pub type Tri = [u32; 3];

/// The most corners of a region that are split into triangles; a larger one is drawn as an outline only (the split costs more than the square of the corners).
pub const MAX_CORNERS: usize = 400;

/// The shapes of a layer with the triangles of each region (empty for the other shapes).
#[derive(Debug, Clone, Default)]
pub struct Drawing {
    pub shapes: Vec<Shape>,
    pub tris: Vec<Vec<Tri>>,
}

impl Drawing {
    pub fn new(shapes: &[Shape]) -> Drawing {
        let tris = shapes
            .iter()
            .map(|s| match s {
                Shape::Region { pts } => triangulate(pts),
                _ => Vec::new(),
            })
            .collect();
        Drawing {
            shapes: shapes.to_vec(),
            tris,
        }
    }

    /// The box round every shape as its two corners, or `None` when there is nothing.
    pub fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        let mut lo = (f64::INFINITY, f64::INFINITY);
        let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut put = |x: f64, y: f64, pad: f64| {
            lo = (lo.0.min(x - pad), lo.1.min(y - pad));
            hi = (hi.0.max(x + pad), hi.1.max(y + pad));
        };
        for s in &self.shapes {
            match s {
                Shape::Disc { x, y, r } => put(*x, *y, *r),
                Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
                    put(*x, *y, w.max(*h) / 2.0)
                }
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    put(*x0, *y0, w / 2.0);
                    put(*x1, *y1, w / 2.0);
                }
                Shape::Region { pts } => pts.iter().for_each(|p| put(p.0, p.1, 0.0)),
            }
        }
        (lo.0.is_finite() && hi.0.is_finite() && lo.1.is_finite() && hi.1.is_finite())
            .then_some((lo, hi))
    }
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn signed_area(pts: &[(f64, f64)], order: &[usize]) -> f64 {
    let mut sum = 0.0;
    for k in 0..order.len() {
        let p = pts[order[k]];
        let q = pts[order[(k + 1) % order.len()]];
        sum += p.0 * q.1 - q.0 * p.1;
    }
    sum / 2.0
}

fn inside(p: (f64, f64), a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
    cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
}

/// The triangles that cover a simple polygon, cutting off one ear at a time. A polygon with fewer than three corners, with no area or with more than `MAX_CORNERS` gives none.
/// A last corner equal to the first (a closed ring) is not counted twice.
pub fn triangulate(pts: &[(f64, f64)]) -> Vec<Tri> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    if pts.len() > 3 && pts.first() == pts.last() {
        order.pop();
    }
    if order.len() < 3 || order.len() > MAX_CORNERS {
        return Vec::new();
    }
    if pts.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
        return Vec::new();
    }
    let area = signed_area(pts, &order);
    if area.abs() < 1e-12 {
        return Vec::new();
    }
    if area < 0.0 {
        order.reverse();
    }
    let mut out = Vec::new();
    while order.len() > 3 {
        let m = order.len();
        let mut cut = None;
        for i in 0..m {
            let (a, b, c) = (order[(i + m - 1) % m], order[i], order[(i + 1) % m]);
            if cross(pts[a], pts[b], pts[c]) <= 1e-12 {
                continue;
            }
            let blocked = order.iter().any(|&p| {
                p != a
                    && p != b
                    && p != c
                    && pts[p] != pts[a]
                    && pts[p] != pts[b]
                    && pts[p] != pts[c]
                    && inside(pts[p], pts[a], pts[b], pts[c])
            });
            if !blocked {
                cut = Some((i, [a as u32, b as u32, c as u32]));
                break;
            }
        }
        match cut {
            Some((i, tri)) => {
                out.push(tri);
                order.remove(i);
            }
            // a polygon that crosses itself has no ear: the corner that blocks is dropped so that the rest is still filled
            None => {
                order.remove(0);
            }
        }
    }
    if order.len() == 3 && cross(pts[order[0]], pts[order[1]], pts[order[2]]) > 1e-12 {
        out.push([order[0] as u32, order[1] as u32, order[2] as u32]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(pts: &[(f64, f64)], tris: &[Tri]) -> f64 {
        tris.iter()
            .map(|t| cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]) / 2.0)
            .sum()
    }

    #[test]
    fn a_square_is_two_triangles_with_its_area() {
        let sq = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        let t = triangulate(&sq);
        assert_eq!(t.len(), 2);
        assert!((area_of(&sq, &t) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn a_square_written_clockwise_gives_the_same_area() {
        let sq = [(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0)];
        let t = triangulate(&sq);
        assert_eq!(t.len(), 2);
        assert!((area_of(&sq, &t).abs() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn a_closed_ring_does_not_count_the_first_corner_twice() {
        let sq = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)];
        assert_eq!(triangulate(&sq).len(), 2);
    }

    #[test]
    fn a_concave_l_is_covered_exactly() {
        // an L: 3 x 3 with the corner of 2 x 2 taken out, area 5
        let l = [
            (0.0, 0.0),
            (3.0, 0.0),
            (3.0, 1.0),
            (1.0, 1.0),
            (1.0, 3.0),
            (0.0, 3.0),
        ];
        let t = triangulate(&l);
        assert_eq!(t.len(), 4);
        assert!((area_of(&l, &t) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn what_cannot_be_filled_gives_no_triangles() {
        assert!(triangulate(&[]).is_empty());
        assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0)]).is_empty());
        assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]).is_empty());
        assert!(triangulate(&[(0.0, 0.0), (f64::NAN, 1.0), (2.0, 0.0)]).is_empty());
        let big: Vec<(f64, f64)> = (0..=MAX_CORNERS)
            .map(|k| {
                let a = k as f64 / (MAX_CORNERS + 1) as f64 * std::f64::consts::TAU;
                (a.cos(), a.sin())
            })
            .collect();
        assert!(triangulate(&big).is_empty());
    }

    #[test]
    fn a_circle_of_many_corners_is_covered() {
        let n = 60;
        let ring: Vec<(f64, f64)> = (0..n)
            .map(|k| {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                (a.cos(), a.sin())
            })
            .collect();
        let t = triangulate(&ring);
        assert_eq!(t.len(), n - 2);
        let polygon = n as f64 / 2.0 * (std::f64::consts::TAU / n as f64).sin();
        assert!((area_of(&ring, &t) - polygon).abs() < 1e-9);
    }

    #[test]
    fn the_drawing_keeps_a_list_of_triangles_for_each_shape() {
        let d = Drawing::new(&[
            Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 1.0,
            },
            Shape::Region {
                pts: vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
            },
        ]);
        assert_eq!(d.shapes.len(), 2);
        assert!(d.tris[0].is_empty());
        assert_eq!(d.tris[1].len(), 1);
    }

    #[test]
    fn the_extent_covers_the_width_of_the_tracks() {
        let d = Drawing::new(&[Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 0.0,
            w: 2.0,
        }]);
        let (lo, hi) = d.extent().unwrap();
        assert_eq!(lo, (-1.0, -1.0));
        assert_eq!(hi, (11.0, 1.0));
        assert!(Drawing::default().extent().is_none());
    }
}
