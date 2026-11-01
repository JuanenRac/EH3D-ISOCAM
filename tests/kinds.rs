//! The tolerance for pads alone, the filling of contours, the kind of an input file, the pens of a plot of holes, the pen speed and the DXF output.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::drill::Hole;
use eh3d_isocam::dxf;
use eh3d_isocam::filetype::{detect, expect_drills, expect_gerber, FileKind};
use eh3d_isocam::fill::{rubout_contours, Fill};
use eh3d_isocam::hpgl::{parse_pen_map, pen_map_text, pens_for, set_pen_speed, write_holes};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::problems::{thin_for_tolerance, thin_for_tolerance_in};
use eh3d_isocam::tolerance::{close_pieces_scoped, ToleranceScope};
use eh3d_isocam::transform::Axis;
use std::process::Command;

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn track(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Shape {
    Shape::Stroke { x0, y0, x1, y1, w }
}

// ---------------------------------------------------------------- tolerance only for pads

/// A pad with a track 0.18 mm from it, and two tracks 0.18 mm apart: both gaps are 0.02 mm narrower than a tool of 0.2 mm.
fn two_narrow_gaps() -> Scene {
    scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 0.5,
        },
        track(-2.0, 0.73, 2.0, 0.73, 0.1),
        track(-2.0, 5.0, 2.0, 5.0, 0.1),
        track(-2.0, 5.28, 2.0, 5.28, 0.1),
    ])
}

#[test]
fn the_tolerance_for_all_objects_accepts_both_gaps() {
    let s = two_narrow_gaps();
    let none = close_pieces_scoped(&s, 0.2, 0.0, ToleranceScope::All, 0.02).unwrap();
    assert_eq!(none.len(), 2);
    let all = close_pieces_scoped(&s, 0.2, 0.04, ToleranceScope::All, 0.02).unwrap();
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn the_tolerance_for_pads_accepts_only_the_gap_beside_a_pad() {
    let s = two_narrow_gaps();
    let pads = close_pieces_scoped(&s, 0.2, 0.04, ToleranceScope::Pads, 0.02).unwrap();
    assert_eq!(pads.len(), 1, "{pads:?}");
    assert!(
        pads[0].y > 4.5,
        "the gap that stays is the one between the tracks: {pads:?}"
    );
}

#[test]
fn the_tolerance_for_pads_with_no_pad_in_the_layer_accepts_nothing() {
    let s = scene(vec![
        track(-2.0, 5.0, 2.0, 5.0, 0.1),
        track(-2.0, 5.28, 2.0, 5.28, 0.1),
    ]);
    let r = close_pieces_scoped(&s, 0.2, 0.04, ToleranceScope::Pads, 0.02).unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn a_gap_too_narrow_even_for_the_tolerance_stays_beside_a_pad() {
    // 0.1 mm beside a pad: narrower than 0.2 - 0.04
    let s = scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 0.5,
        },
        track(-2.0, 0.65, 2.0, 0.65, 0.1),
    ]);
    let r = close_pieces_scoped(&s, 0.2, 0.04, ToleranceScope::Pads, 0.02).unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn the_thin_copper_warning_leaves_out_tracks_when_only_pads_count() {
    let s = scene(vec![
        track(0.0, 0.0, 5.0, 0.0, 0.1),
        Shape::Rect {
            x: 10.0,
            y: 0.0,
            w: 0.1,
            h: 2.0,
        },
    ]);
    assert_eq!(thin_for_tolerance(&s, 0.05, 0.05).unwrap().len(), 2);
    assert_eq!(
        thin_for_tolerance_in(&s, 0.05, 0.05, true).unwrap().len(),
        1
    );
}

