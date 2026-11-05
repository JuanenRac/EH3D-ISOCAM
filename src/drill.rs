//! The holes and the outline of a board: Excellon drill files read (with the reader of the Bungard formats for the Bungard files in mils, and the reader of the suite for the
//! rest) and written (in mils, as the Bungard programs expect), board outlines read from Gerber, DXF or plotter files, and the path of the tool round the board.

use crate::bitmap::Bitmap;
use crate::contour::{loops, orient_climb, simplify, Grid, Loop};
use crate::edt::distance_mm;
use crate::isolate::Pass;
use crate::order::Cut;
use eh3d_bungard_formats::{dxf, gerber, hpgl as plotter, ncd};
use eh3d_gerberviewer::scene::Bounds;

pub use eh3d_cam::hole::Hole;

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

/// The number of digits the coordinates of a drill file without decimal points are written with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExcellonFormat {
    /// What the file says; a file in inches that says nothing is in whole mils (the Bungard convention), and one in millimetres in 3.3.
    #[default]
    Auto,
    /// Two digits before the point and four after (`X012340` is 1.2340), with the zeros as the header says (`TZ` or `LZ`). A file that states its own format keeps it.
    TwoFour,
}

impl ExcellonFormat {
    /// The value of the setting `excellon_format`: `auto` or `2.4`.
    pub fn parse(v: &str) -> Result<ExcellonFormat, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(ExcellonFormat::Auto),
            "2.4" => Ok(ExcellonFormat::TwoFour),
            _ => Err(format!("excellon_format '{}' is not auto or 2.4", v.trim())),
        }
    }

    /// The value as the setting is written.
    pub fn name(self) -> &'static str {
        match self {
            ExcellonFormat::Auto => "auto",
            ExcellonFormat::TwoFour => "2.4",
        }
    }
}

/// The most a coordinate of the 2.4 format can hold: 99.9999 in, written as six digits.
const MAX_TWO_FOUR: i64 = 999_999;

/// A Bungard file in whole mils, read with the reader of those files; `None` when the file is not one.
fn read_whole_mils(text: &str) -> Option<Result<Drills, String>> {
    let f = ncd::parse(text).ok()?;
    if f.metric || f.holes.is_empty() {
        return None;
    }
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
    Some(check_holes(Drills {
        holes,
        notes: Vec::new(),
    }))
}

/// Reads an Excellon drill file. A Bungard file in mils is read with the reader of those files; a metric file, or one with decimal coordinates, with the reader of the suite.
pub fn read_drills(text: &str) -> Result<Drills, String> {
    read_drills_with_units(text, None)
}

/// Whether a file states the format of its coordinates itself: a `FILE_FORMAT=2:4` comment, or the digits after the units (`INCH,TZ,00.0000`). Such a file is read by the reader of the
/// suite, which honours it, and never as whole mils.
fn states_its_format(text: &str) -> bool {
    text.lines().any(|raw| {
        let t: String = raw
            .trim()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        if let Some(c) = t.strip_prefix(';') {
            return c.starts_with("FILE_FORMAT=");
        }
        (t.starts_with("INCH") || t.starts_with("METRIC"))
            && t.split(',')
                .skip(1)
                .any(|p| p.contains('.') && p.chars().all(|c| c.is_ascii_digit() || c == '.'))
    })
}

/// Like [`read_drills`] with the units of the file given by the person (`--units`): they win over what the file says. A file that does not say its units and whose holes are so
/// close together that it may be in inches is refused unless the units are given.
pub fn read_drills_with_units(
    text: &str,
    forced: Option<eh3d_parser_excellon::Units>,
) -> Result<Drills, String> {
    read_drills_with_format(text, forced, ExcellonFormat::Auto)
}

