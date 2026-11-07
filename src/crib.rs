//! The tools of the crib of EH3D-TOOL-MANAGER as the source of the tools of a job: the CSV table the crib program exports is read here (no dependency on its crates, only on the
//! table), a tool is chosen by its number and the width it cuts with becomes the tool of the plan. Plain code: nothing here opens a file.

use crate::config::Settings;

/// The kinds of tool the crib knows, with the words the table uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Drill,
    EndMill,
    VBit,
    Engraver,
    Laser,
    Nozzle,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Drill => "drill",
            Kind::EndMill => "end-mill",
            Kind::VBit => "v-bit",
            Kind::Engraver => "engraver",
            Kind::Laser => "laser",
            Kind::Nozzle => "nozzle",
        }
    }

    pub fn parse(text: &str) -> Result<Kind, String> {
        [
            Kind::Drill,
            Kind::EndMill,
            Kind::VBit,
            Kind::Engraver,
            Kind::Laser,
            Kind::Nozzle,
        ]
        .into_iter()
        .find(|k| k.key() == text.trim().to_ascii_lowercase())
        .ok_or_else(|| format!("'{text}' is not a kind of tool (drill, end-mill, v-bit, engraver, laser or nozzle)"))
    }
}

/// The widest tool a crib may hold, in mm (the limit of the crib program).
pub const MAX_DIAMETER_MM: f64 = 50.0;
/// The most tools read from a table.
pub const MAX_TOOLS: usize = 10_000;
/// The included angle of a V-bit or an engraver, in degrees.
pub const ANGLE_LIMITS: (f64, f64) = (10.0, 170.0);
/// The width of the flat tip of a V-bit, in mm.
pub const FLAT_LIMITS: (f64, f64) = (0.0, 5.0);
/// The narrowest cut a V-bit is said to make, in mm.
pub const MIN_WIDTH_MM: f64 = 0.01;
/// A width is cut down to a ten-thousandth of a millimetre (this many parts to the millimetre), so that the tool is never said to be wider than it is.
const PARTS_PER_MM: f64 = 10_000.0;

/// The columns of the table that are read; the others (wear counters, shank, material) are left alone.
const COLUMNS: [&str; 5] = ["id", "name", "kind", "diameter_mm", "tip_angle_deg"];

/// A tool of the crib, with what a milling job needs of it.
#[derive(Debug, Clone, PartialEq)]
pub struct CribTool {
    pub id: u32,
    pub name: String,
    pub kind: Kind,
    pub diameter_mm: f64,
    /// The included angle of a V-bit or an engraver.
    pub tip_angle_deg: Option<f64>,
}

/// The characters with which a spreadsheet would start a formula; the crib program puts a quote mark in front of a text that starts so.
const FORMULA_START: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

fn unformula(text: &str) -> String {
    match text.strip_prefix('\'') {
        Some(rest) if rest.starts_with(FORMULA_START) => rest.to_string(),
        _ => text.to_string(),
    }
}

/// Splits the table in rows of fields, honouring the double quotes. An unclosed quote, or text after a closing quote, is an error.
fn records(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c != '"' {
                field.push(c);
            } else if chars.peek() == Some(&'"') {
                field.push('"');
                chars.next();
            } else {
                quoted = false;
                closed = true;
            }
            continue;
        }
        if closed {
            if !matches!(c, ',' | '\n' | '\r') {
                return Err(format!("there is '{c}' after the closing quote of a field"));
            }
            closed = false;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if quoted {
        return Err("a quote is opened and never closed".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows.retain(|r| !(r.len() == 1 && r[0].trim().is_empty()));
    Ok(rows)
}

/// Reads the CSV table the crib program exports. The first row names the columns (`id`, `name`, `kind`, `diameter_mm` and `tip_angle_deg` are needed, in any order, and the
/// others are ignored); every row is checked and an error names the row.
pub fn parse_csv(text: &str) -> Result<Vec<CribTool>, String> {
    let rows = records(text.trim_start_matches('\u{feff}'))?;
    let (head, body) = rows.split_first().ok_or("the table is empty")?;
    let head: Vec<String> = head.iter().map(|h| h.trim().to_ascii_lowercase()).collect();
    let mut at = [0usize; 5];
    for (slot, name) in at.iter_mut().zip(COLUMNS) {
        *slot = head
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| format!("the table has no column '{name}'"))?;
    }
    if body.len() > MAX_TOOLS {
        return Err(format!(
            "the table has {} tools: more than the {MAX_TOOLS} allowed",
            body.len()
        ));
    }
    let mut tools: Vec<CribTool> = Vec::new();
    for (i, row) in body.iter().enumerate() {
        let n = i + 2;
        if row.len() != head.len() {
            return Err(format!(
                "row {n}: it has {} fields, not {}",
                row.len(),
                head.len()
            ));
        }
        let id = row[at[0]]
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|&v| v >= 1)
            .ok_or_else(|| {
                format!(
                    "row {n}: the id '{}' is not a whole number from 1",
                    row[at[0]]
                )
            })?;
        if tools.iter().any(|t| t.id == id) {
            return Err(format!("row {n}: the id {id} is used twice"));
        }
        let kind = Kind::parse(&row[at[2]]).map_err(|e| format!("row {n}: {e}"))?;
        let diameter_mm = row[at[3]]
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|d| d.is_finite() && *d > 0.0 && *d <= MAX_DIAMETER_MM)
            .ok_or_else(|| {
                format!(
                    "row {n}: diameter_mm '{}' must be a number above 0 and at most {MAX_DIAMETER_MM}",
                    row[at[3]]
                )
            })?;
        let tip_angle_deg = if row[at[4]].trim().is_empty() {
            None
        } else {
            let a = row[at[4]]
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|a| a.is_finite() && (ANGLE_LIMITS.0..=ANGLE_LIMITS.1).contains(a))
                .ok_or_else(|| {
                    format!(
                        "row {n}: tip_angle_deg '{}' must be a number from {} to {}",
                        row[at[4]], ANGLE_LIMITS.0, ANGLE_LIMITS.1
                    )
                })?;
            Some(a)
        };
        if matches!(kind, Kind::VBit | Kind::Engraver) && tip_angle_deg.is_none() {
            return Err(format!("row {n}: a {} needs its tip_angle_deg", kind.key()));
        }
        tools.push(CribTool {
            id,
            name: unformula(&row[at[1]]),
            kind,
            diameter_mm,
            tip_angle_deg,
        });
    }
    Ok(tools)
}