#[test]
fn the_scope_is_a_setting_with_a_default_that_changes_nothing() {
    let mut s = Settings::default();
    assert_eq!(s.conflict_tolerance_scope, ToleranceScope::All);
    s.set("conflict_tolerance_scope", "pads").unwrap();
    s.set("rubout_fill_margin_mm", "0.05").unwrap();
    s.set("hpgl_pen_map", "1:3, 2:1").unwrap();
    s.set("hpgl_pen_speed_cm_s", "8").unwrap();
    s.validate().unwrap();
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    assert!(s.set("conflict_tolerance_scope", "tracks").is_err());
    assert!(s.set("hpgl_pen_map", "1-3").is_err());
    assert!(s.set("hpgl_pen_map", "0:1").is_err());
    assert!(s.set("hpgl_pen_map", "1:9").is_err());
    assert!(s.set("hpgl_pen_map", "1:1, 1:2").is_err());
    s.set("hpgl_pen_map", "").unwrap();
    assert!(s.hpgl_pen_map.is_empty());
    s.hpgl_pen_speed_cm_s = 500.0;
    assert!(s.validate().is_err());
    s.hpgl_pen_speed_cm_s = 0.05;
    assert!(s.validate().is_err());
    let d = Settings::default();
    assert_eq!(
        d.hpgl_pen_speed_cm_s, 0.0,
        "no speed unless it is asked for"
    );
}

// ---------------------------------------------------------------- contours

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn fill(direction: Axis) -> Fill {
    Fill {
        tool_mm: 0.8,
        overlap: 0.2,
        direction,
        margin_mm: 0.0,
        res_mm: 0.02,
    }
}

#[test]
fn a_contour_is_filled_with_lines_that_stay_a_radius_inside() {
    let p = rubout_contours(&[square(0.0, 0.0, 4.0, 4.0)], None, &fill(Axis::Horizontal)).unwrap();
    assert!(p.rubout);
    assert_eq!(p.open.len(), 6);
    for l in &p.open {
        let ys = [l[0].1, l[1].1];
        assert!((ys[0] - ys[1]).abs() < 1e-9, "horizontal lines");
        let (a, b) = (l[0].0.min(l[1].0), l[0].0.max(l[1].0));
        assert!((a - 0.4).abs() < 1e-9 && (b - 3.6).abs() < 1e-9, "{l:?}");
    }
    assert!((p.open[0][0].1 - 0.4).abs() < 1e-9);
    assert!((p.open[5][0].1 - 3.6).abs() < 1e-9);
    // back and forth
    assert!(p.open[0][0].0 < p.open[0][1].0);
    assert!(p.open[1][0].0 > p.open[1][1].0);
}

#[test]
fn vertical_lines_fill_the_same_contour() {
    let p = rubout_contours(&[square(0.0, 0.0, 4.0, 2.0)], None, &fill(Axis::Vertical)).unwrap();
    assert!(p.open.iter().all(|l| (l[0].0 - l[1].0).abs() < 1e-9));
    let xs: Vec<f64> = p.open.iter().map(|l| l[0].0).collect();
    assert!((xs[0] - 0.4).abs() < 1e-9 && (xs[xs.len() - 1] - 3.6).abs() < 1e-9);
    assert!(p
        .open
        .iter()
        .all(|l| l.iter().all(|q| (0.4 - 1e-9..=1.6 + 1e-9).contains(&q.1))));
}

#[test]
fn a_path_inside_another_is_a_hole() {
    let p = rubout_contours(
        &[square(0.0, 0.0, 6.0, 6.0), square(2.0, 2.0, 4.0, 4.0)],
        None,
        &fill(Axis::Horizontal),
    )
    .unwrap();
    // the lines that cross the hole are cut in two
    let through_hole: Vec<_> = p
        .open
        .iter()
        .filter(|l| l[0].1 > 2.7 && l[0].1 < 3.4)
        .collect();
    assert!(through_hole.len() >= 2, "{through_hole:?}");
    for l in &p.open {
        let y = l[0].1;
        let (a, b) = (l[0].0.min(l[1].0), l[0].0.max(l[1].0));
        if y > 2.4 && y < 3.6 {
            assert!(
                b <= 1.6 + 1e-9 || a >= 4.4 - 1e-9,
                "line over the hole: {l:?}"
            );
        }
    }
}

