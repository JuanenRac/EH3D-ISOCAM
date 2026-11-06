//! A board milled on both sides: the plot that keeps the origin of the board and carries the cut round the outline, the holes turned over about the middle of the outline,
//! and the plain form of the drill file.

use eh3d_isocam::config::Settings;
use eh3d_isocam::drill::{read_drills, write_drills_as, DrillStyle, Hole};
use eh3d_isocam::filetype::{detect, FileKind};
use eh3d_isocam::gcode::{write_with, Job};
use eh3d_isocam::hpgl::{write_holes_at, write_holes_opts, write_opts, PenBy, PlotOptions};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::Order;
use eh3d_isocam::transform::{lay_out, Axis, Board, Layout};
use std::process::Command;

fn hole(tool: u8, x_mil: f64, y_mil: f64, d: f64) -> Hole {
    Hole {
        tool,
        x_mm: x_mil * 0.0254,
        y_mm: y_mil * 0.0254,
        diameter_mm: d,
    }
}

fn pass(paths: Vec<Vec<(f64, f64)>>) -> Pass {
    Pass {
        paths,
        ..Pass::default()
    }
}

#[test]
fn a_plot_keeps_its_negative_coordinates_when_asked() {
    let p = pass(vec![vec![(-1.0, -0.5), (5.0, -0.5), (5.0, 3.0)]]);
    let order = Order::default();
    let moved = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions::default(),
    )
    .unwrap();
    assert!(!moved.contains('-'), "{moved}");
    assert!(moved.contains("PU0,0;"), "{moved}");
    let kept = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions {
            keep_negative: true,
            ..PlotOptions::default()
        },
    )
    .unwrap();
    assert!(kept.contains("-40,-20"), "{kept}");
    assert!(kept.contains("200,-20"), "{kept}");
}

#[test]
fn the_pen_of_a_pass_can_be_chosen() {
    let a = pass(vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 3.0)]]);
    let b = pass(vec![vec![(10.0, 0.0), (15.0, 0.0), (15.0, 3.0)]]);
    let text = write_opts(
        &[a, b],
        &Order::default(),
        PenBy::Pass,
        &PlotOptions {
            pen_of_pass: vec![1, 4],
            ..PlotOptions::default()
        },
    )
    .unwrap();
    let first = text.find("SP1;").unwrap();
    let second = text.find("SP4;").unwrap();
    assert!(first < second, "{text}");
    assert!(!text.contains("SP2;"), "{text}");
}

#[test]
fn the_holes_are_turned_over_about_the_middle_of_the_outline() {
    // the outline of a board 2.8625 in wide, its left edge at 0.0101 in
    let outline = vec![vec![
        (10.1 * 0.0254, 10.5 * 0.0254),
        (2872.6 * 0.0254, 10.5 * 0.0254),
        (2872.6 * 0.0254, 1972.5 * 0.0254),
        (10.1 * 0.0254, 1972.5 * 0.0254),
    ]];
    let board = Board {
        holes: vec![hole(1, 866.0, 104.0, 0.8), hole(3, 2678.0, 710.0, 2.0)],
        outline,
        ..Board::default()
    };
    let layout = Layout {
        mirror: Some(Axis::Vertical),
        ..Layout::default()
    };
    let (turned, _) = lay_out(&board, &layout).unwrap();
    let sum = (10.1 + 2872.6) * 0.0254;
    for (before, after) in board.holes.iter().zip(&turned.holes) {
        assert!((before.x_mm + after.x_mm - sum).abs() < 1e-9);
        assert!((before.y_mm - after.y_mm).abs() < 1e-9);
    }
}

