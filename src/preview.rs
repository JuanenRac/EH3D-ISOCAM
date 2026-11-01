//! A look at the passes without a window: an SVG drawing (the copper in grey, each pass in a colour, drawn as wide as its tool) or a text picture.

use crate::bitmap::{capsule_distance, inside_polygon};
use crate::isolate::Pass;
use eh3d_gerberviewer::scene::{Scene, Shape};

type Pt = (f64, f64);

const COLOURS: [&str; 8] = [
    "#d95f02", "#1b9e77", "#7570b3", "#e7298a", "#66a61e", "#e6ab02", "#a6761d", "#1f78b4",
];

/// The most columns of the text picture.
pub const MAX_COLS: usize = 300;

fn extent(copper: &Scene, passes: &[Pass]) -> Result<(Pt, Pt), String> {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let b = copper.bounds();
    if !b.is_empty() {
        lo = (lo.0.min(b.min_x), lo.1.min(b.min_y));
        hi = (hi.0.max(b.max_x), hi.1.max(b.max_y));
    }
    for (k, pass) in passes.iter().enumerate() {
        for p in pass.paths.iter().chain(pass.open.iter()).flatten() {
            if !(p.0.is_finite() && p.1.is_finite()) {
                return Err(format!("pass {}: a point is not a number", k + 1));
            }
            lo = (lo.0.min(p.0), lo.1.min(p.1));
            hi = (hi.0.max(p.0), hi.1.max(p.1));
        }
    }
    if lo.0 > hi.0 || lo.1 > hi.1 {
        return Err("nothing to draw: there is no copper and no path".into());
    }
    Ok((lo, hi))
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The copper and the passes as an SVG drawing (millimetres, the y axis up).
pub fn svg(copper: &Scene, passes: &[Pass]) -> Result<String, String> {
    let (lo, hi) = extent(copper, passes)?;
    let widest = passes.iter().map(|p| p.diameter_mm).fold(0.0, f64::max);
    let m = 1.0 + widest;
    let (w, h) = (hi.0 - lo.0 + 2.0 * m, hi.1 - lo.1 + 2.0 * m);
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">\n<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n<g transform=\"translate({} {}) scale(1 -1)\">\n",
        num(w),
        num(h),
        num((w * 10.0).clamp(200.0, 2400.0)),
        num((h * 10.0).clamp(200.0, 2400.0)),
        num(w),
        num(h),
        num(m - lo.0),
        num(h - m + lo.1)
    );
    s.push_str("<g fill=\"#9a9a9a\" stroke=\"#9a9a9a\" stroke-linecap=\"round\">\n");
    for sh in &copper.shapes {
        match sh {
            Shape::Disc { x, y, r } => {
                s.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"none\"/>\n",
                    num(*x),
                    num(*y),
                    num(*r)
                ));
            }
            Shape::Rect { x, y, w, h } => {
                s.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"none\"/>\n",
                    num(x - w / 2.0),
                    num(y - h / 2.0),
                    num(*w),
                    num(*h)
                ));
            }
            Shape::Obround { x, y, w, h } => {
                s.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" stroke=\"none\"/>\n",
                    num(x - w / 2.0),
                    num(y - h / 2.0),
                    num(*w),
                    num(*h),
                    num(w.min(*h) / 2.0)
                ));
            }
            Shape::Stroke { x0, y0, x1, y1, w } => {
                s.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\"/>\n",
                    num(*x0),
                    num(*y0),
                    num(*x1),
                    num(*y1),
                    num(*w)
                ));
            }
            Shape::Region { pts } if pts.len() >= 3 => {
                let list: Vec<String> = pts
                    .iter()
                    .map(|p| format!("{},{}", num(p.0), num(p.1)))
                    .collect();
                s.push_str(&format!(
                    "<polygon points=\"{}\" stroke=\"none\"/>\n",
                    list.join(" ")
                ));
            }
            Shape::Region { .. } => {}
        }
    }
    s.push_str("</g>\n");
    for (k, pass) in passes.iter().enumerate() {
        let colour = COLOURS[k % COLOURS.len()];
        let width = if pass.diameter_mm > 0.0 {
            pass.diameter_mm
        } else {
            0.1
        };
        s.push_str(&format!(
            "<g fill=\"none\" stroke=\"{colour}\" stroke-opacity=\"0.55\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n<title>pass {} ({} mm tool, {} mm from the copper)</title>\n",
            num(width),
            k + 1,
            num(pass.diameter_mm),
            num(pass.offset_mm)
        ));
        for (list, tag) in [(&pass.paths, "polygon"), (&pass.open, "polyline")] {
            for path in list.iter().filter(|p| p.len() >= 2) {
                let pts: Vec<String> = path
                    .iter()
                    .map(|p| format!("{},{}", num(p.0), num(p.1)))
                    .collect();
                s.push_str(&format!("<{tag} points=\"{}\"/>\n", pts.join(" ")));
            }
        }
        s.push_str("</g>\n");
    }
    s.push_str("</g>\n</svg>\n");
    Ok(s)
}

