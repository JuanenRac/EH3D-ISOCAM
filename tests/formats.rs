//! The files the program writes and reads: G-code, HP-GL, DXF, drill files and the kind of a file, with their limits and the mistakes they refuse.

use eh3d_isocam::drill::{read_drills, Hole};
use eh3d_isocam::dxf;
use eh3d_isocam::filetype::{detect, expect_drills, expect_gerber, FileKind};
use eh3d_isocam::gcode::{self, Job};
use eh3d_isocam::hpgl::{
    self, parse_pen_map, pen_map_text, pens_for, set_pen_speed, PenBy, PlotOptions, MAX_PENS,
};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::{LoopStart, Order};
use eh3d_isocam::units::Unit;

fn square(x: f64, y: f64, s: f64) -> Vec<(f64, f64)> {
    vec![(x, y), (x + s, y), (x + s, y + s), (x, y + s)]
}

fn pass(paths: Vec<Vec<(f64, f64)>>, diameter: f64) -> Pass {
    Pass {
        offset_mm: 0.1,
        paths,
        diameter_mm: diameter,
        ..Pass::default()
    }
}

// ---- G-code -----------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_settings_of_the_job_are_valid_exactly_inside_their_ranges() {
    let ok = |j: Job| j.validate().is_ok();
    let d = Job::default();
    assert!(ok(d));
    assert!(
        ok(Job {
            safe_z_mm: 0.5,
            ..d
        }) && !ok(Job {
            safe_z_mm: 0.49,
            ..d
        })
    );
    assert!(
        ok(Job {
            safe_z_mm: 50.0,
            tool_change_z_mm: 50.0,
            ..d
        }) && !ok(Job {
            safe_z_mm: 50.1,
            ..d
        })
    );
    assert!(
        ok(Job {
            tool_change_z_mm: d.safe_z_mm,
            ..d
        }),
        "the change height may be the safe height"
    );
    assert!(
        !ok(Job {
            tool_change_z_mm: d.safe_z_mm - 0.01,
            ..d
        }),
        "a tool cannot be changed lower than it travels"
    );
    assert!(!ok(Job {
        tool_change_z_mm: 200.5,
        ..d
    }));
    assert!(
        ok(Job {
            depth_mm: 0.005,
            ..d
        }) && !ok(Job {
            depth_mm: 0.004,
            ..d
        }) && ok(Job { depth_mm: 2.0, ..d })
            && !ok(Job {
                depth_mm: 2.01,
                ..d
            })
    );
    assert!(
        ok(Job {
            feed_mm_min: 10_000.0,
            ..d
        }) && !ok(Job {
            feed_mm_min: 10_001.0,
            ..d
        }) && !ok(Job {
            feed_mm_min: 0.5,
            ..d
        })
    );
    assert!(
        ok(Job {
            plunge_mm_min: 5_000.0,
            ..d
        }) && !ok(Job {
            plunge_mm_min: 5_001.0,
            ..d
        })
    );
    assert!(
        ok(Job {
            spindle_rpm: 1000.0,
            ..d
        }) && !ok(Job {
            spindle_rpm: 999.0,
            ..d
        }) && !ok(Job {
            spindle_rpm: f64::NAN,
            ..d
        })
    );
    let e = Job {
        feed_mm_min: -1.0,
        ..d
    }
    .validate()
    .unwrap_err();
    assert!(e.contains("feed_mm_min") && e.contains("-1"), "{e}");
}

#[test]
fn a_closed_path_is_cut_all_the_way_round_and_back_to_where_it_started() {
    let g = gcode::write(&[pass(vec![square(10.0, 10.0, 5.0)], 0.2)], &Job::default()).unwrap();
    let cuts: Vec<&str> = g.lines().filter(|l| l.starts_with("G1 X")).collect();
    assert_eq!(cuts.len(), 4, "{g}");
    let first = g
        .lines()
        .find(|l| l.starts_with("G0 X"))
        .unwrap()
        .trim_start_matches("G0 ")
        .to_string();
    let last = cuts
        .last()
        .unwrap()
        .trim_start_matches("G1 ")
        .split(" F")
        .next()
        .unwrap()
        .to_string();
    assert_eq!(first, last, "the last move returns to the first point");
    assert!(
        g.starts_with("G21\nG90\nG0 Z2\nM3 S24000\n") && g.ends_with("G0 Z2\nM5\nM2\n"),
        "{g}"
    );
    assert_eq!(g.matches("G1 Z-0.06 F60").count(), 1);
}

