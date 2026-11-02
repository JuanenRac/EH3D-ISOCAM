//! What kind of file an input is, from its text: extended Gerber (it states its own format), old-style Gerber (blocks ending in `*` with D codes but no format statement),
//! an Excellon drill file, an HP-GL plot, a DXF drawing, or something else. The commands use it to say plainly "this is a drill file" when a Gerber layer was expected.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Gerber with a format statement (`%FS`).
    Gerber,
    /// Gerber without a format statement: it needs the format and the list of apertures from outside.
    GerberOldStyle,
    Excellon,
    Hpgl,
    Dxf,
    Unknown,
}

impl FileKind {
    pub fn name(self) -> &'static str {
        match self {
            FileKind::Gerber => "Gerber layer (with a format statement)",
            FileKind::GerberOldStyle => "old-style Gerber layer (no format statement)",
            FileKind::Excellon => "Excellon drill file",
            FileKind::Hpgl => "HP-GL plot file",
            FileKind::Dxf => "DXF drawing",
            FileKind::Unknown => "file of a kind that is not known",
        }
    }

    /// True for both kinds of Gerber.
    pub fn is_gerber(self) -> bool {
        matches!(self, FileKind::Gerber | FileKind::GerberOldStyle)
    }
}

/// Is `tok` (already trimmed) an HP-GL instruction: two capital letters of the set used by plots, then nothing or a number?
fn hpgl_token(tok: &str) -> Option<&str> {
    let t = tok.trim();
    let op = t.get(..2)?;
    let rest = &t[2..];
    let known = matches!(op, "IN" | "SP" | "PU" | "PD" | "PA" | "PR" | "VS" | "LT");
    let tail_ok = rest
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '-' | '+' | ' '));
    (known && tail_ok).then_some(op)
}

fn is_tool_definition(line: &str) -> bool {
    // T01C0.800 / T1C0.8F200S65
    let l = line.trim();
    let Some(r) = l.strip_prefix('T') else {
        return false;
    };
    let digits = r.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return false;
    }
    let r = &r[digits..];
    r.strip_prefix('C').is_some_and(|c| {
        c.chars()
            .next()
            .is_some_and(|d| d.is_ascii_digit() || d == '.')
    })
}

/// The kind of a file, from its text. A file that cannot be told is `Unknown`.
pub fn detect(text: &str) -> FileKind {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    // DXF: pairs of lines, a group code and a value; a drawing starts with `0` / `SECTION` and ends with `0` / `EOF`
    let first: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| !l.is_empty())
        .take(2)
        .collect();
    if first.len() == 2
        && (first[0] == "0" || first[0] == "999")
        && (first[1].eq_ignore_ascii_case("SECTION") || first[0] == "999")
        && lines.iter().any(|l| l.eq_ignore_ascii_case("EOF"))
        && lines.iter().any(|l| l.eq_ignore_ascii_case("SECTION"))
    {
        return FileKind::Dxf;
    }
    if text.contains("%FS") {
        return FileKind::Gerber;
    }
    let stars = text.matches('*').count();
    // HP-GL: instructions separated by `;` or line ends, no `*`
    if stars == 0 {
        let toks: Vec<&str> = text.split([';', '\n']).filter_map(hpgl_token).collect();
        let draws = toks
            .iter()
            .filter(|t| matches!(**t, "PU" | "PD" | "PA" | "PR"))
            .count();
        if draws >= 2 && (toks.contains(&"IN") || toks.contains(&"SP") || draws >= 4) {
            return FileKind::Hpgl;
        }
    }
    // Excellon: a header with M48 or tool definitions, and no `*` blocks
    let defs = lines.iter().filter(|l| is_tool_definition(l)).count();
    if lines.contains(&"M48") || (defs >= 1 && stars == 0 && lines.contains(&"M30")) {
        return FileKind::Excellon;
    }
    if defs >= 1 && stars == 0 && lines.iter().any(|l| l.starts_with(['X', 'Y'])) {
        return FileKind::Excellon;
    }
    // Excellon with no header line: a `%`, tools by number alone and the holes (the drill files of CAM programs that leave the header out)
    let bare_tools = lines
        .iter()
        .filter(|l| {
            l.strip_prefix('T').is_some_and(|r| {
                (1..=2).contains(&r.len()) && r.chars().all(|c| c.is_ascii_digit())
            })
        })
        .count();
    if stars == 0
        && lines.contains(&"%")
        && bare_tools >= 1
        && lines.iter().any(|l| l.starts_with(['X', 'Y']))
    {
        return FileKind::Excellon;
    }
    // old-style Gerber: blocks that end in `*` and draw with D01, D02 or D03
    if stars >= 2 {
        let draws = text
            .split('*')
            .filter(|b| {
                let b = b.trim();
                (b.contains("D01") || b.contains("D02") || b.contains("D03"))
                    && b.contains(['X', 'Y'])
            })
            .count();
        if draws >= 1 {
            return FileKind::GerberOldStyle;
        }
    }
    FileKind::Unknown
}

/// `Ok` when a file of this kind can be read as a Gerber layer (or when it cannot be told); otherwise the plain reason and what to do.
pub fn expect_gerber(name: &str, kind: FileKind) -> Result<(), String> {
    match kind {
        FileKind::Excellon => Err(format!(
            "{name} is an Excellon drill file, not a Gerber layer: give it as --drills FILE, or to the drills command"
        )),
        FileKind::Hpgl => Err(format!(
            "{name} is an HP-GL plot file, not a Gerber layer: a plot is read as the outline of the board (--outline FILE, or the route command)"
        )),
        FileKind::Dxf => Err(format!(
            "{name} is a DXF drawing, not a Gerber layer: a drawing is read as the outline of the board (--outline FILE, or the route command)"
        )),
        _ => Ok(()),
    }
}

/// `Ok` when a file of this kind can be read as a drill file (or when it cannot be told).
pub fn expect_drills(name: &str, kind: FileKind) -> Result<(), String> {
    match kind {
        FileKind::Gerber | FileKind::GerberOldStyle => Err(format!(
            "{name} is a Gerber layer, not a drill file: give the layer as the first argument and the holes as an Excellon file"
        )),
        FileKind::Hpgl => Err(format!("{name} is an HP-GL plot file, not a drill file")),
        FileKind::Dxf => Err(format!("{name} is a DXF drawing, not a drill file")),
        _ => Ok(()),
    }
}