#[test]
fn the_lines_are_cut_to_the_copper() {
    let copper = scene(vec![Shape::Rect {
        x: 2.0,
        y: 2.0,
        w: 2.0,
        h: 2.0,
    }]);
    let p = rubout_contours(
        &[square(0.0, 0.0, 4.0, 4.0)],
        Some(&copper),
        &fill(Axis::Horizontal),
    )
    .unwrap();
    assert_eq!(p.open.len(), 2, "{:?}", p.open);
    for l in &p.open {
        for q in l {
            assert!((1.3..=2.7).contains(&q.0), "{q:?}: on the copper");
            assert!((1.3..=2.7).contains(&q.1), "{q:?}: on the copper");
        }
    }
    // a margin pulls the lines back from the edge
    let far = rubout_contours(
        &[square(0.0, 0.0, 4.0, 4.0)],
        Some(&copper),
        &Fill {
            margin_mm: 0.7,
            ..fill(Axis::Horizontal)
        },
    );
    assert!(far.is_err(), "no line fits in what is left");
}

#[test]
fn bad_contours_and_tools_are_refused() {
    let f = fill(Axis::Horizontal);
    assert!(rubout_contours(&[], None, &f).is_err());
    assert!(rubout_contours(&[vec![(0.0, 0.0), (1.0, 1.0)]], None, &f).is_err());
    assert!(rubout_contours(&[square(0.0, 0.0, 4.0, 0.5)], None, &f).is_err());
    assert!(rubout_contours(&[square(0.0, 0.0, f64::NAN, 4.0)], None, &f).is_err());
    let bad = |g: Fill| rubout_contours(&[square(0.0, 0.0, 4.0, 4.0)], None, &g);
    assert!(bad(Fill { tool_mm: 0.0, ..f }).is_err());
    assert!(bad(Fill { overlap: 0.95, ..f }).is_err());
    assert!(bad(Fill {
        margin_mm: -1.0,
        ..f
    })
    .is_err());
}

// ---------------------------------------------------------------- kind of file

const GERBER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.3*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
const OLD_GERBER: &str = "G54D10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
const EXCELLON: &str = "M48\nMETRIC,TZ\nT01C0.8\n%\nT01\nX1.0Y2.0\nX3.0Y2.0\nM30\n";
const PLOT: &str = "IN;\nSP1;\nPU0,0;\nPD40,0,40,40;\nPU0,0;SP0;\n";
const DRAWING: &str =
    "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n0\nENDSEC\n0\nEOF\n";

#[test]
fn each_kind_of_input_is_told_apart() {
    assert_eq!(detect(GERBER), FileKind::Gerber);
    assert_eq!(detect(OLD_GERBER), FileKind::GerberOldStyle);
    assert_eq!(detect(EXCELLON), FileKind::Excellon);
    assert_eq!(detect(PLOT), FileKind::Hpgl);
    assert_eq!(detect(DRAWING), FileKind::Dxf);
    assert_eq!(detect(""), FileKind::Unknown);
    assert_eq!(detect("hello\nthere\n"), FileKind::Unknown);
    assert_eq!(detect("\u{0}\u{1}\u{2}"), FileKind::Unknown);
    // an Excellon file without a header, only tools and holes
    assert_eq!(
        detect("T1C0.8\nT1\nX100Y200\nX300Y200\nM30\n"),
        FileKind::Excellon
    );
}