#[test]
fn an_open_run_is_not_closed_and_a_pass_without_paths_writes_nothing() {
    let run = Pass {
        open: vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]],
        diameter_mm: 0.2,
        ..Pass::default()
    };
    let g = gcode::write(&[Pass::default(), run], &Job::default()).unwrap();
    assert_eq!(
        g.matches("G1 X").count(),
        2,
        "an open run of three points is two moves"
    );
    assert!(!g.contains("(pass 1"), "the empty pass is left out: {g}");
    assert!(g.contains("(pass 2 at"));
    assert_eq!(
        gcode::write(&[], &Job::default()).unwrap(),
        "G21\nG90\nG0 Z2\nM3 S24000\nM5\nM2\n"
    );
}

#[test]
fn the_program_stops_for_the_tool_only_when_the_width_changes() {
    let a = pass(vec![square(0.0, 0.0, 5.0)], 0.2);
    let b = pass(vec![square(10.0, 0.0, 5.0)], 0.2);
    let c = pass(vec![square(20.0, 0.0, 5.0)], 0.8);
    let unknown = pass(vec![square(30.0, 0.0, 5.0)], 0.0);
    assert_eq!(
        gcode::write(&[a.clone(), b.clone()], &Job::default())
            .unwrap()
            .matches("M0\n")
            .count(),
        0
    );
    let changed = gcode::write(&[a.clone(), c.clone()], &Job::default()).unwrap();
    assert_eq!(changed.matches("M0\n").count(), 1);
    assert!(
        changed.contains("change to a tool of 0.8 mm") && changed.contains("G0 Z30\n"),
        "{changed}"
    );
    assert_eq!(
        gcode::write(&[a.clone(), unknown.clone(), b.clone()], &Job::default())
            .unwrap()
            .matches("M0\n")
            .count(),
        0,
        "a pass with no known width is not a change"
    );
    assert_eq!(
        gcode::write(&[a, c, b], &Job::default())
            .unwrap()
            .matches("M0\n")
            .count(),
        2,
        "0.2 to 0.8 and back"
    );
}

#[test]
fn inches_convert_every_length_and_the_feeds_but_not_the_speed_and_mils_are_refused() {
    let p = pass(vec![square(25.4, 25.4, 25.4)], 0.2);
    let g = gcode::write_in(
        std::slice::from_ref(&p),
        &Job::default(),
        &Order::default(),
        Unit::Inch,
    )
    .unwrap();
    assert!(g.starts_with("G20\nG90\nG0 Z0.07874\nM3 S24000\n"), "{g}");
    assert!(g.contains("G1 Z-0.00236 F2.3622"), "{g}");
    assert!(
        g.contains("X1 Y1") || g.contains("X2 Y1") || g.contains("X1 Y2"),
        "{g}"
    );
    assert!(g.contains(" in from the copper"));
    let e = gcode::write_in(&[p], &Job::default(), &Order::default(), Unit::Mil).unwrap_err();
    assert!(e.contains("not in mils"), "{e}");
}

#[test]
fn the_numbers_of_the_gcode_never_say_minus_zero() {
    let p = pass(vec![vec![(-0.0004, -0.0004), (5.0, 0.0), (5.0, 5.0)]], 0.2);
    let g = gcode::write(&[p], &Job::default()).unwrap();
    assert!(!g.contains("-0 ") && !g.contains("-0\n"), "{g}");
    assert!(g.contains("G0 X0 Y0\n"));
}

