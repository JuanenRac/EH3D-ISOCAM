//! The holes and the outline of a board: Excellon drill files read (with the reader of the Bungard formats for the Bungard files in mils, and the reader of the suite for the
//! rest) and written (in mils, as the Bungard programs expect), board outlines read from Gerber, DXF or plotter files, and the path of the tool round the board.

use crate::bitmap::Bitmap;
use crate::contour::{loops, orient_climb, simplify, Grid, Loop};
use crate::edt::distance_mm;
use crate::isolate::Pass;
use crate::order::Cut;
use eh3d_bungard_formats::{dxf, gerber, hpgl as plotter, ncd};
use eh3d_gerberviewer::scene::Bounds;

/// One hole: where, how wide and with which tool number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hole {
    pub tool: u8,
    pub x_mm: f64,
    pub y_mm: f64,
    /// 0 when the file does not say.
    pub diameter_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drills {
    pub holes: Vec<Hole>,
    /// What was not understood or not used, each with the reason.
    pub notes: Vec<String>,
}

/// The most coordinate, in mm, that is accepted (a board is never bigger).
const MAX_MM: f64 = 5_000.0;

fn mm_of_inch_or_mm(v: f64) -> f64 {
    // a header such as T1C0.8 can be inches or mm: below 0.2 it can only be inches
    if v < 0.2 {
        v * 25.4
    } else {
        v
    }
}

/// Reads an Excellon drill file. A Bungard file in mils is read with the reader of those files; a metric file, or one with decimal coordinates, with the reader of the suite.
pub fn read_drills(text: &str) -> Result<Drills, String> {
    if let Ok(f) = ncd::parse(text) {
        if !f.metric && !f.holes.is_empty() {
            let dia = |t: u8| {
                f.diameters
                    .iter()
                    .find(|d| d.0 == t)
                    .map_or(0.0, |d| mm_of_inch_or_mm(d.1))
            };
            let holes: Vec<Hole> = f
                .holes
                .iter()
                .map(|h| Hole {
                    tool: h.tool,
                    x_mm: h.x_mm(),
                    y_mm: h.y_mm(),
                    diameter_mm: dia(h.tool),
                })
                .collect();
            return check_holes(Drills {
                holes,
                notes: Vec::new(),
            });
        }
    }
    let f = eh3d_parser_excellon::parse(text);
    let mut notes = Vec::new();
    let units = f.units.unwrap_or_else(|| {
        notes.push("the file does not say its units: millimetres are assumed".into());
        eh3d_parser_excellon::Units::Millimetres
    });
    if let Some(d) = f
        .diagnostics
        .iter()
        .find(|d| d.severity == eh3d_parser_excellon::Severity::Error)
    {
        if f.drills.is_empty() {
            return Err(format!("line {}: {}", d.line, d.message));
        }
        notes.push(format!("line {}: {}", d.line, d.message));
    }
    if !f.slots.is_empty() || !f.routes.is_empty() {
        notes.push(format!(
            "{} slots and {} routed paths in the file are not holes and were left out",
            f.slots.len(),
            f.routes.len()
        ));
    }
    let holes = f
        .drills
        .iter()
        .map(|h| Hole {
            tool: u8::try_from(h.tool).unwrap_or(u8::MAX),
            x_mm: units.to_mm(h.at.x),
            y_mm: units.to_mm(h.at.y),
            diameter_mm: f
                .tools
                .get(&h.tool)
                .map_or(0.0, |t| units.to_mm(t.diameter)),
        })
        .collect();
    check_holes(Drills { holes, notes })
}

fn check_holes(d: Drills) -> Result<Drills, String> {
    if d.holes.is_empty() {
        return Err("the file has no holes".into());
    }
    for h in &d.holes {
        if !(h.x_mm.is_finite()
            && h.y_mm.is_finite()
            && h.x_mm.abs() <= MAX_MM
            && h.y_mm.abs() <= MAX_MM
            && h.diameter_mm.is_finite()
            && (0.0..=50.0).contains(&h.diameter_mm))
        {
            return Err(format!(
                "a hole at ({}, {}) with a diameter of {} mm is out of range",
                h.x_mm, h.y_mm, h.diameter_mm
            ));
        }
    }
    Ok(d)
}

fn mil(mm: f64) -> Result<i64, String> {
    if !(mm.is_finite() && mm.abs() <= MAX_MM) {
        return Err(format!("{mm} mm is out of range for a drill file"));
    }
    Ok((mm / ncd::MIL_MM).round() as i64)
}

fn inches(mm: f64) -> String {
    format!("{:.4}", mm / 25.4)
}

/// The holes as an Excellon file in whole mils, a tool after the other in the order they first appear. Holes that share a tool number must share its diameter.
pub fn write_drills(holes: &[Hole]) -> Result<String, String> {
    write_drills_as(holes, &DrillStyle::default())
}

