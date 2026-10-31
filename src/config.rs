//! Every value that can be tuned lives in one configuration file (`key = value`, one per line, `#` starts a comment) and has a default here. The command line takes the same
//! keys as options (`--fine-tool-mm 0.15` is `fine_tool_mm = 0.15`), so nothing tuned is hidden in the code.

use crate::drc::Rules;
use crate::gcode::Job;
use crate::isolate::{Plan, Rubout, Tool};
use crate::order::{Direction, LoopStart, Order, Strategy};
use crate::transform::{Axis, Layout};

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
    pub depth_mm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub safe_z_mm: f64,
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
    pub alignment_holes: usize,
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
            depth_mm: 0.06,
            feed_mm_min: 300.0,
            plunge_mm_min: 60.0,
            safe_z_mm: 2.0,
            spindle_rpm: 24000.0,
            direction: Direction::Climb,
            start_x_mm: 0.0,
            start_y_mm: 0.0,
            loop_start: LoopStart::Nearest,
            ordering: Strategy::Nearest,
            optimize_max_paths: 300,
            route_tool_mm: 1.0,
            alignment_hole_mm: 3.0,
            alignment_margin_mm: 5.0,
            alignment_holes: 3,
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
        }
    }
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
            "depth_mm" => self.depth_mm = num(key, v)?,
            "feed_mm_min" => self.feed_mm_min = num(key, v)?,
            "plunge_mm_min" => self.plunge_mm_min = num(key, v)?,
            "safe_z_mm" => self.safe_z_mm = num(key, v)?,
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
            other => return Err(format!("{other}: unknown setting")),
        }
        Ok(())
    }

    /// Reads a configuration file; an error says the line.
    pub fn parse(text: &str) -> Result<Settings, String> {
        let mut s = Settings::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: '{line}' is not key = value", n + 1))?;
            s.set(k, v).map_err(|e| format!("line {}: {e}", n + 1))?;
        }
        s.validate()?;
        Ok(s)
    }

    /// The settings as the text of a configuration file (what `parse` reads back).
    pub fn to_text(&self) -> String {
        let b = |v: bool| if v { "yes" } else { "no" };
        format!(
            "# EH3D-ISOCAM settings\nres_mm = {}\nfine_tool_mm = {}\nfine_passes = {}\nfine_stepover_mm = {}\ncoarse_tool_mm = {}\ncoarse_passes = {}\ncoarse_stepover_mm = {}\ntrim_fine = {}\nrubout_tool_mm = {}\nrubout_width_mm = {}\nrubout_overlap = {}\ndepth_mm = {}\nfeed_mm_min = {}\nplunge_mm_min = {}\nsafe_z_mm = {}\nspindle_rpm = {}\ndirection = {}\nstart_x_mm = {}\nstart_y_mm = {}\nloop_start = {}\nordering = {}\noptimize_max_paths = {}\nroute_tool_mm = {}\nalignment_hole_mm = {}\nalignment_margin_mm = {}\nalignment_holes = {}\nalignment_tool = {}\nmirror = {}\nrotate_quarters = {}\npanel_cols = {}\npanel_rows = {}\npanel_gap_mm = {}\nrule_min_width_mm = {}\nrule_min_clearance_mm = {}\nrule_annular_ring_mm = {}\nrule_holes_on_copper = {}\n",
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
            self.depth_mm,
            self.feed_mm_min,
            self.plunge_mm_min,
            self.safe_z_mm,
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
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        self.plan()?.validate()?;
        self.job().validate()?;
        self.rules().validate()?;
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("route_tool_mm", self.route_tool_mm, 0.1, 10.0)?;
        range("alignment_hole_mm", self.alignment_hole_mm, 0.3, 10.0)?;
        range("alignment_margin_mm", self.alignment_margin_mm, 0.5, 100.0)?;
        range("panel_gap_mm", self.panel_gap_mm, 0.0, 100.0)?;
        range("start_x_mm", self.start_x_mm, -10_000.0, 10_000.0)?;
        range("start_y_mm", self.start_y_mm, -10_000.0, 10_000.0)?;
        if !(2..=3).contains(&self.alignment_holes) {
            return Err(format!(
                "alignment_holes is {}: it must be 2 or 3",
                self.alignment_holes
            ));
        }
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
        if !(1..=5000).contains(&self.optimize_max_paths) {
            return Err(format!(
                "optimize_max_paths is {}: it must be from 1 to 5000",
                self.optimize_max_paths
            ));
        }
        Ok(())
    }

    /// The tools and the passes of the isolation.
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
            depth_mm: self.depth_mm,
            feed_mm_min: self.feed_mm_min,
            plunge_mm_min: self.plunge_mm_min,
            spindle_rpm: self.spindle_rpm,
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