#[test]
fn a_bad_job_or_start_writes_nothing() {
    let p = pass(vec![square(0.0, 0.0, 5.0)], 0.2);
    assert!(gcode::write(
        std::slice::from_ref(&p),
        &Job {
            depth_mm: 0.0,
            ..Job::default()
        }
    )
    .is_err());
    let order = Order {
        start: (f64::NAN, 0.0),
        ..Order::default()
    };
    assert!(gcode::write_with(&[p], &Job::default(), &order)
        .unwrap_err()
        .contains("starting point"));
}

#[test]
fn the_total_length_of_closed_paths_includes_the_closing_side() {
    assert!((gcode::length(&[square(0.0, 0.0, 10.0)]) - 40.0).abs() < 1e-12);
    assert!(
        (gcode::length(&[square(0.0, 0.0, 10.0), square(50.0, 0.0, 1.0)]) - 44.0).abs() < 1e-12
    );
    assert_eq!(gcode::length(&[]), 0.0);
    assert_eq!(gcode::length(&[vec![(1.0, 1.0)]]), 0.0);
    assert!(
        (gcode::length(&[vec![(0.0, 0.0), (3.0, 4.0)]]) - 10.0).abs() < 1e-12,
        "two points: there and back"
    );
}

#[test]
fn the_same_passes_give_the_same_gcode_every_time() {
    let passes = vec![pass(
        vec![
            square(0.0, 0.0, 5.0),
            square(20.0, 3.0, 2.0),
            square(8.0, 9.0, 1.0),
        ],
        0.2,
    )];
    assert_eq!(
        gcode::write(&passes, &Job::default()).unwrap(),
        gcode::write(&passes, &Job::default()).unwrap()
    );
}

// ---- HP-GL ---------------------------------------------------------------------------------------------------------------------------------------------------------

fn points_of_plot(plot: &str) -> Vec<(i64, i64)> {
    // every pair of numbers that follows PU, PD or PA, in the order of the file
    let flat = plot.replace(['\r', '\n'], "");
    let mut out = Vec::new();
    for cmd in flat.split(';') {
        let args = ["PU", "PD", "PA"].iter().find_map(|p| cmd.strip_prefix(p));
        if let Some(a) = args.filter(|a| !a.is_empty()) {
            let n: Vec<i64> = a.split(',').map(|v| v.parse().unwrap()).collect();
            out.extend(n.chunks(2).map(|c| (c[0], c[1])));
        }
    }
    out
}

#[test]
fn a_plot_has_one_pen_a_pass_closed_paths_and_the_pen_put_away() {
    let plot = hpgl::write(&[
        pass(vec![square(1.0, 1.0, 2.0)], 0.2),
        pass(vec![square(10.0, 1.0, 2.0)], 0.2),
    ])
    .unwrap();
    assert!(
        plot.starts_with("IN;\nSP1;\n") && plot.ends_with("PU0,0;SP0;\n"),
        "{plot}"
    );
    assert_eq!(plot.matches("SP2;").count(), 1);
    let pd = plot.lines().find(|l| l.starts_with("PD")).unwrap();
    let pts: Vec<&str> = pd[2..pd.len() - 1].split(',').collect();
    assert_eq!(
        pts.len(),
        2 * 4,
        "the first corner is in the PU, the other three and the first one again are in the PD"
    );
    let pu = plot
        .lines()
        .find(|l| l.starts_with("PU") && !l.starts_with("PU0,0"))
        .unwrap();
    let start: Vec<&str> = pu[2..pu.len() - 1].split(',').collect();
    assert_eq!(
        start,
        &pts[pts.len() - 2..],
        "a closed path ends where it began"
    );
    assert_eq!(hpgl::write(&[]).unwrap(), "IN;\nPU0,0;SP0;\n");
}