#[test]
fn a_file_of_another_kind_is_refused_with_a_clear_message() {
    let e = expect_gerber("holes.drl", detect(EXCELLON)).unwrap_err();
    assert!(
        e.contains("Excellon drill file") && e.contains("--drills"),
        "{e}"
    );
    assert!(expect_gerber("a.plt", detect(PLOT))
        .unwrap_err()
        .contains("HP-GL"));
    assert!(expect_gerber("a.dxf", detect(DRAWING))
        .unwrap_err()
        .contains("DXF"));
    assert!(expect_gerber("a.gtl", detect(GERBER)).is_ok());
    assert!(expect_gerber("a.gbr", detect(OLD_GERBER)).is_ok());
    assert!(
        expect_gerber("a.txt", detect("???")).is_ok(),
        "unknown goes on to the reader"
    );
    assert!(expect_drills("a.gtl", detect(GERBER))
        .unwrap_err()
        .contains("Gerber layer"));
    assert!(expect_drills("a.drl", detect(EXCELLON)).is_ok());
    assert!(FileKind::Gerber.is_gerber() && !FileKind::Excellon.is_gerber());
}

#[test]
fn the_command_line_names_the_kind_and_refuses_the_wrong_one() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let drl = format!("{dir}/kinds_holes.drl");
    let gtl = format!("{dir}/kinds_layer.gtl");
    std::fs::write(&drl, EXCELLON).unwrap();
    std::fs::write(&gtl, GERBER).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["kind", &drl, &gtl])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(
        stdout.contains("Excellon drill file") && stdout.contains("Gerber layer"),
        "{stdout}"
    );
    let bad = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["check", &drl])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let err = String::from_utf8_lossy(&bad.stderr);
    assert!(
        err.contains("is an Excellon drill file, not a Gerber layer"),
        "{err}"
    );
    let bad2 = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["check", &gtl, "--drills", &gtl])
        .output()
        .unwrap();
    assert!(!bad2.status.success());
    assert!(String::from_utf8_lossy(&bad2.stderr).contains("not a drill file"));
}

// ---------------------------------------------------------------- pens and speed

fn hole(tool: u8, x: f64, y: f64) -> Hole {
    Hole {
        tool,
        x_mm: x,
        y_mm: y,
        diameter_mm: 0.8,
    }
}

#[test]
fn a_pen_map_is_read_and_written_back() {
    let m = parse_pen_map("1:3, 2:1").unwrap();
    assert_eq!(m, vec![(1, 3), (2, 1)]);
    assert_eq!(parse_pen_map(&pen_map_text(&m)).unwrap(), m);
    assert!(parse_pen_map("").unwrap().is_empty());
    for bad in ["x", "1:", ":2", "300:1", "1:0", "1:9", "1:1,1:2", "1:2:3"] {
        assert!(parse_pen_map(bad).is_err(), "{bad}");
    }
}

#[test]
fn unmapped_tools_take_the_lowest_free_pens() {
    let p = pens_for(&[5, 3, 5, 9], &[(9, 1)]).unwrap();
    assert!(p.contains(&(9, 1)));
    assert!(p.contains(&(3, 2)) && p.contains(&(5, 3)), "{p:?}");
    let many: Vec<u8> = (1..=9).collect();
    assert!(pens_for(&many, &[]).is_err(), "more than 8 pens");
}

#[test]
fn the_holes_are_plotted_with_the_pen_of_their_tool() {
    let holes = [hole(1, 1.0, 1.0), hole(2, 2.0, 1.0), hole(1, 3.0, 1.0)];
    let plain = write_holes(&holes, &[]).unwrap();
    assert!(
        plain.starts_with("IN;\nSP1;\nPU40,40;PD;PU;\nPU120,40;PD;PU;\nSP2;\nPU80,40;PD;PU;\n"),
        "{plain}"
    );
    let mapped = write_holes(&holes, &[(2, 1), (1, 4)]).unwrap();
    assert!(
        mapped.starts_with("IN;\nSP1;\nPU80,40;PD;PU;\nSP4;\nPU40,40;PD;PU;\nPU120,40;PD;PU;\n"),
        "{mapped}"
    );
    assert!(mapped.ends_with("PU0,0;SP0;\n"));
    assert!(write_holes(&[], &[]).is_err());
    assert!(write_holes(&[hole(1, f64::NAN, 0.0)], &[]).is_err());
}