#[test]
fn the_plain_drill_file_is_written_as_the_bungard_programs_write_it() {
    let holes = vec![
        hole(1, 2017.0, 104.0, 0.8),
        hole(1, 2017.0, 504.0, 0.8),
        hole(2, 1799.0, 445.0, 1.3),
        hole(2, 1799.0, 150.0, 1.3),
        hole(2, 1012.0, 150.0, 1.3),
        hole(2, 874.0, 445.0, 1.3),
    ];
    let text = write_drills_as(
        &holes,
        &DrillStyle {
            plain: true,
            digits: 5,
        },
    )
    .unwrap();
    assert_eq!(
        text,
        "M48\r\n%\r\nT1\r\nX02017Y00104\r\nY00504\r\nT2\r\nX01799Y00445\r\nY00150\r\nX01012\r\nX00874Y00445\r\nM00\r\n"
    );
    // and it reads back to the same holes (the diameters are not in the file)
    let back = read_drills(&text).unwrap();
    assert_eq!(back.holes.len(), holes.len());
    for (a, b) in holes.iter().zip(&back.holes) {
        assert_eq!(a.tool, b.tool);
        assert!((a.x_mm - b.x_mm).abs() < 0.013 && (a.y_mm - b.y_mm).abs() < 0.013);
    }
}

#[test]
fn the_usual_drill_file_can_fill_its_coordinates_with_zeros() {
    let holes = vec![hole(1, 866.0, 104.0, 0.8)];
    let text = write_drills_as(
        &holes,
        &DrillStyle {
            plain: false,
            digits: 5,
        },
    )
    .unwrap();
    assert!(
        text.contains("X00866Y00104\n") && text.contains("M30"),
        "{text}"
    );
    let plain = write_drills_as(&holes, &DrillStyle::default()).unwrap();
    assert!(plain.contains("X866Y104\n"), "{plain}");
    assert!(write_drills_as(
        &[],
        &DrillStyle {
            plain: true,
            digits: 0
        }
    )
    .is_err());
}

#[test]
fn a_drill_file_with_no_header_line_is_known() {
    let text = "%\nT01\nX866Y104\nX866Y504\nT02\nX1084Y445\nM30\n";
    assert_eq!(detect(text), FileKind::Excellon);
    let drills = read_drills(text).unwrap();
    assert_eq!(drills.holes.len(), 3);
    assert_eq!(detect("%\nnothing here\n"), FileKind::Unknown);
}

#[test]
fn the_new_settings_are_read_written_and_checked() {
    let s = Settings::parse(
        "hpgl_shift_negative = no\nplot_outline = yes\nhpgl_outline_pen = 1\nhpgl_isolation_pen = 4\nncd_isocam_style = yes\nncd_digits = 5\n",
    )
    .unwrap();
    assert!(!s.hpgl_shift_negative && s.plot_outline && s.ncd_isocam_style);
    assert_eq!(
        (s.hpgl_outline_pen, s.hpgl_isolation_pen, s.ncd_digits),
        (1, 4, 5)
    );
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    let d = Settings::default();
    assert!(d.hpgl_shift_negative && !d.plot_outline && !d.ncd_isocam_style);
    assert_eq!(
        (d.hpgl_outline_pen, d.hpgl_isolation_pen, d.ncd_digits),
        (0, 0, 0)
    );
    assert!(Settings::parse("hpgl_outline_pen = 9\n").is_err());
    assert!(Settings::parse("hpgl_isolation_pen = 1.5\n").is_err());
    assert!(Settings::parse("ncd_digits = 12\n").is_err());
    assert!(Settings::parse("ncd_digits = -1\n").is_err());
}

const LAYER: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.3*%\nD10*\nX100000Y100000D02*\nX200000Y100000D01*\nM02*\n";
const OUTLINE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D02*\nX300000Y0D01*\nX300000Y220000D01*\nX0Y220000D01*\nX0Y0D01*\nM02*\n";