#[test]
fn paths_with_negative_coordinates_are_moved_unless_asked_to_stay() {
    let passes = [pass(vec![vec![(-5.0, -2.0), (5.0, -2.0), (5.0, 3.0)]], 0.2)];
    let moved = hpgl::write_opts(
        &passes,
        &Order {
            loop_start: LoopStart::Original,
            ..Order::default()
        },
        PenBy::Pass,
        &PlotOptions::default(),
    )
    .unwrap();
    assert!(
        points_of_plot(&moved)
            .iter()
            .all(|&(x, y)| x >= 0 && y >= 0),
        "{moved}"
    );
    assert!(
        moved.contains("PU0,0;\nPD"),
        "the lowest-left corner is the origin: {moved}"
    );
    let kept = hpgl::write_opts(
        &passes,
        &Order {
            loop_start: LoopStart::Original,
            ..Order::default()
        },
        PenBy::Pass,
        &PlotOptions {
            keep_negative: true,
            ..PlotOptions::default()
        },
    )
    .unwrap();
    assert!(kept.contains("PU-200,-80;"), "{kept}");
}

#[test]
fn the_pens_follow_the_pass_the_tool_or_the_choice_and_there_are_at_most_eight() {
    let by_tool = |tool: usize| Pass {
        tool,
        ..pass(vec![square(0.0, f64::from(tool as u8) * 10.0, 2.0)], 0.2)
    };
    let passes = [by_tool(0), by_tool(2), by_tool(0)];
    let order = Order::default();
    let tool = hpgl::write_with(&passes, &order, PenBy::Tool).unwrap();
    assert!(
        tool.contains("SP1;") && tool.contains("SP3;") && !tool.contains("SP2;"),
        "{tool}"
    );
    let chosen = hpgl::write_opts(
        &passes,
        &order,
        PenBy::Pass,
        &PlotOptions {
            pen_of_pass: vec![0, 7, 0],
            ..PlotOptions::default()
        },
    )
    .unwrap();
    assert!(
        chosen.contains("SP1;") && chosen.contains("SP7;") && chosen.contains("SP3;"),
        "{chosen}"
    );
    let nine: Vec<Pass> = (0..=MAX_PENS)
        .map(|i| pass(vec![square(f64::from(i as u8) * 10.0, 0.0, 2.0)], 0.2))
        .collect();
    let e = hpgl::write(&nine).unwrap_err();
    assert!(e.contains('9') && e.contains(&MAX_PENS.to_string()), "{e}");
    assert!(hpgl::write_with(
        &passes,
        &Order {
            start: (f64::INFINITY, 0.0),
            ..order
        },
        PenBy::Pass
    )
    .is_err());
}

#[test]
fn the_pen_speed_is_put_once_after_the_start_and_only_inside_its_limits() {
    let plot = "IN;\nSP1;\nPU0,0;\nPD1,1;\nPU0,0;SP0;\n";
    assert_eq!(set_pen_speed(plot, 0.0).unwrap(), plot);
    assert!(set_pen_speed(plot, 5.0)
        .unwrap()
        .starts_with("IN;\nVS5;\nSP1;"));
    assert!(set_pen_speed(plot, 2.5)
        .unwrap()
        .starts_with("IN;\nVS2.5;\n"));
    assert!(set_pen_speed(plot, 0.1)
        .unwrap()
        .starts_with("IN;\nVS0.1;\n"));
    assert!(set_pen_speed(plot, 100.0)
        .unwrap()
        .starts_with("IN;\nVS100;\n"));
    assert_eq!(set_pen_speed(plot, 5.0).unwrap().matches("VS").count(), 1);
    for bad in [0.05, 100.01, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            set_pen_speed(plot, bad).unwrap_err().contains("cm/s"),
            "{bad}"
        );
    }
}

