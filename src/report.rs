//! What is told before a job is cut: the report of the job (its passes, tools, arcs, time and the findings of the dry run), the tools it needs set against the table of the crib of
//! EH3D-TOOL-MANAGER, and the checks of the layer (places where the tool does not fit, problem zones and design-rule findings). All of it is plain code that the command line and
//! the window share; the sentences are in the eight languages of the window.

use crate::board;
use crate::config::Settings;
use crate::crib::{self, CribTool};
use crate::isolate::Pass;
use crate::lang::{fill, Lang, Msg};
use crate::simulate;
use crate::tolerance::close_pieces_scoped;
use crate::transform::Board;

/// The width a tool is taken to cut, in mm, within which a tool of the crib is the one the job needs.
pub const CRIB_MATCH_MM: f64 = 0.005;

/// A tool of the job and the tool of the crib that cuts it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCheck {
    pub width_mm: f64,
    /// The number and the name of the crib tool; none when no tool of the crib cuts that width (or when the crib was not read).
    pub found: Option<(u32, String)>,
}

/// The report of a job.
#[derive(Debug, Clone, PartialEq)]
pub struct JobReport {
    pub passes: usize,
    pub paths: usize,
    pub cut_mm: f64,
    /// The moves of the program and how many of them are arcs.
    pub moves: usize,
    pub arcs: usize,
    /// The tools of the job, in the order they are used, each with its match in the crib.
    pub tools: Vec<ToolCheck>,
    /// Whether the table of the crib was given (when it was not, the tools are not checked).
    pub crib_read: bool,
    /// The places where the tool does not fit and nothing was laid.
    pub unfixed: usize,
    /// The dry run of the program the paths would be written as.
    pub dry: simulate::Report,
}

impl JobReport {
    /// How many things there are to look at before the job: the findings of the dry run, the tools the crib does not have and the places where the tool does not fit.
    pub fn findings(&self) -> usize {
        self.dry.warnings.len()
            + self
                .tools
                .iter()
                .filter(|t| self.crib_read && t.found.is_none())
                .count()
            + usize::from(self.unfixed > 0)
    }

    /// The report as lines in a language; `length` writes a length in mm the way the person reads it.
    pub fn text(&self, lang: Lang, length: &dyn Fn(f64) -> String) -> String {
        let mut t = String::new();
        let line = |t: &mut String, m: Msg, v: &[&str]| {
            t.push_str(&fill(m.tr(lang), v));
            t.push('\n');
        };
        line(
            &mut t,
            Msg::ReportJob,
            &[
                &self.passes.to_string(),
                &self.paths.to_string(),
                &length(self.cut_mm),
            ],
        );
        let widths: Vec<String> = self.tools.iter().map(|k| length(k.width_mm)).collect();
        if !widths.is_empty() {
            line(&mut t, Msg::ReportTools, &[&widths.join(", ")]);
        }
        if self.dry.stops > 0 {
            line(&mut t, Msg::ReportHand, &[&self.dry.stops.to_string()]);
        }
        if self.arcs > 0 {
            line(
                &mut t,
                Msg::ReportArcs,
                &[&self.arcs.to_string(), &self.moves.to_string()],
            );
        }
        if self.crib_read {
            for k in &self.tools {
                match &k.found {
                    Some((id, name)) => line(
                        &mut t,
                        Msg::ReportCribFound,
                        &[&length(k.width_mm), &id.to_string(), name],
                    ),
                    None => line(&mut t, Msg::ReportCribMissing, &[&length(k.width_mm)]),
                }
            }
        } else if !self.tools.is_empty() {
            line(&mut t, Msg::ReportCribNone, &[]);
        }
        if self.unfixed > 0 {
            line(&mut t, Msg::ReportGaps, &[&self.unfixed.to_string()]);
        }
        t.push_str(&self.dry.text(lang.code(), length));
        match self.findings() {
            0 => line(&mut t, Msg::ReportClean, &[]),
            n => line(&mut t, Msg::ReportFindings, &[&n.to_string()]),
        }
        t
    }
}

