//! The copper drawn on a grid of square cells of `res` mm: a cell is copper when its centre is inside a pad, a track or a region.

use eh3d_gerberviewer::scene::{Bounds, Scene, Shape};

/// The most cells a grid may have (a board of 200 x 200 mm at 0.02 mm is 100 million: too many; the tool says so instead of running out of memory).
pub const MAX_CELLS: usize = 60_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    pub res: f64,
    /// The lower left corner of the grid, in mm.
    pub x0: f64,
    pub y0: f64,
    pub on: Vec<bool>,
}

impl Bitmap {
    pub fn new(bounds: &Bounds, margin_mm: f64, res: f64) -> Result<Bitmap, String> {
        if !(res.is_finite() && (0.002..=1.0).contains(&res)) {
            return Err(format!("the cell is {res} mm: it must be from 0.002 to 1"));
        }
        if bounds.is_empty() {
            return Err("the layer has no copper".into());
        }
        let m = margin_mm.max(0.0);
        let (x0, y0) = (bounds.min_x - m, bounds.min_y - m);
        let w = ((bounds.width() + 2.0 * m) / res).ceil() as usize + 1;
        let h = ((bounds.height() + 2.0 * m) / res).ceil() as usize + 1;
        if w.saturating_mul(h) > MAX_CELLS {
            return Err(format!("{w} x {h} cells is more than the {MAX_CELLS} allowed: use a larger cell or a smaller board"));
        }
        Ok(Bitmap {
            w,
            h,
            res,
            x0,
            y0,
            on: vec![false; w * h],
        })
    }

    /// The centre of the cell `(ix, iy)` in mm.
    pub fn centre(&self, ix: f64, iy: f64) -> (f64, f64) {
        (
            self.x0 + (ix + 0.5) * self.res,
            self.y0 + (iy + 0.5) * self.res,
        )
    }

    pub fn count(&self) -> usize {
        self.on.iter().filter(|b| **b).count()
    }

    fn fill(
        &mut self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        inside: impl Fn(f64, f64) -> bool,
    ) {
        let cell = |v: f64, o: f64| ((v - o) / self.res - 0.5).floor().max(0.0) as usize;
        let (ix0, ix1) = (
            cell(x_min, self.x0),
            (cell(x_max, self.x0) + 1).min(self.w.saturating_sub(1)),
        );
        let (iy0, iy1) = (
            cell(y_min, self.y0),
            (cell(y_max, self.y0) + 1).min(self.h.saturating_sub(1)),
        );
        for iy in iy0..=iy1 {
            for ix in ix0..=ix1 {
                let (x, y) = self.centre(ix as f64, iy as f64);
                if inside(x, y) {
                    self.on[iy * self.w + ix] = true;
                }
            }
        }
    }

    /// Draws the shapes of a scene.
    pub fn draw(&mut self, scene: &Scene) {
        for s in &scene.shapes {
            match s {
                Shape::Disc { x, y, r } => self.fill(x - r, x + r, y - r, y + r, |px, py| {
                    (px - x).powi(2) + (py - y).powi(2) <= r * r
                }),
                Shape::Rect { x, y, w, h } => self.fill(
                    x - w / 2.0,
                    x + w / 2.0,
                    y - h / 2.0,
                    y + h / 2.0,
                    |px, py| (px - x).abs() <= w / 2.0 && (py - y).abs() <= h / 2.0,
                ),
                Shape::Obround { x, y, w, h } => {
                    // a capsule along the longer side
                    let (r, half) = (w.min(*h) / 2.0, (w.max(*h) - w.min(*h)) / 2.0);
                    let (ax, ay, bx, by) = if w >= h {
                        (x - half, *y, x + half, *y)
                    } else {
                        (*x, y - half, *x, y + half)
                    };
                    self.fill(
                        x - w / 2.0,
                        x + w / 2.0,
                        y - h / 2.0,
                        y + h / 2.0,
                        |px, py| capsule_distance(px, py, ax, ay, bx, by) <= r,
                    )
                }
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    let r = w / 2.0;
                    self.fill(
                        x0.min(*x1) - r,
                        x0.max(*x1) + r,
                        y0.min(*y1) - r,
                        y0.max(*y1) + r,
                        |px, py| capsule_distance(px, py, *x0, *y0, *x1, *y1) <= r,
                    )
                }
                Shape::Region { pts } => {
                    let (min_x, max_x) = (
                        pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                        pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                    );
                    let (min_y, max_y) = (
                        pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                        pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                    );
                    self.fill(min_x, max_x, min_y, max_y, |px, py| {
                        inside_polygon(px, py, pts)
                    });
                }
            }
        }
    }
}

fn capsule_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    ((px - (ax + t * dx)).powi(2) + (py - (ay + t * dy)).powi(2)).sqrt()
}

/// Even-odd test of a point against a polygon.
fn inside_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi, xj, yj) = (pts[i].0, pts[i].1, pts[j].0, pts[j].1);
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The copper of a scene on a grid with a margin around it.
pub fn from_scene(scene: &Scene, margin_mm: f64, res: f64) -> Result<Bitmap, String> {
    let mut b = Bitmap::new(&scene.bounds(), margin_mm, res)?;
    b.draw(scene);
    Ok(b)
}