#[test]
fn the_style_of_the_original_program_keeps_every_point_in_order_and_ends_every_line_with_crlf() {
    let plot = hpgl::write(&[pass(
        vec![square(1.0, 1.0, 20.0), square(40.0, 5.0, 3.0)],
        0.2,
    )])
    .unwrap();
    let styled = hpgl::isocam_style(&plot);
    assert_eq!(
        points_of_plot(&plot)
            .iter()
            .filter(|p| **p != (0, 0))
            .collect::<Vec<_>>(),
        points_of_plot(&styled)
            .iter()
            .filter(|p| **p != (0, 0))
            .collect::<Vec<_>>()
    );
    assert!(styled.ends_with("PU;\r\nPA0,0;SP;\r\n"), "{styled:?}");
    assert!(
        !styled.replace("\r\n", "").contains('\n') && !styled.replace("\r\n", "").contains('\r')
    );
    assert!(styled.starts_with("IN;\r\nSP1;\r\n"), "{styled:?}");
    assert!(styled.lines().all(|l| l.len() < 100), "no line runs on");
}

#[test]
fn a_map_of_tools_to_pens_reads_back_as_it_was_written_and_refuses_what_makes_no_sense() {
    let map = parse_pen_map(" 1:2 , 3:1,,12 : 8 ").unwrap();
    assert_eq!(map, vec![(1, 2), (3, 1), (12, 8)]);
    assert_eq!(pen_map_text(&map), "1:2, 3:1, 12:8");
    assert_eq!(parse_pen_map(&pen_map_text(&map)).unwrap(), map);
    assert!(parse_pen_map("").unwrap().is_empty());
    for bad in [
        "1", "1:", ":2", "0:1", "1:0", "1:9", "256:1", "-1:1", "a:b", "1:2,1:3", "1:2:3",
    ] {
        assert!(parse_pen_map(bad).is_err(), "{bad:?}");
    }
    assert!(parse_pen_map("1:8").is_ok() && parse_pen_map("255:1").is_ok());
}

#[test]
fn unmapped_tools_take_the_lowest_pens_that_nobody_asked_for() {
    // tool 5 asked for pen 1; tools 2 and 9 are not mapped and take 2 and 3
    let pens = pens_for(&[9, 2, 5, 2], &[(5, 1)]).unwrap();
    let pen = |t: u8| pens.iter().find(|(x, _)| *x == t).unwrap().1;
    assert_eq!((pen(5), pen(2), pen(9)), (1, 2, 3));
    assert_eq!(pens.len(), 3, "a tool that appears twice has one pen");
    // a pen asked for by a tool that is not used is still kept free
    let free = pens_for(&[1, 2], &[(7, 1)]).unwrap();
    assert!(free.iter().all(|(_, p)| *p != 1), "{free:?}");
    let many: Vec<u8> = (1..=9).collect();
    assert!(pens_for(&many, &[]).unwrap_err().contains("hpgl_pen_map"));
    assert!(pens_for(&many[..8], &[]).is_ok());
}

#[test]
fn the_holes_are_plotted_by_pen_in_the_order_of_the_pens_and_negative_ones_are_moved() {
    let holes = [
        Hole {
            tool: 2,
            x_mm: -1.0,
            y_mm: 5.0,
            diameter_mm: 1.0,
        },
        Hole {
            tool: 1,
            x_mm: 2.0,
            y_mm: -3.0,
            diameter_mm: 0.8,
        },
        Hole {
            tool: 2,
            x_mm: 4.0,
            y_mm: 5.0,
            diameter_mm: 1.0,
        },
    ];
    let plot = hpgl::write_holes(&holes, &[(2, 1), (1, 2)]).unwrap();
    assert!(plot.find("SP1;").unwrap() < plot.find("SP2;").unwrap());
    let after_sp1 = &plot[plot.find("SP1;").unwrap()..plot.find("SP2;").unwrap()];
    assert_eq!(
        after_sp1.matches("PD;PU;").count(),
        2,
        "both holes of tool 2 come under pen 1: {plot}"
    );
    assert!(
        points_of_plot(&plot).iter().all(|&(x, y)| x >= 0 && y >= 0),
        "{plot}"
    );
    assert!(hpgl::write_holes(&[], &[])
        .unwrap_err()
        .contains("no holes"));
    assert!(hpgl::write_holes(
        &[Hole {
            tool: 1,
            x_mm: f64::NAN,
            y_mm: 0.0,
            diameter_mm: 1.0
        }],
        &[]
    )
    .is_err());
    let kept = hpgl::write_holes_opts(&holes, &[], true).unwrap();
    assert!(kept.contains("PU-40,200;"), "{kept}");
}