/// Like [`read_drills_with_units`] with the format of the coordinates given by the person (`--excellon-format`). With 2.4 the file is read by the reader of the suite as a file of two
/// digits before the point and four after, unless it states a format of its own; the zeros are as its header says. A file that states its format is never read as whole mils.
pub fn read_drills_with_format(
    text: &str,
    forced: Option<eh3d_parser_excellon::Units>,
    format: ExcellonFormat,
) -> Result<Drills, String> {
    let two_four = format == ExcellonFormat::TwoFour;
    if !two_four && !states_its_format(text) {
        if let Some(mils) = read_whole_mils(text) {
            return mils;
        }
    }
    // the format asked for goes first as a comment, which the reader takes as a default: a format that the file states itself comes later and wins; the line it takes is not counted
    let framed;
    let (text, shift) = if two_four {
        framed = format!("; FILE_FORMAT=2:4\n{text}");
        (framed.as_str(), 1)
    } else {
        (text, 0)
    };
    let f = match forced {
        Some(units) => eh3d_parser_excellon::parse_with_units(text, units),
        None => eh3d_parser_excellon::parse(text),
    };
    if forced.is_none()
        && f.units.is_none()
        && f.diagnostics
            .iter()
            .any(|d| d.code == eh3d_parser_excellon::codes::UNITS_SUSPECT)
    {
        return Err("the file does not say its units and its holes are so close together that it may be in inches: give the units with --units mm or --units inch".into());
    }
    let mut notes = Vec::new();
    let units = f.units.unwrap_or_else(|| {
        notes.push("the file does not say its units: millimetres are assumed; if it is in inches, give --units inch".into());
        eh3d_parser_excellon::Units::Millimetres
    });
    if let Some(d) = f
        .diagnostics
        .iter()
        .find(|d| d.severity == eh3d_parser_excellon::Severity::Error)
    {
        if f.drills.is_empty() {
            return Err(format!(
                "line {}: {}",
                d.line.saturating_sub(shift),
                d.message
            ));
        }
        notes.push(format!(
            "line {}: {}",
            d.line.saturating_sub(shift),
            d.message
        ));
    }
    if !f.slots.is_empty() || !f.routes.is_empty() {
        notes.push(format!(
            "{} slots and {} routed paths in the file are not holes and were left out",
            f.slots.len(),
            f.routes.len()
        ));
    }
    // the holes keep their tool as a number of 1 to 255: a larger one gets a free number so that two tools never merge
    let used: std::collections::BTreeSet<u32> = f.drills.iter().map(|h| h.tool).collect();
    let mut free = (1..=u32::from(u8::MAX)).rev().filter(|n| !used.contains(n));
    let mut renumbered: std::collections::BTreeMap<u32, u8> = std::collections::BTreeMap::new();
    let mut unresolved: Vec<u32> = Vec::new();
    for &t in used.iter().filter(|t| **t > u32::from(u8::MAX)) {
        match free.next() {
            Some(n) => {
                renumbered.insert(t, n as u8);
            }
            None => unresolved.push(t),
        }
    }
    for (from, to) in &renumbered {
        notes.push(format!(
            "tool-number-too-large: tool {from} is above 255 and was renumbered as {to}"
        ));
    }
    for t in &unresolved {
        notes.push(format!(
            "tool-number-too-large: tool {t} is above 255, no free number was left and it was read as 255"
        ));
    }
    let holes = f
        .drills
        .iter()
        .map(|h| Hole {
            tool: u8::try_from(h.tool)
                .unwrap_or_else(|_| renumbered.get(&h.tool).copied().unwrap_or(u8::MAX)),
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

/// A coordinate of the 2.4 format: the position in ten-thousandths of an inch as six digits, with a minus sign in front when it is negative.
fn two_four(mm: f64) -> Result<String, String> {
    if !mm.is_finite() {
        return Err(format!("{mm} mm is out of range for a drill file"));
    }
    let v = (mm / 25.4 * 10_000.0).round();
    if v.abs() > MAX_TWO_FOUR as f64 {
        return Err(format!(
            "{mm} mm does not fit the 2.4 format of a drill file (at most 99.9999 in)"
        ));
    }
    let v = v as i64;
    Ok(if v < 0 {
        format!("-{:06}", -v)
    } else {
        format!("{v:06}")
    })
}

/// The text of one position (`mm`) in the format asked for: whole mils filled with zeros in front up to `digits` (`Auto`), or six digits of the 2.4 format.
fn position(mm: f64, format: ExcellonFormat, digits: u8) -> Result<String, String> {
    match format {
        ExcellonFormat::Auto => Ok(coordinate(mil(mm)?, digits)),
        ExcellonFormat::TwoFour => two_four(mm),
    }
}

/// The first lines of a file in the 2.4 format, drills and routing alike: `INCH,TZ`, the format as a comment, and a note of how far a position can be off by the rounding.
const TWO_FOUR_HEAD: &str = "M48\nINCH,TZ\n; FILE_FORMAT=2:4\n; coordinates are inches with 2 digits before the point and 4 after, in 6 digits; each is rounded to the nearest 0.0001 in, at most 0.00127 mm off\n";

/// The holes as an Excellon file in whole mils, as `style` says.
pub fn write_drills_as(holes: &[Hole], style: &DrillStyle) -> Result<String, String> {
    write_drills_in_format(holes, style, ExcellonFormat::Auto)
}

/// Like [`write_drills_as`] with the format of the coordinates: `Auto` writes whole mils, `TwoFour` writes inches with two digits before the point and four after, always six digits
/// (`X012340` is 1.2340 in), announced by `INCH,TZ` and a `FILE_FORMAT=2:4` comment. The plain style of the Bungard programs is whole mils only, so it is refused with 2.4.
pub fn write_drills_in_format(
    holes: &[Hole],
    style: &DrillStyle,
    format: ExcellonFormat,
) -> Result<String, String> {
    let fixed = format == ExcellonFormat::TwoFour;
    if style.plain && fixed {
        return Err("the plain style of the Bungard programs (ncd_isocam_style) is written in whole mils: it cannot be written in the 2.4 format".into());
    }
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
    // the coordinates are whole mils with no decimal point (the convention of the Bungard programs), not the 2.4 format that other Excellon readers assume for inches: the file says so,
    // with the most a position can be off by the rounding to a mil
    let mut s = String::from(if fixed {
        TWO_FOUR_HEAD
    } else {
        "M48\nINCH\n; coordinates are whole mils (0.001 in) without a decimal point; each is rounded to the nearest mil, at most 0.0127 mm off\n"
    });
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
                position(h.x_mm, format, style.digits)?,
                position(h.y_mm, format, style.digits)?
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
    write_route_excellon_in_format(cuts, tool_mm, tool, ExcellonFormat::Auto)
}

/// Like [`write_route_excellon`] with the format of the coordinates: `Auto` writes whole mils, `TwoFour` writes inches in six digits (two before the point, four after) announced by
/// `INCH,TZ` and a `FILE_FORMAT=2:4` comment, as the drill files are.
pub fn write_route_excellon_in_format(
    cuts: &[Cut],
    tool_mm: f64,
    tool: u8,
    format: ExcellonFormat,
) -> Result<String, String> {
    if !(tool_mm.is_finite() && tool_mm > 0.0 && tool_mm <= 50.0) {
        return Err(format!("the tool is {tool_mm} mm: it must be from 0 to 50"));
    }
    if tool == 0 {
        return Err("the tool number is 0: tools are numbered from 1".into());
    }
    if !cuts.iter().any(|c| c.pts.len() >= 2) {
        return Err("there is nothing to route".into());
    }
    let head = if format == ExcellonFormat::TwoFour {
        TWO_FOUR_HEAD
    } else {
        "M48\nINCH\n"
    };
    let mut s = format!("{head}T{tool}C{}\n%\nT{tool}\n", inches(tool_mm));
    let xy = |p: &(f64, f64)| -> Result<String, String> {
        Ok(format!(
            "X{}Y{}",
            position(p.0, format, 0)?,
            position(p.1, format, 0)?
        ))
    };
    for c in cuts.iter().filter(|c| c.pts.len() >= 2) {
        s.push_str(&format!("G00{}\nM15\n", xy(&c.pts[0])?));
        let rest = c.pts.iter().skip(1).chain(c.closed.then(|| &c.pts[0]));
        for p in rest {
            s.push_str(&format!("G01{}\n", xy(p)?));
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
    use std::collections::HashMap;
    type Run = Vec<(f64, f64)>;
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) <= tol;
    let mut left: Vec<Option<Run>> = runs
        .into_iter()
        .filter(|r| r.len() >= 2)
        .map(Some)
        .collect();
    // the two ends of every run in a grid of cells as wide as the tolerance: the run that meets a point is found by looking at the nine cells around it
    // (the plain search looked at every run left for every join, which is quadratic in the runs)
    let size = if tol.is_finite() && tol > 1e-9 {
        tol
    } else {
        1e-9
    };
    let key = |p: (f64, f64)| ((p.0 / size).floor() as i64, (p.1 / size).floor() as i64);
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, r) in left.iter().enumerate() {
        if let Some(r) = r {
            for p in [r[0], r[r.len() - 1]] {
                grid.entry(key(p)).or_default().push(i);
            }
        }
    }
    // the first run of the list (by its place) with an end within the tolerance of `p`; a run already taken is skipped
    let find = |left: &[Option<Run>], p: (f64, f64)| -> Option<usize> {
        let (kx, ky) = key(p);
        let mut best: Option<usize> = None;
        for dx in -1..=1_i64 {
            for dy in -1..=1_i64 {
                if let Some(ids) = grid.get(&(kx.saturating_add(dx), ky.saturating_add(dy))) {
                    for &i in ids {
                        if best.is_some_and(|b| b <= i) {
                            continue;
                        }
                        if let Some(r) = &left[i] {
                            if near(r[0], p) || near(r[r.len() - 1], p) {
                                best = Some(i);
                            }
                        }
                    }
                }
            }
        }
        best
    };
    let mut out = Outline::default();
    let mut top = left.len();
    loop {
        // the last run that is left starts the next outline
        let mut run = loop {
            if top == 0 {
                return out;
            }
            top -= 1;
            if let Some(r) = left[top].take() {
                break r;
            }
        };
        loop {
            let end = run[run.len() - 1];
            if near(run[0], end) && run.len() >= 4 {
                run.pop();
                out.closed.push(run);
                break;
            }
            match find(&left, end) {
                Some(i) => {
                    let Some(mut r) = left[i].take() else {
                        continue;
                    };
                    if !near(r[0], end) {
                        r.reverse();
                    }
                    run.extend(r.into_iter().skip(1));
                }
                None => {
                    // the start may meet the end of another run
                    let start = run[0];
                    match find(&left, start) {
                        Some(i) => {
                            let Some(mut r) = left[i].take() else {
                                continue;
                            };
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