/// How a drill file is written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrillStyle {
    /// The plain form of the files of the Bungard programs: `M48` and `%`, the tools by number only (no unit line, no diameters), a coordinate left out when it is the same as
    /// the one before it, CR LF line ends and `M00` at the end.
    pub plain: bool,
    /// Digits of a coordinate, filled with zeros in front; 0 writes the number as it is.
    pub digits: u8,
}

fn coordinate(v: i64, digits: u8) -> String {
    format!("{v:0w$}", w = usize::from(digits))
}

/// The holes as an Excellon file in whole mils, as `style` says.
pub fn write_drills_as(holes: &[Hole], style: &DrillStyle) -> Result<String, String> {
    if style.plain {
        return write_plain_drills(holes, style.digits);
    }
    if holes.is_empty() {
        return Err("there are no holes to write".into());
    }
    let mut tools: Vec<(u8, f64)> = Vec::new();
    for h in holes {
        if h.tool == 0 {
            return Err("a hole has tool number 0: tools are numbered from 1".into());
        }
        if !(h.diameter_mm.is_finite() && (0.0..=50.0).contains(&h.diameter_mm)) {
            return Err(format!(
                "a hole has a diameter of {} mm: it must be from 0 to 50",
                h.diameter_mm
            ));
        }
        match tools.iter().find(|t| t.0 == h.tool) {
            Some(t) if (t.1 - h.diameter_mm).abs() > 0.005 => {
                return Err(format!(
                    "tool {} is used with {} mm and with {} mm",
                    h.tool, t.1, h.diameter_mm
                ));
            }
            Some(_) => {}
            None => tools.push((h.tool, h.diameter_mm)),
        }
    }
    let mut s = String::from("M48\nINCH\n");
    for (t, d) in &tools {
        if *d > 0.0 {
            s.push_str(&format!("T{t}C{}\n", inches(*d)));
        } else {
            s.push_str(&format!("T{t}\n"));
        }
    }
    s.push_str("%\n");
    for (t, _) in &tools {
        s.push_str(&format!("T{t}\n"));
        for h in holes.iter().filter(|h| h.tool == *t) {
            s.push_str(&format!(
                "X{}Y{}\n",
                coordinate(mil(h.x_mm)?, style.digits),
                coordinate(mil(h.y_mm)?, style.digits)
            ));
        }
    }
    s.push_str("M30\n");
    Ok(s)
}

fn write_plain_drills(holes: &[Hole], digits: u8) -> Result<String, String> {
    if holes.is_empty() {
        return Err("there are no holes to write".into());
    }
    let mut tools: Vec<(u8, f64)> = Vec::new();
    for h in holes {
        if h.tool == 0 {
            return Err("a hole has tool number 0: tools are numbered from 1".into());
        }
        match tools.iter().find(|t| t.0 == h.tool) {
            Some(t) if (t.1 - h.diameter_mm).abs() > 0.005 => {
                return Err(format!(
                    "tool {} is used with {} mm and with {} mm",
                    h.tool, t.1, h.diameter_mm
                ));
            }
            Some(_) => {}
            None => tools.push((h.tool, h.diameter_mm)),
        }
    }
    let mut s = String::from("M48\r\n%\r\n");
    for (t, _) in &tools {
        s.push_str(&format!("T{t}\r\n"));
        let mut last: Option<(i64, i64)> = None;
        for h in holes.iter().filter(|h| h.tool == *t) {
            let (x, y) = (mil(h.x_mm)?, mil(h.y_mm)?);
            match last {
                Some((lx, ly)) if lx == x && ly != y => {
                    s.push_str(&format!("Y{}\r\n", coordinate(y, digits)));
                }
                Some((lx, ly)) if ly == y && lx != x => {
                    s.push_str(&format!("X{}\r\n", coordinate(x, digits)));
                }
                _ => s.push_str(&format!(
                    "X{}Y{}\r\n",
                    coordinate(x, digits),
                    coordinate(y, digits)
                )),
            }
            last = Some((x, y));
        }
    }
    s.push_str("M00\r\n");
    Ok(s)
}