#[test]
fn isolate_puts_the_cut_round_the_outline_first_and_keeps_the_origin() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let layer = format!("{dir}/two_sided_layer.gtl");
    let outline = format!("{dir}/two_sided_outline.gbr");
    let out = format!("{dir}/two_sided.plt");
    std::fs::write(&layer, LAYER).unwrap();
    std::fs::write(&outline, OUTLINE).unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec![
            "isolate",
            layer.as_str(),
            out.as_str(),
            "--outline",
            outline.as_str(),
            "--fine-tool-mm",
            "0.4",
            "--route-tool-mm",
            "2",
            "--hpgl-isocam-style",
            "no",
            "--force",
        ];
        args.extend_from_slice(extra);
        let done = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
            .args(&args)
            .output()
            .unwrap();
        (
            done.status.success(),
            String::from_utf8_lossy(&done.stderr).into_owned(),
        )
    };
    // without the setting the plot is only the isolation
    let (ok, err) = run(&[]);
    assert!(ok, "{err}");
    let plain = std::fs::read_to_string(&out).unwrap();
    assert!(plain.contains("SP1;") && !plain.contains("SP4;"), "{plain}");
    // with it: the cut first on pen 1, the isolation on pen 4, and the origin of the board
    let (ok, err) = run(&[
        "--plot-outline",
        "yes",
        "--hpgl-outline-pen",
        "1",
        "--hpgl-isolation-pen",
        "4",
        "--hpgl-shift-negative",
        "no",
    ]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&out).unwrap();
    let one = text.find("SP1;").unwrap();
    let four = text.find("SP4;").unwrap();
    assert!(one < four, "{text}");
    // the tool of 2 mm runs 1 mm outside the board: x from -1 mm (-40 units)
    assert!(text[one..four].contains("-40"), "{text}");
    assert!(!text[four..].contains('-'), "{text}");
    // the usual way moves everything to positive coordinates
    let (ok, err) = run(&["--plot-outline", "yes"]);
    assert!(ok, "{err}");
    assert!(!std::fs::read_to_string(&out).unwrap().contains('-'));
    // no outline, no cut
    let none = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args([
            "isolate",
            layer.as_str(),
            out.as_str(),
            "--plot-outline",
            "yes",
            "--force",
        ])
        .output()
        .unwrap();
    assert!(!none.status.success());
    assert!(String::from_utf8_lossy(&none.stderr).contains("plot_outline needs the outline"));
}

#[test]
fn drills_can_be_turned_over_and_written_plain_from_the_command_line() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let drd = format!("{dir}/two_sided.drd");
    let dim = format!("{dir}/two_sided_dim.gbr");
    let out = format!("{dir}/two_sided.ncd");
    std::fs::write(&drd, "%\nT01\nX866Y104\nX866Y504\nT02\nX1084Y445\nM30\n").unwrap();
    // an outline 0 to 73.22 mm wide: the middle is 36.61 mm = 1441.3 mil
    std::fs::write(
        &dim,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D02*\nX732200Y0D01*\nX732200Y500000D01*\nX0Y500000D01*\nX0Y0D01*\nM02*\n",
    )
    .unwrap();
    let done = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args([
            "drills",
            drd.as_str(),
            out.as_str(),
            "--outline",
            dim.as_str(),
            "--mirror",
            "vertical",
            "--ncd-isocam-style",
            "yes",
            "--ncd-digits",
            "5",
        ])
        .output()
        .unwrap();
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    // 2882.7 mil less 866 = 2016.7 -> 2017; less 1084 = 1798.7 -> 1799
    assert_eq!(
        text,
        "M48\r\n%\r\nT1\r\nX02017Y00104\r\nY00504\r\nT2\r\nX01799Y00445\r\nM00\r\n"
    );
}

#[test]
fn the_isolation_plot_and_the_hole_plot_of_one_job_share_one_shift() {
    // the copper starts at 0 and the tool reaches 0.1 mm beyond it: the paths start at -0.1 and the holes at 0
    let p = pass(vec![vec![(-0.1, -0.1), (5.0, -0.1), (5.0, 3.0)]]);
    let holes = vec![Hole {
        tool: 1,
        x_mm: 0.0,
        y_mm: 0.0,
        diameter_mm: 0.8,
    }];
    // each plot on its own moves by its own minimum: the hole stays at 0 and the path goes to 0, 0.1 mm apart
    let alone = write_holes_opts(&holes, &[], false).unwrap();
    assert!(alone.contains("PU0,0;PD;"), "{alone}");
    // with the origin of the job both move by the same amount
    let origin = Some((-1.0, -1.0));
    let iso = write_opts(
        std::slice::from_ref(&p),
        &Order::default(),
        PenBy::Pass,
        &PlotOptions {
            origin,
            ..PlotOptions::default()
        },
    )
    .unwrap();
    let drl = write_holes_at(&holes, &[], false, origin).unwrap();
    // 1 mm = 40 plotter units: the hole is at (40,40) and the path starts 0.1 mm (4 units) before it
    assert!(drl.contains("PU40,40;PD;"), "{drl}");
    assert!(iso.contains("PU36,36;"), "{iso}");
}