// ---- DXF ------------------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_dxf_has_a_layer_for_each_pass_a_flag_for_closed_paths_and_all_the_vertices() {
    let rub = Pass {
        rubout: true,
        ..pass(vec![square(0.0, 0.0, 3.0)], 0.2)
    };
    let run = Pass {
        open: vec![vec![(0.0, 0.0), (9.0, 9.0)]],
        ..Pass::default()
    };
    let text = dxf::write(&[pass(vec![square(1.0, 1.0, 2.0)], 0.2), rub, run]).unwrap();
    assert!(text.starts_with("0\nSECTION\n2\nENTITIES\n") && text.ends_with("0\nENDSEC\n0\nEOF\n"));
    assert!(
        text.contains("8\nPASS_1\n")
            && text.contains("8\nRUBOUT_2\n")
            && text.contains("8\nPASS_3\n")
    );
    assert_eq!(text.matches("0\nPOLYLINE\n").count(), 3);
    assert_eq!(text.matches("0\nVERTEX\n").count(), 4 + 4 + 2);
    assert_eq!(text.matches("70\n1\n").count(), 2, "two closed polylines");
    assert_eq!(text.matches("70\n0\n").count(), 1, "one open run");
    assert_eq!(text.matches("0\nSEQEND\n").count(), 3);
    assert_eq!(detect(&text), FileKind::Dxf);
}

#[test]
fn the_dxf_refuses_nothing_to_write_and_a_coordinate_that_is_not_a_number() {
    assert!(dxf::write(&[]).unwrap_err().contains("no path"));
    assert!(dxf::write(&[Pass::default()])
        .unwrap_err()
        .contains("no path"));
    let bad = pass(vec![vec![(0.0, 0.0), (f64::NAN, 1.0), (2.0, 2.0)]], 0.2);
    assert!(dxf::write(&[bad]).unwrap_err().contains("not a number"));
}

// ---- the kind of a file ------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn each_kind_of_file_is_told_from_a_few_lines_of_its_text() {
    for (text, want) in [
        ("%FSLAX24Y24*%\n%MOMM*%\nM02*\n", FileKind::Gerber),
        (
            "M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n",
            FileKind::Excellon,
        ),
        ("T1C0.8\nX1.0Y1.0\nM30\n", FileKind::Excellon),
        ("%\nT1\nX1.0Y1.0\nT2\nX2.0Y2.0\n", FileKind::Excellon),
        ("IN;SP1;PU0,0;PD10,10;PU;", FileKind::Hpgl),
        ("PU0,0;PD10,10;PU20,0;PD30,10;", FileKind::Hpgl),
        (
            "G04 old*\nD10*\nX100Y200D02*\nX300Y200D01*\nM02*\n",
            FileKind::GerberOldStyle,
        ),
        (
            "0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n",
            FileKind::Dxf,
        ),
        ("999\ncomment\n0\nSECTION\n0\nEOF\n", FileKind::Dxf),
        ("", FileKind::Unknown),
        ("hello\nworld\n", FileKind::Unknown),
        ("G21\nG1 X10 F100\nM30\n", FileKind::Unknown),
        ("\0\u{1}\u{2}", FileKind::Unknown),
    ] {
        assert_eq!(detect(text), want, "{text:?}");
    }
}

#[test]
fn a_dxf_that_never_ends_and_a_plot_with_one_stroke_are_not_taken_for_what_they_look_like() {
    assert_eq!(
        detect("0\nSECTION\n2\nENTITIES\n"),
        FileKind::Unknown,
        "no EOF"
    );
    assert_eq!(detect("PU0,0;"), FileKind::Unknown);
    assert_eq!(
        detect("IN;PD1,1;"),
        FileKind::Unknown,
        "one stroke is not a plot"
    );
    assert_eq!(
        detect("PU0,0;PD1,1;PU2,2;PD3,3;*"),
        FileKind::Unknown,
        "a star means blocks of Gerber, not a plot"
    );
}