/// The cuts as a routing Excellon file in whole mils: the tool is put down at the start of each cut (`M15`), moves in straight lines (`G01`) and is lifted (`M16`).
pub fn write_route_excellon(cuts: &[Cut], tool_mm: f64, tool: u8) -> Result<String, String> {
    if !(tool_mm.is_finite() && tool_mm > 0.0 && tool_mm <= 50.0) {
        return Err(format!("the tool is {tool_mm} mm: it must be from 0 to 50"));
    }
    if tool == 0 {
        return Err("the tool number is 0: tools are numbered from 1".into());
    }
    if !cuts.iter().any(|c| c.pts.len() >= 2) {
        return Err("there is nothing to route".into());
    }
    let mut s = format!("M48\nINCH\nT{tool}C{}\n%\nT{tool}\n", inches(tool_mm));
    for c in cuts.iter().filter(|c| c.pts.len() >= 2) {
        s.push_str(&format!(
            "G00X{}Y{}\nM15\n",
            mil(c.pts[0].0)?,
            mil(c.pts[0].1)?
        ));
        let rest = c.pts.iter().skip(1).chain(c.closed.then(|| &c.pts[0]));
        for p in rest {
            s.push_str(&format!("G01X{}Y{}\n", mil(p.0)?, mil(p.1)?));
        }
        s.push_str("M16\n");
    }
    s.push_str("M30\n");
    Ok(s)
}

/// The outline of a board: the paths that close, and the runs that do not.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outline {
    pub closed: Vec<Loop>,
    pub open: Vec<Loop>,
}

impl Outline {
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::empty();
        for p in self.closed.iter().chain(&self.open).flatten() {
            b.include(p.0, p.1);
        }
        b
    }
}

/// Joins runs that meet end to end (within `tol` mm), as an outline drawn as separate strokes does; a run that comes back to its start becomes a closed path.
pub fn join_runs(runs: Vec<Vec<(f64, f64)>>, tol: f64) -> Outline {
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) <= tol;
    let mut left: Vec<Vec<(f64, f64)>> = runs.into_iter().filter(|r| r.len() >= 2).collect();
    let mut out = Outline::default();
    while let Some(mut run) = left.pop() {
        loop {
            let end = run[run.len() - 1];
            if near(run[0], end) && run.len() >= 4 {
                run.pop();
                out.closed.push(run);
                break;
            }
            let next = left
                .iter()
                .position(|r| near(r[0], end) || near(r[r.len() - 1], end));
            match next {
                Some(i) => {
                    let mut r = left.remove(i);
                    if !near(r[0], end) {
                        r.reverse();
                    }
                    run.extend(r.into_iter().skip(1));
                }
                None => {
                    // the start may meet the end of another run
                    let start = run[0];
                    let prev = left
                        .iter()
                        .position(|r| near(r[r.len() - 1], start) || near(r[0], start));
                    match prev {
                        Some(i) => {
                            let mut r = left.remove(i);
                            if !near(r[r.len() - 1], start) {
                                r.reverse();
                            }
                            r.extend(run.into_iter().skip(1));
                            run = r;
                        }
                        None => {
                            out.open.push(run);
                            break;
                        }
                    }
                }
            }
        }
    }
    out
}

/// Reads the outline of a board from a Gerber outline layer, a DXF drawing or a plotter file (by the extension of the name).
pub fn read_outline(name: &str, text: &str) -> Result<Outline, String> {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    let plot = if ext == dxf::EXTENSION {
        dxf::outline_default(text)?
    } else if matches!(ext, "plt" | "hpgl" | "hp" | "hgl") {
        plotter::parse(text)
    } else {
        gerber::outline_default(text)?
    };
    let runs: Vec<Vec<(f64, f64)>> = plot
        .paths
        .iter()
        .map(|p| {
            p.points
                .iter()
                .map(|q| {
                    (
                        f64::from(q.0) * plotter::UNIT_MM,
                        f64::from(q.1) * plotter::UNIT_MM,
                    )
                })
                .collect()
        })
        .collect();
    let o = join_runs(runs, plotter::UNIT_MM * 1.5);
    if o.closed.is_empty() && o.open.is_empty() {
        return Err("the file has no outline".into());
    }
    Ok(o)
}

/// The path of a tool of `tool_mm` round the board and inside its cutouts: the lines at the radius of the tool from the board, outside it and inside the cutouts. A path that
/// does not close is left out (it is counted in the notes of the caller).
pub fn route_outline(o: &Outline, tool_mm: f64, res_mm: f64) -> Result<Pass, String> {
    if !(tool_mm.is_finite() && (0.1..=10.0).contains(&tool_mm)) {
        return Err(format!(
            "the tool is {tool_mm} mm: it must be from 0.1 to 10"
        ));
    }
    if !(res_mm.is_finite() && (0.002..=tool_mm / 2.0).contains(&res_mm)) {
        return Err(format!(
            "the cell is {res_mm} mm: it must be from 0.002 to half the tool"
        ));
    }
    if o.closed.is_empty() {
        return Err("the outline has no closed path: the tool cannot go round it".into());
    }
    let mut bm = Bitmap::new(&o.bounds(), tool_mm / 2.0 + 4.0 * res_mm, res_mm)?;
    bm.draw_even_odd(&o.closed);
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    let grid = Grid {
        dist: &dist,
        w: bm.w,
        h: bm.h,
        res: bm.res,
        x0: bm.x0,
        y0: bm.y0,
    };
    let r = tool_mm / 2.0;
    let paths: Vec<Loop> = loops(&grid, r + res_mm / 2.0)
        .into_iter()
        .map(|l| simplify(&l, res_mm / 2.0))
        .filter(|l| l.len() >= 3)
        .map(|mut l| {
            orient_climb(&grid, &mut l);
            l
        })
        .collect();
    if paths.is_empty() {
        return Err("no path of the tool was found round the outline".into());
    }
    Ok(Pass {
        offset_mm: r,
        paths,
        diameter_mm: tool_mm,
        ..Pass::default()
    })
}