#[test]
fn the_spindle_waits_after_every_start_when_the_settings_say_so() {
    let p = pass(vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 3.0)]]);
    let none = write_with(std::slice::from_ref(&p), &Job::default(), &Order::default()).unwrap();
    assert!(!none.contains("G4"), "{none}");
    let job = Job {
        spindle_wait_s: 2.0,
        ..Job::default()
    };
    let g = write_with(std::slice::from_ref(&p), &job, &Order::default()).unwrap();
    assert!(g.contains("M3 S24000\nG4 P2\n"), "{g}");
    assert!(write_with(
        std::slice::from_ref(&p),
        &Job {
            spindle_wait_s: -1.0,
            ..Job::default()
        },
        &Order::default()
    )
    .is_err());
    let s = Settings::parse(
        "spindle_wait_s = 3
hpgl_margin_mm = 0.5
",
    )
    .unwrap();
    assert_eq!(
        (s.spindle_wait_s, s.hpgl_margin_mm, s.job().spindle_wait_s),
        (3.0, 0.5, 3.0)
    );
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    assert_eq!(Settings::default().spindle_wait_s, 2.0);
}

// ---- the thinnest tool and the estimate of the time of a job --------------------------------------------------------------------------------------------

#[test]
fn the_time_of_a_job_is_worked_out_from_the_speeds() {
    use eh3d_isocam::gcode::estimate_time;
    // 600 mm of cut at 300 mm/min = 120 s; 10 runs down 2 + 0.06 mm at 60 mm/min = 10 x 2.06 s = 20.6 s;
    // (300 mm of travel + 10 x 2.06 mm of going up) at 3000 mm/min = 320.6 / 3000 x 60 = 6.412 s
    let t = estimate_time(&Job::default(), 3000.0, 600.0, 300.0, 10).unwrap();
    assert!((t.cutting_s - 120.0).abs() < 1e-9, "{t:?}");
    assert!((t.plunging_s - 20.6).abs() < 1e-9, "{t:?}");
    assert!((t.rapid_s - 6.412).abs() < 1e-9, "{t:?}");
    assert!((t.total_s() - 147.012).abs() < 1e-9, "{t:?}");
    // no work, no time
    let none = estimate_time(&Job::default(), 3000.0, 0.0, 0.0, 0).unwrap();
    assert_eq!(none.total_s(), 0.0);
    // a speed that is not above zero, or a length that is not a number, is refused
    for rapid in [0.0, -5.0, f64::NAN] {
        assert!(estimate_time(&Job::default(), rapid, 1.0, 1.0, 1).is_err());
    }
    assert!(estimate_time(&Job::default(), 3000.0, f64::NAN, 1.0, 1).is_err());
    assert!(estimate_time(&Job::default(), 3000.0, 1.0, -1.0, 1).is_err());
}

#[test]
fn a_time_is_written_for_a_person() {
    use eh3d_isocam::gcode::duration_text;
    for (seconds, text) in [
        (0.0, "0 s"),
        (-3.0, "0 s"),
        (f64::NAN, "0 s"),
        (44.6, "45 s"),
        (147.012, "2 min 27 s"),
        (59.6, "1 min 0 s"),
        (3725.0, "1 h 2 min"),
    ] {
        assert_eq!(duration_text(seconds), text, "{seconds}");
    }
}