#[test]
fn the_pen_speed_is_only_written_when_asked_for() {
    let plot = "IN;\nSP1;\nPU0,0;\nPD40,40;\nPU0,0;SP0;\n";
    assert_eq!(set_pen_speed(plot, 0.0).unwrap(), plot);
    let fast = set_pen_speed(plot, 7.5).unwrap();
    assert!(fast.starts_with("IN;\nVS7.5;\nSP1;"), "{fast}");
    assert!(set_pen_speed(plot, 10.0).unwrap().contains("VS10;"));
    assert!(set_pen_speed(plot, -1.0).is_err());
    assert!(set_pen_speed(plot, f64::NAN).is_err());
    assert!(set_pen_speed(plot, 1000.0).is_err());
}

// ---------------------------------------------------------------- DXF

#[test]
fn the_passes_are_written_as_a_dxf_drawing() {
    let pass = Pass {
        paths: vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        open: vec![vec![(5.0, 5.0), (6.0, 5.0)]],
        ..Pass::default()
    };
    let rub = Pass {
        open: vec![vec![(0.0, 2.0), (3.0, 2.0)]],
        rubout: true,
        ..Pass::default()
    };
    let text = dxf::write(&[pass, rub]).unwrap();
    assert_eq!(detect(&text), FileKind::Dxf);
    assert_eq!(text.matches("POLYLINE").count(), 3);
    assert!(text.contains("PASS_1") && text.contains("RUBOUT_2"));
    assert!(
        text.contains("70\n1\n"),
        "the closed path is a closed polyline"
    );
    assert!(text.ends_with("0\nEOF\n"));
    assert!(dxf::write(&[]).is_err());
    let bad = Pass {
        open: vec![vec![(0.0, f64::NAN), (1.0, 0.0)]],
        ..Pass::default()
    };
    assert!(dxf::write(&[bad]).is_err());
}

#[test]
fn the_command_line_writes_dxf_plots_of_holes_and_clipped_fills() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
    let exe = env!("CARGO_BIN_EXE_eh3d-isocam");
    let dxf_path = format!("{dir}/kinds_out.dxf");
    let o = Command::new(exe)
        .args(["isolate", sample, &dxf_path])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let text = std::fs::read_to_string(&dxf_path).unwrap();
    assert!(text.contains("POLYLINE") && text.ends_with("EOF\n"));

    let drl = format!("{dir}/kinds_in.drl");
    std::fs::write(&drl, EXCELLON).unwrap();
    let plt = format!("{dir}/kinds_holes.plt");
    let o = Command::new(exe)
        .args([
            "drills",
            &drl,
            &plt,
            "--hpgl-pen-map",
            "1:4",
            "--hpgl-pen-speed-cm-s",
            "5",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let plot = std::fs::read_to_string(&plt).unwrap();
    assert!(
        plot.contains("VS5;") && plot.contains("SP4;") && plot.contains("PD;PU;"),
        "{plot}"
    );

    let nc = format!("{dir}/kinds_fill.nc");
    let o = Command::new(exe)
        .args([
            "rubout-area",
            &nc,
            sample,
            "--area",
            "0,0,18,18",
            "--rubout-lines-tool-mm",
            "0.8",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let clipped = std::fs::read_to_string(&nc).unwrap();
    let boxed = format!("{dir}/kinds_box.nc");
    let o = Command::new(exe)
        .args([
            "rubout-area",
            &boxed,
            "--area",
            "0,0,18,18",
            "--rubout-lines-tool-mm",
            "0.8",
        ])
        .output()
        .unwrap();
    assert!(o.status.success());
    let all = std::fs::read_to_string(&boxed).unwrap();
    assert!(
        clipped.len() < all.len(),
        "the fill cut to the copper is shorter than the whole box"
    );
}
