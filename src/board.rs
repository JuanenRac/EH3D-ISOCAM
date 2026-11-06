//! The steps that the command line and the window share, so that both do the same thing: reading a drill file or an outline with the checks of the kind of file, laying the board out,
//! the plots, the drill files, the cut round the board and the openings of the solder mask as text, and the guard that keeps an output from being written over an input.

use crate::config::Settings;
use crate::contour::Loop;
use crate::drill::{
    add_tabs, read_drills_with_format, read_outline, route_outline, write_drills_in_format,
    write_route_excellon_in_format, DrillStyle, Hole, Outline, TabReport, Tabs,
};
use crate::fiducial::{apply_to_holes, apply_to_passes, Fit};
use crate::filetype;
use crate::hpgl::{
    isocam_style, set_pen_speed, write_holes_scaled, write_opts, PenBy, PlotOptions,
};
use crate::isolate::Pass;
use crate::mask;
use crate::order::{arrange, Cut};
use crate::transform::{alignment_pattern, lay_out, Axis, Board};
use eh3d_gerberviewer::scene::{Bounds, Scene};
use eh3d_parser_gerber::{read_file_limited, InputLimits};
use std::fs;
use std::path::{Path, PathBuf};

/// The name of a file without its folder.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The largest file that is read, in bytes, from the settings.
pub fn input_limit(s: &Settings) -> usize {
    (s.max_input_mb * 1_048_576.0) as usize
}

/// The text of a file, read with the size limit: UTF-8 (with or without a byte order mark) as it is, anything else as Latin-1.
pub fn read_text(path: &Path, s: &Settings) -> Result<String, String> {
    let name = file_name(path);
    let limits = InputLimits {
        max_bytes: input_limit(s),
    };
    let bytes = read_file_limited(path, &limits).map_err(|e| format!("{name}: {e}"))?;
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => e.into_bytes().iter().map(|&b| char::from(b)).collect(),
    };
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

/// Whether two paths are the same file (compared by their canonical names when both exist).
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Whether the output is one of the files the work reads: writing there would destroy the input.
pub fn is_input(out: &Path, inputs: &[&Path]) -> bool {
    inputs.iter().any(|p| same_file(p, out))
}

/// The lower-case extension of a name, without the dot.
pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// A drill file that was read.
#[derive(Debug, Clone)]
pub struct DrillFile {
    pub path: PathBuf,
    pub name: String,
    pub holes: Vec<Hole>,
    /// What the reader did not understand or did not use, each with the reason.
    pub notes: Vec<String>,
}

impl DrillFile {
    /// How many different drill tools the holes use.
    pub fn tools(&self) -> usize {
        let mut t: Vec<u8> = self.holes.iter().map(|h| h.tool).collect();
        t.sort_unstable();
        t.dedup();
        t.len()
    }
}

/// Reads a drill file the way the command line does: with the size limit, refusing what is not a drill file by its text, and with the units of the person when they are given.
pub fn read_drill_file(
    path: &Path,
    s: &Settings,
    units: Option<eh3d_parser_excellon::Units>,
) -> Result<DrillFile, String> {
    let name = file_name(path);
    let text = read_text(path, s)?;
    filetype::expect_drills(&name, filetype::detect(&text))?;
    let d = read_drills_with_format(&text, units, s.excellon_format)
        .map_err(|e| format!("{name}: {e}"))?;
    Ok(DrillFile {
        path: path.to_path_buf(),
        name,
        holes: d.holes,
        notes: d.notes,
    })
}

/// A board outline that was read.
#[derive(Debug, Clone)]
pub struct OutlineFile {
    pub path: PathBuf,
    pub name: String,
    pub closed: Vec<Loop>,
    /// The runs that do not close; they are left out.
    pub open_runs: usize,
}

/// Reads the outline of a board from a Gerber outline layer, a DXF drawing or a plot file (by the extension of the name).
pub fn read_outline_file(path: &Path, s: &Settings) -> Result<OutlineFile, String> {
    let name = file_name(path);
    let text = read_text(path, s)?;
    let o = read_outline(&name, &text).map_err(|e| format!("{name}: {e}"))?;
    Ok(OutlineFile {
        path: path.to_path_buf(),
        name,
        closed: o.closed,
        open_runs: o.open.len(),
    })
}