#[test]
fn the_settings_name_the_thinnest_tool_and_the_speed_of_the_rapid_moves() {
    let d = Settings::default();
    assert_eq!((d.min_tool_mm, d.rapid_mm_min), (0.02, 0.0));
    let s = Settings::parse("min_tool_mm = 0.1\nrapid_mm_min = 3000\n").unwrap();
    assert_eq!((s.min_tool_mm, s.rapid_mm_min), (0.1, 3000.0));
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    // the gap that counts as a conflict never goes below the thinnest tool
    assert_eq!(s.conflict_gap_mm(0.05), 0.1);
    assert_eq!(d.conflict_gap_mm(0.05), 0.05);
    for (text, part) in [
        ("min_tool_mm = 0.01\n", "from 0.02 to 10"),
        ("min_tool_mm = 11\n", "from 0.02 to 10"),
        ("rapid_mm_min = 50\n", "rapid_mm_min is 50"),
        ("rapid_mm_min = -1\n", "rapid_mm_min is -1"),
        (
            "min_tool_mm = 0.05\nfine_tool_mm = 0.03\n",
            "fine_tool_mm is 0.03",
        ),
        (
            "min_tool_mm = 0.3\nfine_tool_mm = 0.4\nnet_tool_mm = 0.25\n",
            "net_tool_mm is 0.25",
        ),
        (
            "min_tool_mm = 0.1\nfine_tool_mm = 0.2\nconflict_tolerance_mm = 0.15\n",
            "at least 0.1 mm",
        ),
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?} gave {e:?}");
    }
    // a tool that is switched off (0) is not a tool below the limit
    assert!(Settings::parse("min_tool_mm = 0.1\ncoarse_tool_mm = 0\nrubout_tool_mm = 0\n").is_ok());
}

// ---- the resolution of a plot ---------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_plot_is_written_with_the_units_to_the_millimetre_that_are_asked_for() {
    use eh3d_isocam::hpgl::write_holes_scaled;
    let holes = vec![Hole {
        tool: 1,
        x_mm: 100.0,
        y_mm: 50.0,
        diameter_mm: 0.8,
    }];
    // 100 mm = 4000 units and 50 mm = 2000 units at 40 to the millimetre; 10000 and 5000 at 100
    let usual = write_holes_at(&holes, &[], false, None).unwrap();
    assert!(usual.contains("PU4000,2000;PD;PU;"), "{usual}");
    assert_eq!(
        write_holes_scaled(&holes, &[], false, None, 40.0).unwrap(),
        usual
    );
    let fine = write_holes_scaled(&holes, &[], false, None, 100.0).unwrap();
    assert!(fine.contains("PU10000,5000;PD;PU;"), "{fine}");
    // the paths: 5 mm = 200 units and 3 mm = 120 at 40; 500 and 300 at 100
    let p = pass(vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 3.0)]]);
    let order = Order::default();
    let at40 = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions::default(),
    )
    .unwrap();
    assert!(at40.contains("200,0") && at40.contains("200,120"), "{at40}");
    let at100 = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions {
            units_per_mm: 100.0,
            ..PlotOptions::default()
        },
    )
    .unwrap();
    assert!(
        at100.contains("500,0") && at100.contains("500,300"),
        "{at100}"
    );
    // a resolution that is not above zero is refused
    for bad in [0.0, -40.0, f64::NAN] {
        assert!(write_holes_scaled(&holes, &[], false, None, bad).is_err());
    }
}

#[test]
fn the_range_of_a_plot_follows_its_resolution() {
    // 400 mm is 16000 units at 40 to the millimetre, which a plotter takes, and 40000 at 100, which it does not (32767 at the most = 327 mm)
    let p = pass(vec![vec![(0.0, 0.0), (400.0, 0.0), (400.0, 10.0)]]);
    let order = Order::default();
    let ok = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions::default(),
    );
    assert!(ok.is_ok());
    let e = write_opts(
        std::slice::from_ref(&p),
        &order,
        PenBy::Pass,
        &PlotOptions {
            units_per_mm: 100.0,
            ..PlotOptions::default()
        },
    )
    .unwrap_err();
    assert!(
        e.contains("beyond 32767 units (328 mm at 100 units to the millimetre)"),
        "{e}"
    );
}

#[test]
fn the_settings_name_the_resolution_of_a_plot() {
    assert_eq!(Settings::default().hpgl_units_per_mm, 40.0);
    let s = Settings::parse("hpgl_units_per_mm = 100\n").unwrap();
    assert_eq!(s.hpgl_units_per_mm, 100.0);
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    for text in [
        "hpgl_units_per_mm = 9\n",
        "hpgl_units_per_mm = 1001\n",
        "hpgl_units_per_mm = many\n",
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains("hpgl_units_per_mm"), "{text:?} gave {e:?}");
    }
    assert!(Settings::parse("hpgl_units_per_mm = 10\n").is_ok());
    assert!(Settings::parse("hpgl_units_per_mm = 1000\n").is_ok());
}
