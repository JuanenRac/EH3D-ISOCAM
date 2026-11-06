//! Every value that can be tuned lives in one configuration file (`key = value`, one per line, `#` starts a comment) and has a default here. The command line takes the same
//! keys as options (`--fine-tool-mm 0.15` is `fine_tool_mm = 0.15`), so nothing tuned is hidden in the code.

use crate::drc::Rules;
use crate::drill::ExcellonFormat;
use crate::fiducial::{fit, Affine, Fit, Limits, Model, Pair};
use crate::forced::Forced;
use crate::gcode::Job;
use crate::isolate::{Plan, Rubout, Tool};
use crate::legacy::{self, Format, Zeros};
use crate::optimize::Optimize;
use crate::order::{Direction, LoopStart, Order, Strategy};
use crate::problems::ProblemRules;
use crate::tolerance::ToleranceScope;
use crate::transform::{Axis, HolePattern, Layout};
use crate::units::{parse_length, Unit};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub res_mm: f64,
    pub fine_tool_mm: f64,
    pub fine_passes: usize,
    pub fine_stepover_mm: f64,
    /// 0 switches the coarse tool off.
    pub coarse_tool_mm: f64,
    pub coarse_passes: usize,
    pub coarse_stepover_mm: f64,
    /// Leave out the stretches of the fine tool that the coarse tool has already cut along.
    pub trim_fine: bool,
    /// 0 switches the removal of the copper that is left over off.
    pub rubout_tool_mm: f64,
    pub rubout_width_mm: f64,
    pub rubout_overlap: f64,
    /// 0 (off): every tool cuts everywhere. Above 0: the wider tools cut only the rings round pads whose smaller side is at least this wide, and the narrowest tool does the rest.
    pub around_pads_min_mm: f64,
    pub depth_mm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub safe_z_mm: f64,
    /// The height for a tool change by hand.
    pub tool_change_z_mm: f64,
    pub spindle_rpm: f64,
    pub direction: Direction,
    pub start_x_mm: f64,
    pub start_y_mm: f64,
    pub loop_start: LoopStart,
    pub ordering: Strategy,
    pub optimize_max_paths: usize,
    /// The tool that cuts out the board.
    pub route_tool_mm: f64,
    pub alignment_hole_mm: f64,
    pub alignment_margin_mm: f64,
    /// 2, 3 or 6 holes (a special pattern has 2 or 6).
    pub alignment_holes: usize,
    /// Outside the board (normal) or in a fixed pattern about its middle (special).
    pub alignment_pattern: HolePattern,
    /// A special pattern: the distance between the two holes of the pair, and the distance between the two holes added on each side of it for six.
    pub alignment_special_a_mm: f64,
    pub alignment_special_b_mm: f64,
    /// The first tool number given to the holes the program adds.
    pub alignment_tool: u8,
    pub mirror: Option<Axis>,
    /// Quarter turns counterclockwise before anything else, 0 to 3.
    pub rotate_quarters: u8,
    pub panel_cols: usize,
    pub panel_rows: usize,
    pub panel_gap_mm: f64,
    pub rule_min_width_mm: f64,
    pub rule_min_clearance_mm: f64,
    pub rule_annular_ring_mm: f64,
    pub rule_holes_on_copper: bool,
    /// The unit of a length with no unit written (a file and the command line); a length may carry `mm`, `mil` or `in`.
    pub input_units: Unit,
    /// The unit of the G-code: mm or inch.
    pub output_units: Unit,
    /// The unit of the lengths the command line prints.
    pub report_units: Unit,
    /// Lay a path along the gaps the finest tool does not fit in (off: they are only reported).
    pub forced_isolation: bool,
    /// The narrowest gap that gets a forced path.
    pub forced_min_gap_mm: f64,
    /// Points that lie on a line within this are one move; 0 leaves the paths as they are (0.01 by default).
    pub optimize_tolerance_mm: f64,
    /// Spikes whose shorter side is at most this are taken out; 0 leaves them (0.02 by default).
    pub spike_max_mm: f64,
    pub spike_angle_deg: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    /// Reference points, relative to the centre of the board: where a mark should be and where it was measured.
    pub fiducials: Vec<Pair>,
    pub fiducial_model: Model,
    pub fiducial_max_scale_deviation: f64,
    pub fiducial_max_residual_mm: f64,
    /// How far the solder mask opens beyond a pad.
    pub mask_margin_mm: f64,
    /// The width of the text preview, in characters.
    pub preview_cols: usize,
    /// Old-style Gerber layers (no format statement): digits before and after the point, the zeros left out, the units when the file does not say, and how the list of
    /// apertures is read (its unit and which number after the D code is the size).
    pub gerber_int_digits: u8,
    pub gerber_dec_digits: u8,
    pub gerber_zeros: Zeros,
    pub gerber_units: Unit,
    pub aperture_units: Unit,
    pub aperture_size_column: usize,
    /// How much narrower than the tool a gap between two pieces of copper may be and still be accepted (the tool is taken to scrape the copper a little there); 0 accepts none.
    pub conflict_tolerance_mm: f64,
    /// Inner corners sharper than this angle are reported as problem zones; 0 does not look for them.
    pub problem_corner_angle_deg: f64,
    /// A track with a pad on each side nearer than this multiple of the tool is reported as a problem zone; 0 does not look for them.
    pub problem_track_gap_factor: f64,
    /// Bridges that hold the board in the sheet when it is cut out: how many on each closed path (0 puts none), how wide the bridge of material is, and the shortest piece of
    /// path that may be left between two bridges (a path too short for two of them gets fewer).
    pub tab_count: usize,
    pub tab_width_mm: f64,
    pub tab_min_length_mm: f64,
    /// A track narrower than twice the tolerance plus this is reported when a tolerance is set.
    pub tolerance_warning_margin_mm: f64,
    /// The tool that isolates one net chosen by a point (`--net X,Y`).
    pub net_tool_mm: f64,
    /// The tool and the direction of the parallel lines that clear an area (`rubout-area`).
    pub rubout_lines_tool_mm: f64,
    pub rubout_lines_direction: Axis,
    /// Where the conflict tolerance applies: to every gap (`all`) or only to the gaps beside a pad (`pads`).
    pub conflict_tolerance_scope: ToleranceScope,
    /// How far from the edge of the copper, beyond the radius of the tool, the lines that fill a contour stay when they are cut to the copper.
    pub rubout_fill_margin_mm: f64,
    /// The pen of each drill tool in a plot of the holes (`tool:pen, ...`); the tools it leaves out take the lowest free pens.
    pub hpgl_pen_map: Vec<(u8, u8)>,
    /// The pen speed of a plot file in cm/s; 0 leaves it to the machine.
    pub hpgl_pen_speed_cm_s: f64,
    /// Write a plot file the way the original program did (a `PA` for each point, lines of about 75 columns); off writes the compact form.
    pub hpgl_isocam_style: bool,
    /// Move a plot so that no coordinate is negative (the usual way); off keeps the coordinates of the board, negative ones included, so that a plot and a drill file share one origin.
    pub hpgl_shift_negative: bool,
    /// With `hpgl_shift_negative`, the plot is moved by the box of the board minus this margin, the same for every plot of a job, so that the isolation and the holes stay aligned; the margin covers the reach of the tool outside the board.
    pub hpgl_margin_mm: f64,
    /// Units of a plot file to the millimetre (40 by default, the resolution of the Bungard routing programs); a board must stay within 32767 units.
    pub hpgl_units_per_mm: f64,
    /// Seconds to wait after the spindle is started (`G4`) before the tool goes down; 0 writes no wait.
    pub spindle_wait_s: f64,
    /// The largest input file, in MiB, that is read; a larger one is refused before it is read into memory (`--max-bytes` on the command line wins over it).
    pub max_input_mb: f64,
    /// The thinnest tool, in mm, that a setting may name (0.02 at the least); a tool below it is refused with a message that names the setting.
    pub min_tool_mm: f64,
    /// The speed of the rapid moves in mm/min, used only to estimate the time of a job (`info`); 0 (the default) gives no estimate.
    pub rapid_mm_min: f64,
    /// The largest distance, in mm, between an arc of a layer and the polygon that is drawn for it.
    pub arc_tolerance_mm: f64,
    /// Put the cut round the outline (`--outline`) first in a plot of `isolate`.
    pub plot_outline: bool,
    /// The pen of the cut round the outline when it is in the plot; 0 takes the first pen after those of the passes.
    pub hpgl_outline_pen: u8,
    /// The pen of the isolation passes when the cut round the outline is in the plot; 0 leaves them as they are.
    pub hpgl_isolation_pen: u8,
    /// Write a drill file the way the original program did (no unit line, no tool diameters, a coordinate left out when it repeats, CR LF, M00 at the end).
    pub ncd_isocam_style: bool,
    /// Digits of a coordinate in a drill file, filled with zeros in front; 0 writes the number as it is.
    pub ncd_digits: u8,
    /// The format of the coordinates of a drill file (read and written) and of a routing file (written) that have no decimal point: `auto` (what the file says; whole mils for inches that say nothing) or `2.4` (two digits before the
    /// point and four after, read and written).
    pub excellon_format: ExcellonFormat,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            res_mm: 0.02,
            fine_tool_mm: 0.2,
            fine_passes: 1,
            fine_stepover_mm: 0.1,
            coarse_tool_mm: 0.0,
            coarse_passes: 1,
            coarse_stepover_mm: 0.3,
            trim_fine: true,
            rubout_tool_mm: 0.0,
            rubout_width_mm: 1.0,
            rubout_overlap: 0.2,
            around_pads_min_mm: 0.0,
            depth_mm: 0.06,
            feed_mm_min: 300.0,
            plunge_mm_min: 60.0,
            safe_z_mm: 2.0,
            tool_change_z_mm: 30.0,
            spindle_rpm: 24000.0,
            direction: Direction::Climb,
            start_x_mm: 0.0,
            start_y_mm: 0.0,
            loop_start: LoopStart::Nearest,
            ordering: Strategy::Optimized,
            optimize_max_paths: 300,
            route_tool_mm: 1.0,
            alignment_hole_mm: 3.0,
            alignment_margin_mm: 5.0,
            alignment_holes: 3,
            alignment_pattern: HolePattern::Normal,
            alignment_special_a_mm: 40.0,
            alignment_special_b_mm: 60.0,
            alignment_tool: 90,
            mirror: None,
            rotate_quarters: 0,
            panel_cols: 1,
            panel_rows: 1,
            panel_gap_mm: 2.0,
            rule_min_width_mm: 0.2,
            rule_min_clearance_mm: 0.2,
            rule_annular_ring_mm: 0.15,
            rule_holes_on_copper: false,
            input_units: Unit::Mm,
            output_units: Unit::Mm,
            report_units: Unit::Mm,
            forced_isolation: false,
            forced_min_gap_mm: 0.1,
            optimize_tolerance_mm: 0.01,
            spike_max_mm: 0.02,
            spike_angle_deg: 170.0,
            scale_x: 1.0,
            scale_y: 1.0,
            fiducials: Vec::new(),
            fiducial_model: Model::Auto,
            fiducial_max_scale_deviation: 0.05,
            fiducial_max_residual_mm: 0.2,
            mask_margin_mm: 0.05,
            preview_cols: 100,
            gerber_int_digits: 2,
            gerber_dec_digits: 3,
            gerber_zeros: Zeros::Leading,
            gerber_units: Unit::Inch,
            aperture_units: Unit::Mil,
            aperture_size_column: 1,
            conflict_tolerance_mm: 0.0,
            problem_corner_angle_deg: 60.0,
            problem_track_gap_factor: 1.5,
            tab_count: 0,
            tab_width_mm: 1.0,
            tab_min_length_mm: 4.0,
            tolerance_warning_margin_mm: 0.05,
            net_tool_mm: 1.0,
            rubout_lines_tool_mm: 0.8,
            rubout_lines_direction: Axis::Horizontal,
            conflict_tolerance_scope: ToleranceScope::All,
            rubout_fill_margin_mm: 0.0,
            hpgl_pen_map: Vec::new(),
            hpgl_pen_speed_cm_s: 0.0,
            hpgl_isocam_style: true,
            hpgl_shift_negative: true,
            hpgl_margin_mm: 1.0,
            hpgl_units_per_mm: 40.0,
            spindle_wait_s: 2.0,
            max_input_mb: 64.0,
            min_tool_mm: 0.02,
            rapid_mm_min: 0.0,
            arc_tolerance_mm: 0.005,
            plot_outline: false,
            hpgl_outline_pen: 0,
            hpgl_isolation_pen: 0,
            ncd_isocam_style: false,
            ncd_digits: 0,
            excellon_format: ExcellonFormat::Auto,
        }
    }
}