/// The tool of the crib that cuts `width_mm` at the depth of the job: an end mill of that diameter, or a V-bit or an engraver whose cut at that depth has that width.
fn crib_match(tools: &[CribTool], width_mm: f64, s: &Settings) -> Option<(u32, String)> {
    tools.iter().find_map(|k| {
        let w = crib::cutting_width_mm(k, s.depth_mm, s.crib_vbit_flat_mm).ok()?;
        ((w - width_mm).abs() <= CRIB_MATCH_MM).then(|| (k.id, k.name.clone()))
    })
}

/// The report of the job the passes make with these settings. `crib` is the table of the crib when it was read (`None` checks no tool); `unfixed` is the number of places where the
/// tool does not fit. The program is the one the paths would be written as, walked with the limits of the machine of the settings.
pub fn job_report(
    passes: &[Pass],
    s: &Settings,
    unfixed: usize,
    crib: Option<&[CribTool]>,
) -> Result<JobReport, String> {
    let (program, _) = board::gcode_program(passes, s, true)?;
    let dry = simulate::run_checked(&program, s.rapid_mm_min, &s.machine_limits());
    let mut widths: Vec<f64> = Vec::new();
    for p in passes.iter().filter(|p| p.diameter_mm > 0.0) {
        if !widths.iter().any(|w| (w - p.diameter_mm).abs() < 1e-9) {
            widths.push(p.diameter_mm);
        }
    }
    let tools = widths
        .into_iter()
        .map(|width_mm| ToolCheck {
            width_mm,
            found: crib.and_then(|c| crib_match(c, width_mm, s)),
        })
        .collect();
    Ok(JobReport {
        passes: passes.len(),
        paths: passes.iter().map(|p| p.cuts().len()).sum(),
        cut_mm: passes
            .iter()
            .flat_map(Pass::cuts)
            .map(|c| c.length_mm())
            .sum(),
        moves: dry.moves,
        arcs: program
            .lines()
            .filter(|l| l.starts_with("G2 ") || l.starts_with("G3 "))
            .count(),
        tools,
        crib_read: crib.is_some(),
        unfixed,
        dry,
    })
}

/// What the checks of a layer found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checks {
    /// Places where the finest tool does not fit between two pieces of copper.
    pub gaps: usize,
    /// Corners and narrow places where copper is left that the tool cannot reach.
    pub problem_zones: usize,
    /// Findings of the design rules.
    pub rules: usize,
    /// One line for each finding, with its place, in English.
    pub details: Vec<String>,
}

impl Checks {
    pub fn total(&self) -> usize {
        self.gaps + self.problem_zones + self.rules
    }

    /// The summary in a language.
    pub fn text(&self, lang: Lang) -> String {
        if self.total() == 0 {
            return Msg::ChecksClean.tr(lang).to_string();
        }
        fill(
            Msg::ChecksFound.tr(lang),
            &[
                &self.gaps.to_string(),
                &self.problem_zones.to_string(),
                &self.rules.to_string(),
            ],
        )
    }
}

/// The checks of a laid-out board with these settings: `check` (the places where the tool does not fit and the problem zones) and `drc` (the design rules, with the holes).
pub fn checks(board: &Board, s: &Settings) -> Result<Checks, String> {
    let pl = s.plan()?;
    pl.validate()?;
    let close = close_pieces_scoped(
        &board.scene,
        pl.finest_mm(),
        s.conflict_tolerance_mm,
        s.conflict_tolerance_scope,
        pl.res_mm,
    )?;
    let zones = crate::problems::problem_zones(&board.scene, &s.problem_rules(), pl.res_mm)?;
    let rules = crate::drc::check(&board.scene, &board.holes, &s.rules(), s.res_mm)?;
    let mut details = Vec::new();
    for c in &close {
        details.push(format!(
            "{} pieces of copper are closer than {} mm near ({:.3}, {:.3})",
            c.pieces,
            pl.finest_mm(),
            c.x,
            c.y
        ));
    }
    details.extend(zones.iter().map(|z| z.describe()));
    details.extend(rules.iter().map(|v| v.describe()));
    Ok(Checks {
        gaps: close.len(),
        problem_zones: zones.len(),
        rules: rules.len(),
        details,
    })
}