/// The width a tool cuts with at a depth, in mm. An end mill cuts with its diameter. A V-bit or an engraver cuts `2 * depth * tan(angle / 2) + flat` wide, no more than its
/// diameter and no less than `MIN_WIDTH_MM`, cut down to a ten-thousandth of a millimetre. Any other kind of tool does not mill the copper and is refused.
pub fn cutting_width_mm(tool: &CribTool, depth_mm: f64, flat_mm: f64) -> Result<f64, String> {
    match tool.kind {
        Kind::EndMill => Ok(tool.diameter_mm),
        Kind::VBit | Kind::Engraver => {
            if !depth_mm.is_finite() || !(0.0..=100.0).contains(&depth_mm) {
                return Err(format!(
                    "the depth is {depth_mm}: it must be from 0 to 100 mm"
                ));
            }
            if !flat_mm.is_finite() || !(FLAT_LIMITS.0..=FLAT_LIMITS.1).contains(&flat_mm) {
                return Err(format!(
                    "the flat is {flat_mm}: it must be from {} to {} mm",
                    FLAT_LIMITS.0, FLAT_LIMITS.1
                ));
            }
            let angle = tool
                .tip_angle_deg
                .ok_or_else(|| format!("the tool {} has no tip_angle_deg", tool.id))?;
            let width = 2.0 * depth_mm * (angle.to_radians() / 2.0).tan() + flat_mm;
            let width = width.min(tool.diameter_mm).max(MIN_WIDTH_MM);
            Ok((width * PARTS_PER_MM + 1e-6).floor() / PARTS_PER_MM)
        }
        other => Err(format!(
            "the tool {} ({}) is a {}: only an end mill, a V-bit or an engraver mills the copper",
            tool.id,
            tool.name,
            other.key()
        )),
    }
}

fn find(tools: &[CribTool], id: u32, setting: &str) -> Result<usize, String> {
    tools
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| format!("{setting} {id}: the crib has no tool with that number"))
}

/// Puts the tools the settings name into the settings: `crib_fine` and `crib_coarse` replace `fine_tool_mm` and `coarse_tool_mm` with the width the tool cuts with at `depth_mm`
/// (an end mill, a V-bit or an engraver), `crib_route` replaces `route_tool_mm` with the diameter of an end mill. A number of 0 leaves the setting alone. Returns one note for
/// each replacement; the settings are not validated here.
pub fn apply(settings: &mut Settings, tools: &[CribTool]) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let depth = settings.depth_mm;
    let flat = settings.crib_vbit_flat_mm;
    let wanted = [
        ("crib_fine", settings.crib_fine, "fine_tool_mm"),
        ("crib_coarse", settings.crib_coarse, "coarse_tool_mm"),
        ("crib_route", settings.crib_route, "route_tool_mm"),
    ];
    for (setting, id, target) in wanted {
        if id == 0 {
            continue;
        }
        let tool = &tools[find(tools, id, setting)?];
        let width = if setting == "crib_route" {
            if tool.kind != Kind::EndMill {
                return Err(format!(
                    "{setting} {id}: the cut round the board needs an end mill, and the tool {} ({}) is a {}",
                    tool.id,
                    tool.name,
                    tool.kind.key()
                ));
            }
            tool.diameter_mm
        } else {
            cutting_width_mm(tool, depth, flat).map_err(|e| format!("{setting} {id}: {e}"))?
        };
        match target {
            "fine_tool_mm" => settings.fine_tool_mm = width,
            "coarse_tool_mm" => settings.coarse_tool_mm = width,
            _ => settings.route_tool_mm = width,
        }
        notes.push(format!(
            "{target} = {width} from the crib: tool {} ({}, {})",
            tool.id,
            tool.name,
            tool.kind.key()
        ));
    }
    Ok(notes)
}