/// The most reference points.
pub const MAX_FIDUCIALS: usize = 50;

/// A pen number: 0 (take the usual one) or from 1 to the most pens of a plot.
fn pen(key: &str, v: &str) -> Result<u8, String> {
    let n = num(key, v)?;
    if n.fract() != 0.0 || !(0.0..=crate::hpgl::MAX_PENS as f64).contains(&n) {
        return Err(format!(
            "{key} '{v}': it must be 0 or a pen from 1 to {}",
            crate::hpgl::MAX_PENS
        ));
    }
    Ok(n as u8)
}

fn num(key: &str, v: &str) -> Result<f64, String> {
    v.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("{key} '{v}' is not a number"))
}

fn count(key: &str, v: &str) -> Result<usize, String> {
    v.trim()
        .parse::<usize>()
        .map_err(|_| format!("{key} '{v}' is not a whole number"))
}

fn flag(key: &str, v: &str) -> Result<bool, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(format!("{key} '{v}' must be yes or no")),
    }
}

impl Settings {
    /// Sets one value by its key (dashes and underscores are the same). The old short names of the command line are accepted too.
    pub fn set(&mut self, key: &str, v: &str) -> Result<(), String> {
        let key = key.trim().trim_start_matches('-').replace('-', "_");
        let key = match key.as_str() {
            "tool" => "fine_tool_mm",
            "passes" => "fine_passes",
            "stepover" => "fine_stepover_mm",
            "res" => "res_mm",
            "depth" => "depth_mm",
            "feed" => "feed_mm_min",
            "plunge" => "plunge_mm_min",
            "safe" => "safe_z_mm",
            "rpm" => "spindle_rpm",
            other => other,
        };
        // a length may carry its unit; it is kept in mm from here on
        let converted;
        let v = if key.ends_with("_mm") {
            converted = parse_length(v, self.input_units)
                .map_err(|e| format!("{key}: {e}"))?
                .to_string();
            converted.as_str()
        } else {
            v
        };
        match key {
            "res_mm" => self.res_mm = num(key, v)?,
            "fine_tool_mm" => self.fine_tool_mm = num(key, v)?,
            "fine_passes" => self.fine_passes = count(key, v)?,
            "fine_stepover_mm" => self.fine_stepover_mm = num(key, v)?,
            "coarse_tool_mm" => self.coarse_tool_mm = num(key, v)?,
            "coarse_passes" => self.coarse_passes = count(key, v)?,
            "coarse_stepover_mm" => self.coarse_stepover_mm = num(key, v)?,
            "trim_fine" => self.trim_fine = flag(key, v)?,
            "rubout_tool_mm" => self.rubout_tool_mm = num(key, v)?,
            "rubout_width_mm" => self.rubout_width_mm = num(key, v)?,
            "rubout_overlap" => self.rubout_overlap = num(key, v)?,
            "around_pads_min_mm" => self.around_pads_min_mm = num(key, v)?,
            "depth_mm" => self.depth_mm = num(key, v)?,
            "feed_mm_min" => self.feed_mm_min = num(key, v)?,
            "plunge_mm_min" => self.plunge_mm_min = num(key, v)?,
            "safe_z_mm" => self.safe_z_mm = num(key, v)?,
            "tool_change_z_mm" => self.tool_change_z_mm = num(key, v)?,
            "spindle_rpm" => self.spindle_rpm = num(key, v)?,
            "direction" => self.direction = Direction::parse(v)?,
            "start_x_mm" => self.start_x_mm = num(key, v)?,
            "start_y_mm" => self.start_y_mm = num(key, v)?,
            "loop_start" => self.loop_start = LoopStart::parse(v)?,
            "ordering" => self.ordering = Strategy::parse(v)?,
            "optimize_max_paths" => self.optimize_max_paths = count(key, v)?,
            "route_tool_mm" => self.route_tool_mm = num(key, v)?,
            "alignment_hole_mm" => self.alignment_hole_mm = num(key, v)?,
            "alignment_margin_mm" => self.alignment_margin_mm = num(key, v)?,
            "alignment_holes" => self.alignment_holes = count(key, v)?,
            "alignment_pattern" => self.alignment_pattern = HolePattern::parse(v)?,
            "alignment_special_a_mm" => self.alignment_special_a_mm = num(key, v)?,
            "alignment_special_b_mm" => self.alignment_special_b_mm = num(key, v)?,
            "alignment_tool" => {
                self.alignment_tool = u8::try_from(count(key, v)?)
                    .map_err(|_| format!("{key} '{v}' must be from 1 to 255"))?
            }
            "mirror" => self.mirror = Axis::parse_option(v)?,
            "rotate_quarters" => {
                self.rotate_quarters = u8::try_from(count(key, v)?)
                    .ok()
                    .filter(|q| *q <= 3)
                    .ok_or_else(|| format!("{key} '{v}' must be from 0 to 3"))?
            }
            "panel_cols" => self.panel_cols = count(key, v)?,
            "panel_rows" => self.panel_rows = count(key, v)?,
            "panel_gap_mm" => self.panel_gap_mm = num(key, v)?,
            "rule_min_width_mm" => self.rule_min_width_mm = num(key, v)?,
            "rule_min_clearance_mm" => self.rule_min_clearance_mm = num(key, v)?,
            "rule_annular_ring_mm" => self.rule_annular_ring_mm = num(key, v)?,
            "rule_holes_on_copper" => self.rule_holes_on_copper = flag(key, v)?,
            "input_units" => self.input_units = Unit::parse(v)?,
            "gerber_format" => {
                let (i, d) = Format::parse_digits(v).map_err(|e| format!("{key}: {e}"))?;
                self.gerber_int_digits = i;
                self.gerber_dec_digits = d;
            }
            "gerber_zeros" => self.gerber_zeros = Zeros::parse(v)?,
            "gerber_units" => {
                self.gerber_units = Unit::parse(v)?;
                if self.gerber_units == Unit::Mil {
                    return Err("gerber_units mil: a Gerber layer is in mm or inch".into());
                }
            }
            "aperture_units" => self.aperture_units = Unit::parse(v)?,
            "aperture_size_column" => self.aperture_size_column = count(key, v)?,
            "output_units" => {
                self.output_units = Unit::parse(v)?;
                if self.output_units == Unit::Mil {
                    return Err("output_units mil: G-code is written in mm or inch".into());
                }
            }
            "report_units" => self.report_units = Unit::parse(v)?,
            "forced_isolation" => self.forced_isolation = flag(key, v)?,
            "forced_min_gap_mm" => self.forced_min_gap_mm = num(key, v)?,
            "optimize_tolerance_mm" => self.optimize_tolerance_mm = num(key, v)?,
            "spike_max_mm" => self.spike_max_mm = num(key, v)?,
            "spike_angle_deg" => self.spike_angle_deg = num(key, v)?,
            "scale_x" => self.scale_x = num(key, v)?,
            "scale_y" => self.scale_y = num(key, v)?,
            "scale" => {
                self.scale_x = num(key, v)?;
                self.scale_y = self.scale_x;
            }
            "fiducial" => {
                if self.fiducials.len() >= MAX_FIDUCIALS {
                    return Err(format!("at most {MAX_FIDUCIALS} reference points"));
                }
                self.fiducials
                    .push(crate::fiducial::parse_pair(v, self.input_units)?)
            }
            "fiducial_model" => self.fiducial_model = Model::parse(v)?,
            "fiducial_max_scale_deviation" => self.fiducial_max_scale_deviation = num(key, v)?,
            "fiducial_max_residual_mm" => self.fiducial_max_residual_mm = num(key, v)?,
            "mask_margin_mm" => self.mask_margin_mm = num(key, v)?,
            "preview_cols" => self.preview_cols = count(key, v)?,
            "conflict_tolerance_mm" => self.conflict_tolerance_mm = num(key, v)?,
            "problem_corner_angle_deg" => self.problem_corner_angle_deg = num(key, v)?,
            "problem_track_gap_factor" => self.problem_track_gap_factor = num(key, v)?,
            "tab_count" => self.tab_count = count(key, v)?,
            "tab_width_mm" => self.tab_width_mm = num(key, v)?,
            "tab_min_length_mm" => self.tab_min_length_mm = num(key, v)?,
            "tolerance_warning_margin_mm" => self.tolerance_warning_margin_mm = num(key, v)?,
            "net_tool_mm" => self.net_tool_mm = num(key, v)?,
            "rubout_lines_tool_mm" => self.rubout_lines_tool_mm = num(key, v)?,
            "rubout_lines_direction" => {
                self.rubout_lines_direction = Axis::parse_option(v)?
                    .ok_or_else(|| format!("{key} '{v}' must be horizontal or vertical"))?
            }
            "conflict_tolerance_scope" => self.conflict_tolerance_scope = ToleranceScope::parse(v)?,
            "rubout_fill_margin_mm" => self.rubout_fill_margin_mm = num(key, v)?,
            "hpgl_pen_map" => {
                self.hpgl_pen_map =
                    crate::hpgl::parse_pen_map(v).map_err(|e| format!("{key}: {e}"))?
            }
            "hpgl_pen_speed_cm_s" => self.hpgl_pen_speed_cm_s = num(key, v)?,
            "hpgl_isocam_style" => self.hpgl_isocam_style = flag(key, v)?,
            "hpgl_shift_negative" => self.hpgl_shift_negative = flag(key, v)?,
            "hpgl_margin_mm" => self.hpgl_margin_mm = num(key, v)?,
            "spindle_wait_s" => self.spindle_wait_s = num(key, v)?,
            "max_input_mb" => self.max_input_mb = num(key, v)?,
            "hpgl_units_per_mm" => self.hpgl_units_per_mm = num(key, v)?,
            "min_tool_mm" => self.min_tool_mm = num(key, v)?,
            "rapid_mm_min" => self.rapid_mm_min = num(key, v)?,
            "arc_tolerance_mm" => self.arc_tolerance_mm = num(key, v)?,
            "plot_outline" => self.plot_outline = flag(key, v)?,
            "hpgl_outline_pen" => self.hpgl_outline_pen = pen(key, v)?,
            "hpgl_isolation_pen" => self.hpgl_isolation_pen = pen(key, v)?,
            "ncd_isocam_style" => self.ncd_isocam_style = flag(key, v)?,
            "ncd_digits" => {
                let n = num(key, v)?;
                if n.fract() != 0.0 || !(0.0..=8.0).contains(&n) {
                    return Err(format!(
                        "{key} '{v}': it must be a whole number from 0 to 8"
                    ));
                }
                self.ncd_digits = n as u8;
            }
            "excellon_format" => self.excellon_format = ExcellonFormat::parse(v)?,
            other => return Err(format!("{other}: unknown setting")),
        }
        Ok(())
    }