/// Lays the copper, the holes and the outline out together as the settings say (turned, turned over, repeated on a panel). Returns the board and the box of the whole panel.
pub fn lay(
    scene: &Scene,
    holes: &[Hole],
    outline: &[Loop],
    s: &Settings,
) -> Result<(Board, Bounds), String> {
    let board = Board {
        scene: scene.clone(),
        holes: holes.to_vec(),
        outline: outline.to_vec(),
    };
    lay_out(&board, &s.layout())
}

/// The centre of the box of the whole board (or panel); the origin when it is empty.
pub fn centre_of(b: &Bounds) -> (f64, f64) {
    if b.is_empty() {
        (0.0, 0.0)
    } else {
        ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0)
    }
}

/// The rows of a table of reference points: a row that starts with a number (a header row, an empty line and a line that starts with `#` are left out).
pub fn mark_rows(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            let first = line
                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .find(|t| !t.is_empty());
            let numeric = first.is_some_and(|t| {
                t.trim_start_matches(['+', '-', '.'])
                    .starts_with(|c: char| c.is_ascii_digit())
            });
            !line.is_empty() && !line.starts_with('#') && numeric
        })
        .collect()
}

/// The passes with the scale and the correction from the reference points applied (the fit is returned when there are reference points).
pub fn move_passes(
    passes: &[Pass],
    s: &Settings,
    whole: &Bounds,
) -> Result<(Vec<Pass>, Option<Fit>), String> {
    let (t, fit) = s.transform(centre_of(whole))?;
    let mut moved = passes.to_vec();
    if !t.is_identity() {
        apply_to_passes(&mut moved, &t)?;
    }
    Ok((moved, fit))
}

/// The smallest X and Y of a job for the plots: the box of the board less the margin, the same for the isolation and for the holes of one board.
pub fn plot_origin(s: &Settings, whole: &Bounds) -> Option<(f64, f64)> {
    let m = s.hpgl_margin_mm;
    (whole.min_x.is_finite() && whole.min_y.is_finite())
        .then_some((whole.min_x - m, whole.min_y - m))
}

/// How a plot is written, from the settings.
pub fn plot_options(s: &Settings, whole: &Bounds) -> PlotOptions {
    PlotOptions {
        keep_negative: !s.hpgl_shift_negative,
        pen_of_pass: Vec::new(),
        origin: plot_origin(s, whole),
        units_per_mm: s.hpgl_units_per_mm,
    }
}

/// A plot file with the pen speed and the style of the settings.
pub fn finish_plot(s: &Settings, text: Result<String, String>) -> Result<String, String> {
    let text = set_pen_speed(&text?, s.hpgl_pen_speed_cm_s)?;
    Ok(if s.hpgl_isocam_style {
        isocam_style(&text)
    } else {
        text
    })
}

/// The isolation passes (already moved by the scale and the reference points) as a plot file. With `plot_outline` the cut round the outline goes first, on its own pen.
pub fn isolation_plot(
    passes: &[Pass],
    s: &Settings,
    whole: &Bounds,
    outline: &[Loop],
) -> Result<String, String> {
    let pl = s.plan()?;
    let order = s.order();
    let pens = if pl.tools.len() > 1 || pl.rubout.is_some() {
        PenBy::Tool
    } else {
        PenBy::Pass
    };
    let mut all = passes.to_vec();
    let mut opts = plot_options(s, whole);
    if s.plot_outline {
        if outline.is_empty() {
            return Err("plot_outline needs the outline of the board: give --outline FILE".into());
        }
        let o = Outline {
            closed: outline.to_vec(),
            open: Vec::new(),
        };
        let mut cut = route_outline(&o, s.route_tool_mm, s.res_mm.min(s.route_tool_mm / 2.0))?;
        let (t, _) = s.transform(centre_of(whole))?;
        if !t.is_identity() {
            let mut one = vec![cut];
            apply_to_passes(&mut one, &t)?;
            cut = one.remove(0);
        }
        // the pens of the passes as they would be, then the cut in front of them
        let usual = |k: usize, p: &Pass| match pens {
            PenBy::Pass => k + 1,
            PenBy::Tool => p.tool + 1,
        };
        let mut pen_of: Vec<usize> = all
            .iter()
            .enumerate()
            .map(|(k, p)| match s.hpgl_isolation_pen {
                0 => usual(k, p),
                n => usize::from(n),
            })
            .collect();
        let outline_pen = match s.hpgl_outline_pen {
            0 => pen_of.iter().copied().max().unwrap_or(0) + 1,
            n => usize::from(n),
        };
        pen_of.insert(0, outline_pen);
        all.insert(0, cut);
        opts.pen_of_pass = pen_of;
    }
    finish_plot(s, write_opts(&all, &order, pens, &opts))
}