#[test]
fn the_kind_comes_out_in_plain_words_and_the_wrong_kind_says_what_to_do() {
    assert!(FileKind::Excellon.name().contains("drill"));
    assert!(
        FileKind::Gerber.is_gerber()
            && FileKind::GerberOldStyle.is_gerber()
            && !FileKind::Excellon.is_gerber()
    );
    let e = expect_gerber("holes.drl", FileKind::Excellon).unwrap_err();
    assert!(
        e.starts_with("holes.drl is an Excellon") && e.contains("--drills"),
        "{e}"
    );
    assert!(expect_gerber("o.plt", FileKind::Hpgl)
        .unwrap_err()
        .contains("--outline"));
    assert!(expect_gerber("o.dxf", FileKind::Dxf)
        .unwrap_err()
        .contains("DXF"));
    assert!(
        expect_gerber("x", FileKind::Unknown).is_ok()
            && expect_gerber("x", FileKind::Gerber).is_ok()
    );
    assert!(expect_drills("top.gtl", FileKind::Gerber)
        .unwrap_err()
        .contains("not a drill file"));
    assert!(expect_drills("top.gtl", FileKind::GerberOldStyle).is_err());
    assert!(
        expect_drills("o.plt", FileKind::Hpgl).is_err()
            && expect_drills("o.dxf", FileKind::Dxf).is_err()
    );
    assert!(
        expect_drills("x", FileKind::Unknown).is_ok()
            && expect_drills("x", FileKind::Excellon).is_ok()
    );
}

// ---- reading drill files -------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_metric_drill_file_with_slots_says_what_it_left_out_and_a_file_without_units_says_so() {
    let d =
        read_drills("M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\nX2.0Y2.0G85X5.0Y2.0\nM30\n").unwrap();
    assert_eq!(d.holes.len(), 1);
    assert!(
        d.notes.iter().any(|n| n.contains("1 slots")),
        "{:?}",
        d.notes
    );
    assert_eq!(
        d.holes[0],
        Hole {
            tool: 1,
            x_mm: 1.0,
            y_mm: 1.0,
            diameter_mm: 0.8
        }
    );
}

#[test]
fn a_drill_file_with_no_holes_or_holes_outside_any_board_is_refused() {
    assert!(read_drills("M48\nMETRIC\nT1C0.8\n%\nM30\n")
        .unwrap_err()
        .contains("no holes"));
    assert!(read_drills("").is_err());
    assert!(
        read_drills("M48\nMETRIC\nT1C0.8\n%\nT1\nX9999.0Y1.0\nM30\n")
            .unwrap_err()
            .contains("out of range")
    );
    assert!(
        read_drills("M48\nMETRIC\nT1C75.0\n%\nT1\nX1.0Y1.0\nM30\n")
            .unwrap_err()
            .contains("out of range"),
        "a drill of 75 mm is not a drill"
    );
    assert!(
        read_drills("M48\nMETRIC\nT1C0.8\n%\nT1\nX5000.0Y-5000.0\nM30\n").is_ok(),
        "5000 mm is the largest accepted, in either direction"
    );
}

#[test]
fn inch_drill_files_become_millimetres_and_the_diameters_too() {
    let d = read_drills("M48\nINCH\nT1C0.0315\n%\nT1\nX1.0Y2.0\nM30\n").unwrap();
    assert!((d.holes[0].x_mm - 25.4).abs() < 1e-9 && (d.holes[0].y_mm - 50.8).abs() < 1e-9);
    assert!(
        (d.holes[0].diameter_mm - 0.8001).abs() < 1e-3,
        "{}",
        d.holes[0].diameter_mm
    );
}