    /// Reads a configuration file; an error says the line.
    pub fn parse(text: &str) -> Result<Settings, String> {
        let mut s = Settings::default();
        // the unit of the bare numbers counts wherever it is written in the file
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "input_units" {
                    s.set(k, v).map_err(|e| format!("line {}: {e}", n + 1))?;
                }
            }
        }
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: '{line}' is not key = value", n + 1))?;
            if k.trim() == "input_units" {
                continue;
            }
            s.set(k, v).map_err(|e| format!("line {}: {e}", n + 1))?;
        }
        s.validate()?;
        Ok(s)
    }

    /// The settings as the text of a configuration file (what `parse` reads back).
    pub fn to_text(&self) -> String {
        let b = |v: bool| if v { "yes" } else { "no" };
        let mut extra = format!(
            "input_units = mm\noutput_units = {}\nreport_units = {}\nforced_isolation = {}\nforced_min_gap_mm = {}\noptimize_tolerance_mm = {}\nspike_max_mm = {}\nspike_angle_deg = {}\nscale_x = {}\nscale_y = {}\nfiducial_model = {}\nfiducial_max_scale_deviation = {}\nfiducial_max_residual_mm = {}\nmask_margin_mm = {}\npreview_cols = {}\ngerber_format = {}.{}\ngerber_zeros = {}\ngerber_units = {}\naperture_units = {}\naperture_size_column = {}\nconflict_tolerance_mm = {}\nproblem_corner_angle_deg = {}\nproblem_track_gap_factor = {}\nalignment_pattern = {}\nalignment_special_a_mm = {}\nalignment_special_b_mm = {}\ntab_count = {}\ntab_width_mm = {}\ntab_min_length_mm = {}\ntolerance_warning_margin_mm = {}\nnet_tool_mm = {}\nrubout_lines_tool_mm = {}\nrubout_lines_direction = {}\n",
            self.output_units.name(),
            self.report_units.name(),
            b(self.forced_isolation),
            self.forced_min_gap_mm,
            self.optimize_tolerance_mm,
            self.spike_max_mm,
            self.spike_angle_deg,
            self.scale_x,
            self.scale_y,
            self.fiducial_model.name(),
            self.fiducial_max_scale_deviation,
            self.fiducial_max_residual_mm,
            self.mask_margin_mm,
            self.preview_cols,
            self.gerber_int_digits,
            self.gerber_dec_digits,
            self.gerber_zeros.name(),
            self.gerber_units.name(),
            self.aperture_units.name(),
            self.aperture_size_column,
            self.conflict_tolerance_mm,
            self.problem_corner_angle_deg,
            self.problem_track_gap_factor,
            self.alignment_pattern.name(),
            self.alignment_special_a_mm,
            self.alignment_special_b_mm,
            self.tab_count,
            self.tab_width_mm,
            self.tab_min_length_mm,
            self.tolerance_warning_margin_mm,
            self.net_tool_mm,
            self.rubout_lines_tool_mm,
            self.rubout_lines_direction.name()
        );
        extra.push_str(&format!(
            "conflict_tolerance_scope = {}
rubout_fill_margin_mm = {}
hpgl_pen_map = {}
hpgl_pen_speed_cm_s = {}
hpgl_isocam_style = {}
hpgl_shift_negative = {}
plot_outline = {}
hpgl_outline_pen = {}
hpgl_isolation_pen = {}
ncd_isocam_style = {}
ncd_digits = {}
hpgl_margin_mm = {}
hpgl_units_per_mm = {}
spindle_wait_s = {}
max_input_mb = {}
min_tool_mm = {}
rapid_mm_min = {}
arc_tolerance_mm = {}
excellon_format = {}
",
            self.conflict_tolerance_scope.name(),
            self.rubout_fill_margin_mm,
            crate::hpgl::pen_map_text(&self.hpgl_pen_map),
            self.hpgl_pen_speed_cm_s,
            self.hpgl_isocam_style,
            self.hpgl_shift_negative,
            self.plot_outline,
            self.hpgl_outline_pen,
            self.hpgl_isolation_pen,
            self.ncd_isocam_style,
            self.ncd_digits,
            self.hpgl_margin_mm,
            self.hpgl_units_per_mm,
            self.spindle_wait_s,
            self.max_input_mb,
            self.min_tool_mm,
            self.rapid_mm_min,
            self.arc_tolerance_mm,
            self.excellon_format.name()
        ));
        for p in &self.fiducials {
            extra.push_str(&format!(
                "fiducial = {}, {}, {}, {}\n",
                p.design.0, p.design.1, p.measured.0, p.measured.1
            ));
        }
        let main = format!(
            "# EH3D-ISOCAM settings\nres_mm = {}\nfine_tool_mm = {}\nfine_passes = {}\nfine_stepover_mm = {}\ncoarse_tool_mm = {}\ncoarse_passes = {}\ncoarse_stepover_mm = {}\ntrim_fine = {}\nrubout_tool_mm = {}\nrubout_width_mm = {}\nrubout_overlap = {}\naround_pads_min_mm = {}\ndepth_mm = {}\nfeed_mm_min = {}\nplunge_mm_min = {}\nsafe_z_mm = {}\ntool_change_z_mm = {}\nspindle_rpm = {}\ndirection = {}\nstart_x_mm = {}\nstart_y_mm = {}\nloop_start = {}\nordering = {}\noptimize_max_paths = {}\nroute_tool_mm = {}\nalignment_hole_mm = {}\nalignment_margin_mm = {}\nalignment_holes = {}\nalignment_tool = {}\nmirror = {}\nrotate_quarters = {}\npanel_cols = {}\npanel_rows = {}\npanel_gap_mm = {}\nrule_min_width_mm = {}\nrule_min_clearance_mm = {}\nrule_annular_ring_mm = {}\nrule_holes_on_copper = {}\n",
            self.res_mm,
            self.fine_tool_mm,
            self.fine_passes,
            self.fine_stepover_mm,
            self.coarse_tool_mm,
            self.coarse_passes,
            self.coarse_stepover_mm,
            b(self.trim_fine),
            self.rubout_tool_mm,
            self.rubout_width_mm,
            self.rubout_overlap,
            self.around_pads_min_mm,
            self.depth_mm,
            self.feed_mm_min,
            self.plunge_mm_min,
            self.safe_z_mm,
            self.tool_change_z_mm,
            self.spindle_rpm,
            self.direction.name(),
            self.start_x_mm,
            self.start_y_mm,
            self.loop_start.name(),
            self.ordering.name(),
            self.optimize_max_paths,
            self.route_tool_mm,
            self.alignment_hole_mm,
            self.alignment_margin_mm,
            self.alignment_holes,
            self.alignment_tool,
            self.mirror.map_or("none", Axis::name),
            self.rotate_quarters,
            self.panel_cols,
            self.panel_rows,
            self.panel_gap_mm,
            self.rule_min_width_mm,
            self.rule_min_clearance_mm,
            self.rule_annular_ring_mm,
            b(self.rule_holes_on_copper),
        );
        main + &extra
    }

    /// The text of a Gerber layer ready for the reader: a layer with a format statement is returned as it is; an old-style one (no format statement) is rewritten with the
    /// format and units of the settings and the apertures of the list, which must then be given.
    pub fn gerber_text(&self, layer: &str, apertures: Option<&str>) -> Result<String, String> {
        if legacy::has_format_statement(layer) {
            return Ok(layer.to_string());
        }
        let list = apertures.ok_or_else(|| {
            let hint = legacy::guess_format_and_units(layer)
                .map(|(f, u)| {
                    format!(
                        " (the coordinates look like {}.{} in {}, a hint only)",
                        f.int,
                        f.dec,
                        u.name()
                    )
                })
                .unwrap_or_default();
            format!("the layer has no format statement (old-style Gerber){hint}: give its list of apertures with --apertures FILE and, if it is not 2.3 with the leading zeros left out, --gerber-format and --gerber-zeros")
        })?;
        let aps = legacy::parse_apertures(list, self.aperture_units, self.aperture_size_column)?;
        let fmt = Format {
            int: self.gerber_int_digits,
            dec: self.gerber_dec_digits,
            zeros: self.gerber_zeros,
        };
        legacy::to_extended(layer, fmt, self.gerber_units, &aps)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.plan()?.validate()?;
        self.job().validate()?;
        self.rules().validate()?;
        self.problem_rules().validate()?;
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("route_tool_mm", self.route_tool_mm, 0.1, 10.0)?;
        range(
            "conflict_tolerance_mm",
            self.conflict_tolerance_mm,
            0.0,
            0.5,
        )?;
        range("min_tool_mm", self.min_tool_mm, 0.02, 10.0)?;
        // a rate of 0 gives no estimate of the time; anything else must be a believable speed
        if self.rapid_mm_min != 0.0 {
            range("rapid_mm_min", self.rapid_mm_min, 100.0, 100_000.0)?;
        }
        // the tools that are on (0 switches the coarse tool and the removal of the leftover copper off) may not be thinner than the limit of the file
        for (name, v, off_at_zero) in [
            ("fine_tool_mm", self.fine_tool_mm, false),
            ("coarse_tool_mm", self.coarse_tool_mm, true),
            ("rubout_tool_mm", self.rubout_tool_mm, true),
            ("net_tool_mm", self.net_tool_mm, false),
            ("rubout_lines_tool_mm", self.rubout_lines_tool_mm, false),
        ] {
            if v < self.min_tool_mm && !(off_at_zero && v == 0.0) {
                return Err(format!(
                    "{name} is {v}: it is thinner than min_tool_mm ({}), the thinnest tool this file allows",
                    self.min_tool_mm
                ));
            }
        }
        if self.conflict_tolerance_mm > self.fine_tool_mm - self.min_tool_mm {
            return Err(format!(
                "conflict_tolerance_mm is {}: it must leave a tool of at least {} mm (the finest tool is {} mm)",
                self.conflict_tolerance_mm, self.min_tool_mm, self.fine_tool_mm
            ));
        }
        range("alignment_hole_mm", self.alignment_hole_mm, 0.3, 10.0)?;
        range("alignment_margin_mm", self.alignment_margin_mm, 0.5, 100.0)?;
        range("panel_gap_mm", self.panel_gap_mm, 0.0, 100.0)?;
        range("start_x_mm", self.start_x_mm, -10_000.0, 10_000.0)?;
        range("start_y_mm", self.start_y_mm, -10_000.0, 10_000.0)?;
        if self.ncd_isocam_style && self.excellon_format != ExcellonFormat::Auto {
            return Err(format!(
                "excellon_format {}: the plain style of the Bungard programs (ncd_isocam_style) is written in whole mils",
                self.excellon_format.name()
            ));
        }
        if !matches!(self.alignment_holes, 2 | 3 | 6) {
            return Err(format!(
                "alignment_holes is {}: it must be 2, 3 or 6",
                self.alignment_holes
            ));
        }
        if self.alignment_pattern == HolePattern::Special {
            if self.alignment_holes == 3 {
                return Err("alignment_pattern is special: alignment_holes must be 2 or 6".into());
            }
            range(
                "alignment_special_a_mm",
                self.alignment_special_a_mm,
                1.0,
                1000.0,
            )?;
            range(
                "alignment_special_b_mm",
                self.alignment_special_b_mm,
                1.0,
                1000.0,
            )?;
        }
        if self.tab_count > 0 {
            if self.tab_count > 50 {
                return Err(format!(
                    "tab_count is {}: it must be from 0 to 50",
                    self.tab_count
                ));
            }
            range("tab_width_mm", self.tab_width_mm, 0.1, 50.0)?;
            // a piece between two bridges must be longer than two tools
            range(
                "tab_min_length_mm",
                self.tab_min_length_mm,
                2.0 * self.route_tool_mm,
                10_000.0,
            )?;
        }
        range(
            "tolerance_warning_margin_mm",
            self.tolerance_warning_margin_mm,
            0.0,
            1.0,
        )?;
        range("net_tool_mm", self.net_tool_mm, 0.02, 10.0)?;
        range(
            "rubout_fill_margin_mm",
            self.rubout_fill_margin_mm,
            0.0,
            10.0,
        )?;
        if self.hpgl_pen_speed_cm_s != 0.0 {
            range("hpgl_pen_speed_cm_s", self.hpgl_pen_speed_cm_s, 0.1, 100.0)?;
        }
        range("rubout_lines_tool_mm", self.rubout_lines_tool_mm, 0.1, 10.0)?;

        if self.alignment_tool == 0 {
            return Err("alignment_tool is 0: it must be from 1 to 255".into());
        }
        if !(1..=20).contains(&self.panel_cols) || !(1..=20).contains(&self.panel_rows) {
            return Err(format!(
                "the panel is {} x {}: the columns and the rows must each be from 1 to 20",
                self.panel_cols, self.panel_rows
            ));
        }
        if self.panel_cols * self.panel_rows > 100 {
            return Err("a panel has at most 100 boards".into());
        }
        range("mask_margin_mm", self.mask_margin_mm, -1.0, 1.0)?;
        range("hpgl_margin_mm", self.hpgl_margin_mm, 0.0, 50.0)?;
        range("hpgl_units_per_mm", self.hpgl_units_per_mm, 10.0, 1000.0)?;
        range("spindle_wait_s", self.spindle_wait_s, 0.0, 60.0)?;
        range("max_input_mb", self.max_input_mb, 1.0, 4096.0)?;
        range("arc_tolerance_mm", self.arc_tolerance_mm, 0.0001, 1.0)?;
        range("scale_x", self.scale_x, 0.5, 2.0)?;
        range("scale_y", self.scale_y, 0.5, 2.0)?;
        range(
            "fiducial_max_scale_deviation",
            self.fiducial_max_scale_deviation,
            0.0,
            0.5,
        )?;
        range(
            "fiducial_max_residual_mm",
            self.fiducial_max_residual_mm,
            0.001,
            10.0,
        )?;
        if self.output_units == Unit::Mil {
            return Err("output_units is mil: G-code is written in mm or inch".into());
        }
        if !(10..=crate::preview::MAX_COLS).contains(&self.preview_cols) {
            return Err(format!(
                "preview_cols is {}: it must be from 10 to {}",
                self.preview_cols,
                crate::preview::MAX_COLS
            ));
        }
        self.optimize().validate()?;
        if let Some(f) = self.forced() {
            f.validate(&self.plan()?)?;
        }
        self.transform((0.0, 0.0))?;
        if !(1..=5000).contains(&self.optimize_max_paths) {
            return Err(format!(
                "optimize_max_paths is {}: it must be from 1 to 5000",
                self.optimize_max_paths
            ));
        }
        Ok(())
    }

    pub fn optimize(&self) -> Optimize {
        Optimize {
            tolerance_mm: self.optimize_tolerance_mm,
            spike_max_mm: self.spike_max_mm,
            spike_angle_deg: self.spike_angle_deg,
        }
    }

    /// The forced isolation, when it is switched on.
    pub fn forced(&self) -> Option<Forced> {
        self.forced_isolation.then_some(Forced {
            min_gap_mm: self.forced_min_gap_mm,
        })
    }

    /// The scale and the correction from the reference points, as one transformation, and the fit when there are reference points. Both are about the centre of the board:
    /// the scale leaves the centre where it is and the reference points are given relative to it. The transformation is in the coordinates of the work.
    pub fn transform(&self, centre: (f64, f64)) -> Result<(Affine, Option<Fit>), String> {
        if !(centre.0.is_finite() && centre.1.is_finite()) {
            return Err("the centre of the board is not a number".into());
        }
        let to_centre = Affine {
            e: -centre.0,
            f: -centre.1,
            ..Affine::identity()
        };
        let back = Affine {
            e: centre.0,
            f: centre.1,
            ..Affine::identity()
        };
        let scale = to_centre.then(&Affine::scale(self.scale_x, self.scale_y));
        if self.fiducials.is_empty() {
            return Ok((scale.then(&back), None));
        }
        let limits = Limits {
            max_scale_deviation: self.fiducial_max_scale_deviation,
            max_residual_mm: self.fiducial_max_residual_mm,
        };
        let f = fit(&self.fiducials, self.fiducial_model, &limits)?;
        Ok((scale.then(&f.affine).then(&back), Some(f)))
    }

    /// The tools and the passes of the isolation.
    /// The smallest pad (its smaller side) the wider tools cut round, or `None` when every tool cuts everywhere.
    pub fn around_pads(&self) -> Option<f64> {
        (self.around_pads_min_mm > 0.0).then_some(self.around_pads_min_mm)
    }

    pub fn plan(&self) -> Result<Plan, String> {
        let mut tools = vec![Tool {
            diameter_mm: self.fine_tool_mm,
            passes: self.fine_passes,
            stepover_mm: self.fine_stepover_mm,
        }];
        if self.coarse_tool_mm > 0.0 {
            tools.push(Tool {
                diameter_mm: self.coarse_tool_mm,
                passes: self.coarse_passes,
                stepover_mm: self.coarse_stepover_mm,
            });
        }
        Ok(Plan {
            tools,
            rubout: (self.rubout_tool_mm > 0.0).then_some(Rubout {
                tool_mm: self.rubout_tool_mm,
                width_mm: self.rubout_width_mm,
                overlap: self.rubout_overlap,
            }),
            res_mm: self.res_mm,
            trim: self.trim_fine,
        })
    }

    pub fn job(&self) -> Job {
        Job {
            safe_z_mm: self.safe_z_mm,
            tool_change_z_mm: self.tool_change_z_mm,
            depth_mm: self.depth_mm,
            feed_mm_min: self.feed_mm_min,
            plunge_mm_min: self.plunge_mm_min,
            spindle_rpm: self.spindle_rpm,
            spindle_wait_s: self.spindle_wait_s,
        }
    }

    pub fn order(&self) -> Order {
        Order {
            direction: self.direction,
            start: (self.start_x_mm, self.start_y_mm),
            loop_start: self.loop_start,
            strategy: self.ordering,
            optimize_max_paths: self.optimize_max_paths,
        }
    }

    pub fn layout(&self) -> Layout {
        Layout {
            quarters: self.rotate_quarters,
            mirror: self.mirror,
            cols: self.panel_cols,
            rows: self.panel_rows,
            gap_mm: self.panel_gap_mm,
        }
    }

    pub fn problem_rules(&self) -> ProblemRules {
        ProblemRules {
            tool_mm: self.fine_tool_mm,
            corner_angle_deg: self.problem_corner_angle_deg,
            track_gap_factor: self.problem_track_gap_factor,
        }
    }

    /// The width below which a gap between two pieces of copper counts as one the tool does not fit in: the tool less the tolerance.
    pub fn conflict_gap_mm(&self, finest_mm: f64) -> f64 {
        (finest_mm - self.conflict_tolerance_mm).max(self.min_tool_mm)
    }

    pub fn rules(&self) -> Rules {
        Rules {
            min_width_mm: self.rule_min_width_mm,
            min_clearance_mm: self.rule_min_clearance_mm,
            annular_ring_mm: self.rule_annular_ring_mm,
            tool_mm: self.fine_tool_mm,
            holes_on_copper: self.rule_holes_on_copper,
        }
    }
}