fn mark(c: usize) -> char {
    "123456789abcdefghijklmnopqrstuvwxyz"
        .chars()
        .nth(c)
        .unwrap_or('*')
}

/// The copper (`#`) and the passes (`1`, `2`, ... one character for each) as text, with `cols` characters across. A key follows the picture.
pub fn text(copper: &Scene, passes: &[Pass], cols: usize) -> Result<String, String> {
    if !(10..=MAX_COLS).contains(&cols) {
        return Err(format!(
            "preview_cols is {cols}: it must be from 10 to {MAX_COLS}"
        ));
    }
    let (lo, hi) = extent(copper, passes)?;
    let (wmm, hmm) = ((hi.0 - lo.0).max(1e-6), (hi.1 - lo.1).max(1e-6));
    // a character is about twice as tall as it is wide
    let cs = wmm.max(hmm / 2.0) / cols as f64;
    let rows = ((hmm / (2.0 * cs)).ceil() as usize + 1).clamp(1, 2 * MAX_COLS);
    let cols = (wmm / cs).ceil() as usize + 1;
    let mut grid = vec![' '; cols * rows];
    let cell = |x: f64, y: f64| -> Option<usize> {
        let (cx, cy) = (((x - lo.0) / cs).floor(), ((y - lo.1) / (2.0 * cs)).floor());
        (cx >= 0.0 && cy >= 0.0 && (cx as usize) < cols && (cy as usize) < rows)
            .then(|| cy as usize * cols + cx as usize)
    };
    let centre = |i: usize| -> Pt {
        (
            lo.0 + ((i % cols) as f64 + 0.5) * cs,
            lo.1 + ((i / cols) as f64 + 0.5) * 2.0 * cs,
        )
    };
    for sh in &copper.shapes {
        let (bx, by, tx, ty) = match sh {
            Shape::Disc { x, y, r } => (x - r, y - r, x + r, y + r),
            Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
                (x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0)
            }
            Shape::Stroke { x0, y0, x1, y1, w } => (
                x0.min(*x1) - w / 2.0,
                y0.min(*y1) - w / 2.0,
                x0.max(*x1) + w / 2.0,
                y0.max(*y1) + w / 2.0,
            ),
            Shape::Region { pts } if pts.len() >= 3 => (
                pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
            ),
            Shape::Region { .. } => continue,
        };
        let (Some(a), Some(b)) = (
            cell(bx.max(lo.0), by.max(lo.1)),
            cell(tx.min(hi.0), ty.min(hi.1)),
        ) else {
            continue;
        };
        for iy in a / cols..=b / cols {
            for ix in a % cols..=b % cols {
                let i = iy * cols + ix;
                let (px, py) = centre(i);
                let inside = match sh {
                    Shape::Disc { x, y, r } => (px - x).hypot(py - y) <= *r,
                    Shape::Rect { x, y, w, h } => {
                        (px - x).abs() <= w / 2.0 && (py - y).abs() <= h / 2.0
                    }
                    Shape::Obround { x, y, w, h } => {
                        let (r, half) = (w.min(*h) / 2.0, (w.max(*h) - w.min(*h)) / 2.0);
                        if w >= h {
                            capsule_distance(px, py, x - half, *y, x + half, *y) <= r
                        } else {
                            capsule_distance(px, py, *x, y - half, *x, y + half) <= r
                        }
                    }
                    Shape::Stroke { x0, y0, x1, y1, w } => {
                        capsule_distance(px, py, *x0, *y0, *x1, *y1) <= w / 2.0
                    }
                    Shape::Region { pts } => inside_polygon(px, py, pts),
                };
                if inside {
                    grid[i] = '#';
                }
            }
        }
    }
    for (k, pass) in passes.iter().enumerate() {
        for (list, closed) in [(&pass.paths, true), (&pass.open, false)] {
            for path in list {
                let n = path.len();
                let segs = if closed { n } else { n.saturating_sub(1) };
                for i in 0..segs {
                    let (a, b) = (path[i], path[(i + 1) % n]);
                    let steps =
                        ((b.0 - a.0).hypot(b.1 - a.1) / (cs / 2.0)).ceil().max(1.0) as usize;
                    for t in 0..=steps {
                        let f = t as f64 / steps as f64;
                        if let Some(ix) = cell(a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f) {
                            grid[ix] = mark(k);
                        }
                    }
                }
            }
        }
    }
    let mut out = String::new();
    for row in (0..rows).rev() {
        let line: String = grid[row * cols..(row + 1) * cols].iter().collect();
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.push_str(&format!(
        "# copper; one character is {} x {} mm\n",
        num(cs),
        num(2.0 * cs)
    ));
    for (k, pass) in passes.iter().enumerate() {
        out.push_str(&format!(
            "{} pass {} ({} mm tool, {} mm from the copper, {} paths and {} runs)\n",
            mark(k),
            k + 1,
            num(pass.diameter_mm),
            num(pass.offset_mm),
            pass.paths.len(),
            pass.open.len()
        ));
    }
    Ok(out)
}