/// Adds the holes that hold the board when it is turned over. When the tool number of the settings is already used for another diameter, the next free number is taken.
pub fn add_alignment(holes: &mut Vec<Hole>, whole: &Bounds, s: &Settings) -> Result<(), String> {
    let clash = holes
        .iter()
        .any(|h| h.tool == s.alignment_tool && (h.diameter_mm - s.alignment_hole_mm).abs() > 0.005);
    let tool = if clash {
        holes
            .iter()
            .map(|h| h.tool)
            .max()
            .and_then(|m| m.checked_add(1))
            .ok_or("no free tool number for the alignment holes")?
    } else {
        s.alignment_tool
    };
    holes.extend(alignment_pattern(
        whole,
        s.alignment_hole_mm,
        s.alignment_margin_mm,
        s.alignment_holes,
        s.mirror.unwrap_or(Axis::Vertical),
        tool,
        s.alignment_pattern,
        (s.alignment_special_a_mm, s.alignment_special_b_mm),
    )?);
    Ok(())
}

/// A drill file or a plot of holes, and how many holes it has.
#[derive(Debug, Clone)]
pub struct DrillJob {
    pub text: String,
    pub holes: usize,
}

/// The holes (already laid out) as a drill file, or as a plot with a pen for each drill. With `align` the holes that hold the board are added first; the scale and the correction
/// from the reference points are applied.
pub fn drills_job(
    mut holes: Vec<Hole>,
    whole: &Bounds,
    s: &Settings,
    align: bool,
    plot: bool,
) -> Result<DrillJob, String> {
    if align {
        add_alignment(&mut holes, whole, s)?;
    }
    let (t, _) = s.transform(centre_of(whole))?;
    if !t.is_identity() {
        apply_to_holes(&mut holes, &t)?;
    }
    let text = if plot {
        finish_plot(
            s,
            write_holes_scaled(
                &holes,
                &s.hpgl_pen_map,
                !s.hpgl_shift_negative,
                plot_origin(s, whole),
                s.hpgl_units_per_mm,
            ),
        )?
    } else {
        write_drills_in_format(
            &holes,
            &DrillStyle {
                plain: s.ncd_isocam_style,
                digits: s.ncd_digits,
            },
            s.excellon_format,
        )?
    };
    Ok(DrillJob {
        text,
        holes: holes.len(),
    })
}

/// The cut round the board as a file, with how long it is and what the bridges did.
#[derive(Debug, Clone)]
pub struct RouteJob {
    pub text: String,
    pub paths: usize,
    pub cut_mm: f64,
    pub tabs: TabReport,
}

/// The path of the tool round the closed paths of the outline (already laid out), with bridges when the settings ask for them, as a plot file or a routing Excellon file.
pub fn route_job(
    closed: &[Loop],
    whole: &Bounds,
    s: &Settings,
    plot: bool,
) -> Result<RouteJob, String> {
    let outline = Outline {
        closed: closed.to_vec(),
        open: Vec::new(),
    };
    let mut pass = route_outline(
        &outline,
        s.route_tool_mm,
        s.res_mm.min(s.route_tool_mm / 2.0),
    )?;
    let tabs = if s.tab_count > 0 {
        add_tabs(
            &mut pass,
            s.route_tool_mm,
            &Tabs {
                count: s.tab_count,
                width_mm: s.tab_width_mm,
                min_length_mm: s.tab_min_length_mm,
            },
        )?
    } else {
        TabReport::default()
    };
    let mut passes = vec![pass];
    let (t, _) = s.transform(centre_of(whole))?;
    if !t.is_identity() {
        apply_to_passes(&mut passes, &t)?;
    }
    let pass = passes.remove(0);
    let order = s.order();
    let text = if plot {
        finish_plot(
            s,
            write_opts(
                std::slice::from_ref(&pass),
                &order,
                PenBy::Pass,
                &plot_options(s, whole),
            ),
        )?
    } else {
        let (cuts, _) = arrange(&pass.cuts(), &order, order.start);
        write_route_excellon_in_format(&cuts, s.route_tool_mm, 1, s.excellon_format)?
    };
    let cuts = pass.cuts();
    Ok(RouteJob {
        text,
        paths: cuts.len(),
        cut_mm: cuts.iter().map(Cut::length_mm).sum(),
        tabs,
    })
}