/// Bridges that hold the board in the sheet while the tool cuts round it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tabs {
    /// How many bridges on each closed path.
    pub count: usize,
    /// How wide the bridge of material that is left is, measured between the edges of the cut on either side of it.
    pub width_mm: f64,
    /// The shortest piece of path that may go between two bridges, counted along the whole path: a path gets at most its length divided by this many bridges.
    pub min_length_mm: f64,
}

/// What `add_tabs` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TabReport {
    /// The closed paths that were opened into runs.
    pub paths_cut: usize,
    pub bridges: usize,
    /// The closed paths too short for a bridge, which are left closed.
    pub paths_left_closed: usize,
}

/// The point at the distance `d` along a closed path (`cum[i]` is the length up to point `i`, `total` the whole length).
fn point_along(pts: &[(f64, f64)], cum: &[f64], total: f64, d: f64) -> (f64, f64) {
    let d = d.rem_euclid(total);
    let n = pts.len();
    let i = cum
        .partition_point(|c| *c <= d)
        .saturating_sub(1)
        .min(n - 1);
    let (a, b) = (pts[i], pts[(i + 1) % n]);
    let seg = (if i + 1 < n { cum[i + 1] } else { total }) - cum[i];
    let k = if seg > 0.0 { (d - cum[i]) / seg } else { 0.0 };
    (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k)
}

/// Opens the closed paths of a pass into runs with a gap at each bridge. The gap is as long as the width of the bridge plus one tool (the tool stops half a width and half a tool
/// short of the middle of the bridge, so the material that is left is as wide as asked); the bridges are spread evenly along the path, the first half a span from its start.
/// A path that is too short for a bridge stays closed and is counted.
pub fn add_tabs(pass: &mut Pass, tool_mm: f64, t: &Tabs) -> Result<TabReport, String> {
    if !(tool_mm.is_finite() && tool_mm > 0.0) {
        return Err("the tool of the bridges must be wider than 0".into());
    }
    if !(t.width_mm.is_finite() && t.width_mm >= 0.1) {
        return Err(format!(
            "the bridge is {} mm wide: it must be at least 0.1",
            t.width_mm
        ));
    }
    if !(t.min_length_mm.is_finite() && t.min_length_mm >= 2.0 * tool_mm) {
        return Err(format!(
            "the shortest piece between bridges is {} mm: it must be at least two tools ({} mm)",
            t.min_length_mm,
            2.0 * tool_mm
        ));
    }
    let mut report = TabReport::default();
    if t.count == 0 {
        return Ok(report);
    }
    let half = (t.width_mm + tool_mm) / 2.0;
    let mut kept: Vec<Loop> = Vec::new();
    let mut runs: Vec<Loop> = Vec::new();
    for path in std::mem::take(&mut pass.paths) {
        let n = path.len();
        let mut cum = Vec::with_capacity(n);
        let mut total = 0.0;
        for i in 0..n {
            cum.push(total);
            let (a, b) = (path[i], path[(i + 1) % n]);
            total += (b.0 - a.0).hypot(b.1 - a.1);
        }
        let mut bridges = t.count.min((total / t.min_length_mm).floor() as usize);
        // the span between two bridges has to be longer than the gap of a bridge
        while bridges > 0 && total / bridges as f64 <= 2.0 * half + 0.01 {
            bridges -= 1;
        }
        if n < 3 || bridges == 0 {
            report.paths_left_closed += 1;
            kept.push(path);
            continue;
        }
        let span = total / bridges as f64;
        for k in 0..bridges {
            let (from, to) = (
                (k as f64 + 0.5) * span + half,
                (k as f64 + 1.5) * span - half,
            );
            let mut run = vec![point_along(&path, &cum, total, from)];
            for lap in 0..3 {
                for i in 0..n {
                    let d = lap as f64 * total + cum[i];
                    if d > from + 1e-9 && d < to - 1e-9 {
                        run.push(path[i]);
                    }
                }
            }
            run.push(point_along(&path, &cum, total, to));
            runs.push(run);
        }
        report.paths_cut += 1;
        report.bridges += bridges;
    }
    pass.paths = kept;
    pass.open.extend(runs);
    Ok(report)
}