/// The openings of the solder mask as a Gerber file.
#[derive(Debug, Clone)]
pub struct MaskJob {
    pub text: String,
    pub openings: usize,
    pub notes: Vec<String>,
}

/// The openings of the solder mask, from the mask layer when there is one or from the pads of the copper, `margin_mm` beyond each pad.
pub fn mask_job(
    copper: &Scene,
    mask_layer: Option<&Scene>,
    margin_mm: f64,
) -> Result<MaskJob, String> {
    let open = mask::openings(copper, mask_layer, margin_mm)?;
    Ok(MaskJob {
        text: mask::write_gerber(&open)?,
        openings: open.shapes.len(),
        notes: open.notes.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GKO: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";
    const DRILLS: &str =
        "M48\nMETRIC,TZ\nT01C0.8\nT02C3.2\n%\nT01\nX1.0Y2.0\nX3.0Y2.0\nT02\nX5.0Y5.0\nM30\n";

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eh3d-isocam-board-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn hole(tool: u8, x: f64, y: f64, d: f64) -> Hole {
        Hole {
            tool,
            x_mm: x,
            y_mm: y,
            diameter_mm: d,
        }
    }

    fn box_outline() -> Vec<Loop> {
        vec![vec![(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]]
    }

    #[test]
    fn a_drill_file_is_read_with_its_holes_and_tools() {
        let dir = scratch("drills");
        let f = dir.join("board.drl");
        fs::write(&f, DRILLS).unwrap();
        let d = read_drill_file(&f, &Settings::default(), None).unwrap();
        assert_eq!(d.name, "board.drl");
        assert_eq!(d.holes.len(), 3);
        assert_eq!(d.tools(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_gerber_layer_is_refused_as_a_drill_file_and_a_missing_file_is_an_error() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample.gtl");
        let e = read_drill_file(&sample, &Settings::default(), None).unwrap_err();
        assert!(e.contains("not a drill file"), "{e}");
        assert!(
            read_drill_file(Path::new("/no/such/file.drl"), &Settings::default(), None).is_err()
        );
    }

    #[test]
    fn an_outline_is_read_with_its_closed_paths() {
        let dir = scratch("outline");
        let f = dir.join("board.gko");
        fs::write(&f, GKO).unwrap();
        let o = read_outline_file(&f, &Settings::default()).unwrap();
        assert_eq!((o.closed.len(), o.open_runs), (1, 0));
        let junk = dir.join("junk.gko");
        fs::write(&junk, "nothing here\n").unwrap();
        assert!(read_outline_file(&junk, &Settings::default()).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_lines_of_a_table_of_marks_skip_the_header_and_the_comments() {
        let text = "dx,dy,mx,my\n# a comment\n\n1,2,1.1,2.1\n 3;4;3.1;4.1 \n-5 6 -5.1 6.1\n";
        assert_eq!(
            mark_rows(text),
            ["1,2,1.1,2.1", "3;4;3.1;4.1", "-5 6 -5.1 6.1"]
        );
        assert!(mark_rows("").is_empty());
    }

    #[test]
    fn the_board_is_turned_over_about_the_middle_of_the_outline() {
        let mut s = Settings::default();
        s.set("mirror", "vertical").unwrap();
        let (b, whole) = lay(
            &Scene::default(),
            &[hole(1, 2.0, 5.0, 0.8)],
            &box_outline(),
            &s,
        )
        .unwrap();
        assert!((b.holes[0].x_mm - 18.0).abs() < 1e-9);
        assert!((whole.max_x - 20.0).abs() < 1e-9);
    }

    #[test]
    fn a_panel_repeats_the_holes_and_widens_the_box() {
        let mut s = Settings::default();
        s.set("panel_cols", "2").unwrap();
        let (b, whole) = lay(
            &Scene::default(),
            &[hole(1, 2.0, 5.0, 0.8)],
            &box_outline(),
            &s,
        )
        .unwrap();
        assert_eq!(b.holes.len(), 2);
        assert_eq!(b.outline.len(), 2);
        assert!((whole.max_x - 42.0).abs() < 1e-9);
        assert!(lay(&Scene::default(), &[], &[], &s).is_err());
    }

    #[test]
    fn the_centre_of_a_box_and_of_nothing() {
        let b = Bounds {
            min_x: 0.0,
            min_y: 2.0,
            max_x: 10.0,
            max_y: 4.0,
        };
        assert_eq!(centre_of(&b), (5.0, 3.0));
        assert_eq!(centre_of(&Bounds::empty()), (0.0, 0.0));
    }

    #[test]
    fn the_alignment_holes_take_a_free_tool_when_the_number_is_used() {
        let s = Settings::default();
        let whole = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 20.0,
            max_y: 10.0,
        };
        let mut holes = vec![hole(s.alignment_tool, 1.0, 1.0, 1.0)];
        add_alignment(&mut holes, &whole, &s).unwrap();
        assert_eq!(holes.len(), 1 + s.alignment_holes);
        assert!(holes[1..].iter().all(|h| h.tool == s.alignment_tool + 1));
        let mut free = vec![hole(1, 1.0, 1.0, 0.8)];
        add_alignment(&mut free, &whole, &s).unwrap();
        assert!(free[1..].iter().all(|h| h.tool == s.alignment_tool));
    }

    #[test]
    fn the_drills_are_written_as_a_file_or_a_plot_with_the_alignment_holes() {
        let s = Settings::default();
        let whole = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 20.0,
            max_y: 10.0,
        };
        let holes = vec![hole(1, 2.0, 5.0, 0.8), hole(2, 4.0, 5.0, 1.0)];
        let file = drills_job(holes.clone(), &whole, &s, false, false).unwrap();
        assert_eq!(file.holes, 2);
        assert!(file.text.starts_with("M48"));
        let plot = drills_job(holes, &whole, &s, true, true).unwrap();
        assert_eq!(plot.holes, 2 + s.alignment_holes);
        assert!(plot.text.contains("PD"));
        assert!(drills_job(Vec::new(), &whole, &s, false, false).is_err());
    }

    #[test]
    fn the_cut_round_the_board_is_a_routing_file_or_a_plot() {
        let s = Settings::default();
        let whole = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 20.0,
            max_y: 10.0,
        };
        let r = route_job(&box_outline(), &whole, &s, false).unwrap();
        assert_eq!(r.paths, 1);
        assert!(r.cut_mm > 60.0);
        assert!(r.text.contains("M15"));
        assert!(route_job(&box_outline(), &whole, &s, true)
            .unwrap()
            .text
            .contains("PD"));
        assert!(route_job(&[], &whole, &s, false).is_err());
    }

    #[test]
    fn bridges_open_the_cut_and_a_short_path_is_counted() {
        let mut s = Settings::default();
        s.set("tab_count", "2").unwrap();
        let whole = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 20.0,
            max_y: 10.0,
        };
        let r = route_job(&box_outline(), &whole, &s, false).unwrap();
        assert_eq!(r.tabs.bridges, 2);
        assert_eq!(r.tabs.paths_left_closed, 0);
    }

    #[test]
    fn the_mask_openings_come_from_the_pads_of_the_copper() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample.gtl");
        let s = Settings::default();
        let layer = crate::window::read_layer(&sample, &s).unwrap();
        let m = mask_job(&layer.scene, None, s.mask_margin_mm).unwrap();
        assert!(m.openings > 0);
        assert!(m.text.contains("D03"));
    }

    #[test]
    fn an_output_may_not_be_one_of_the_inputs() {
        let dir = scratch("same");
        let a = dir.join("a.gtl");
        let b = dir.join("b.nc");
        fs::write(&a, "x").unwrap();
        assert!(same_file(&a, &dir.join(".").join("a.gtl")));
        assert!(is_input(&a, &[b.as_path(), a.as_path()]));
        assert!(!is_input(&b, &[a.as_path()]));
        assert_eq!(extension(Path::new("OUT.PLT")), "plt");
        assert_eq!(extension(Path::new("noext")), "");
        let _ = fs::remove_dir_all(&dir);
    }
}
