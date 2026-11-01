//! Problem zones (sharp inner corners, tracks between pads that are too near), the tolerance for small conflicts and the distance of the alignment holes.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::isolate::{close_pieces, net_scene, rubout_lines};
use eh3d_isocam::problems::{
    corner_residue_mm2, problem_zones, thin_for_tolerance, ProblemKind, ProblemRules,
};
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

fn rules() -> ProblemRules {
    ProblemRules {
        tool_mm: 0.2,
        corner_angle_deg: 60.0,
        track_gap_factor: 1.5,
    }
}

#[test]
fn a_sharper_corner_leaves_more_copper_in_its_tip() {
    let r = 0.1;
    assert!(corner_residue_mm2(r, 30.0) > corner_residue_mm2(r, 60.0));
    assert!(corner_residue_mm2(r, 60.0) > corner_residue_mm2(r, 89.0));
    // a right angle leaves the well known 1 - pi/4 of r squared
    let right = corner_residue_mm2(r, 90.0);
    assert!((right - r * r * (1.0 - std::f64::consts::FRAC_PI_4)).abs() < 1e-9);
}

#[test]
fn a_narrow_notch_is_reported_and_a_right_angle_is_not() {
    // two tracks that start at one point and open 20 degrees: the empty wedge between them ends in a sharp tip
    let a = 10f64.to_radians();
    let v = scene(vec![
        track(0.0, 0.0, 6.0 * a.cos(), 6.0 * a.sin(), 0.3),
        track(0.0, 0.0, 6.0 * a.cos(), -6.0 * a.sin(), 0.3),
    ]);
    let found = problem_zones(&v, &rules(), 0.02).unwrap();
    let corners: Vec<_> = found
        .iter()
        .filter(|p| p.kind == ProblemKind::SharpCorner)
        .collect();
    assert_eq!(corners.len(), 1, "{found:?}");
    assert!(
        corners[0].x > 0.3 && corners[0].x < 2.0 && corners[0].y.abs() < 0.2,
        "{corners:?}"
    );
    // an L: the empty corner is 90 degrees
    let l = scene(vec![
        track(0.0, 0.0, 6.0, 0.0, 0.3),
        track(0.0, 0.0, 0.0, 6.0, 0.3),
    ]);
    assert!(problem_zones(&l, &rules(), 0.02).unwrap().is_empty());
    // the criterion is the setting: with 0 nothing is looked for
    let off = ProblemRules {
        corner_angle_deg: 0.0,
        ..rules()
    };
    assert!(problem_zones(&v, &off, 0.02).unwrap().is_empty());
}

#[test]
fn a_track_between_two_pads_that_are_too_near_is_reported() {
    // pads of 1.0 mm at y = +-0.45 and a track of 0.2 mm along y = 0: 0.15 mm of gap on each side
    let s = scene(vec![
        Shape::Disc {
            x: 3.0,
            y: 0.7,
            r: 0.5,
        },
        Shape::Disc {
            x: 3.0,
            y: -0.7,
            r: 0.5,
        },
        track(0.0, 0.0, 6.0, 0.0, 0.2),
    ]);
    let only_tracks = ProblemRules {
        corner_angle_deg: 0.0,
        ..rules()
    };
    let found = problem_zones(&s, &only_tracks, 0.02).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, ProblemKind::TrackBetweenPads);
    assert!((found[0].value - 0.1).abs() < 0.02, "{found:?}");
    assert!((found[0].x - 3.0).abs() < 0.2);
    // pads farther away than 1.5 tools from the track are fine
    let far = scene(vec![
        Shape::Disc {
            x: 3.0,
            y: 1.2,
            r: 0.5,
        },
        Shape::Disc {
            x: 3.0,
            y: -1.2,
            r: 0.5,
        },
        track(0.0, 0.0, 6.0, 0.0, 0.2),
    ]);
    assert!(problem_zones(&far, &only_tracks, 0.02).unwrap().is_empty());
    // a track that ends in a pad does not count that pad
    let own = scene(vec![
        Shape::Disc {
            x: 6.0,
            y: 0.0,
            r: 0.5,
        },
        Shape::Disc {
            x: 3.0,
            y: 0.7,
            r: 0.5,
        },
        track(0.0, 0.0, 6.0, 0.0, 0.2),
    ]);
    assert!(problem_zones(&own, &only_tracks, 0.02).unwrap().is_empty());
}

#[test]
fn the_problem_settings_are_read_checked_and_written_back() {
    let s = Settings::parse(
        "problem_corner_angle_deg = 45\nproblem_track_gap_factor = 2\nconflict_tolerance_mm = 0.03\n",
    )
    .unwrap();
    assert_eq!(s.problem_corner_angle_deg, 45.0);
    assert_eq!(s.problem_track_gap_factor, 2.0);
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    for bad in [
        "problem_corner_angle_deg = 5",
        "problem_corner_angle_deg = 95",
        "problem_track_gap_factor = -1",
        "conflict_tolerance_mm = -0.1",
        "conflict_tolerance_mm = 0.19",
    ] {
        assert!(Settings::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(Settings::default().conflict_tolerance_mm, 0.0);
}

#[test]
fn a_small_conflict_is_accepted_up_to_the_tolerance() {
    // two discs 0.16 mm apart and a tool of 0.2 mm: the tool is 0.04 mm too wide
    let s = scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 1.0,
        },
        Shape::Disc {
            x: 2.16,
            y: 0.0,
            r: 1.0,
        },
    ]);
    let none = Settings::default();
    let gap = none.conflict_gap_mm(0.2);
    assert_eq!(gap, 0.2);
    assert_eq!(close_pieces(&s, gap, 0.01).unwrap().len(), 1);
    let mut tolerant = Settings::default();
    tolerant.set("conflict_tolerance_mm", "0.06").unwrap();
    let gap = tolerant.conflict_gap_mm(0.2);
    assert!((gap - 0.14).abs() < 1e-12);
    assert!(close_pieces(&s, gap, 0.01).unwrap().is_empty());
    // a tolerance cannot take the tool to nothing
    assert_eq!(tolerant.conflict_gap_mm(0.04), 0.02);
}

#[test]
fn the_distance_of_the_alignment_holes_is_a_setting_with_5_mm_by_default() {
    assert_eq!(Settings::default().alignment_margin_mm, 5.0);
    let s = Settings::parse("alignment_margin_mm = 10\n").unwrap();
    assert_eq!(s.alignment_margin_mm, 10.0);
    assert!(Settings::parse("alignment_margin_mm = 0.1").is_err());
}

#[test]
fn check_prints_the_problem_zones_as_warnings() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = format!("{dir}/problems.gtl");
    // a Y of two thin tracks: the inner angle between the arms is sharp
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.3*%\nD10*\nX00000Y00000D02*\nX59000Y10400D01*\nX00000Y00000D02*\nX59000Y-10400D01*\nM02*\n";
    std::fs::write(&path, text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["check", &path])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("warning: inner corner sharper than 60"),
        "{stdout}"
    );
    assert!(out.status.success(), "a warning is not a failure: {stdout}");
    let quiet = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["check", &path, "--problem-corner-angle-deg", "0"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&quiet.stdout).contains("warning"));
}

#[test]
fn tracks_and_pads_thinner_than_twice_the_tolerance_plus_the_margin_are_found() {
    let s = scene(vec![
        track(0.0, 0.0, 5.0, 0.0, 0.1),
        track(0.0, 2.0, 5.0, 2.0, 0.3),
        Shape::Rect {
            x: 8.0,
            y: 1.0,
            w: 0.5,
            h: 0.12,
        },
        Shape::Disc {
            x: 9.0,
            y: 3.0,
            r: 0.03,
        },
    ]);
    // limit 2 x 0.05 + 0.05 = 0.15: the thin track and the thin side of the pad, not the round pad (it is not a line or a rectangle)
    let thin = thin_for_tolerance(&s, 0.05, 0.05).unwrap();
    assert_eq!(thin.len(), 2);
    assert!((thin[0].size_mm - 0.1).abs() < 1e-12 && (thin[1].size_mm - 0.12).abs() < 1e-12);
    assert!((thin[0].x - 2.5).abs() < 1e-12);
    // a track exactly at the limit is not thin
    assert!(thin_for_tolerance(&s, 0.025, 0.05).unwrap().is_empty());
    // no tolerance, nothing to say
    assert!(thin_for_tolerance(&s, 0.0, 0.05).unwrap().is_empty());
    assert!(thin_for_tolerance(&s, -0.1, 0.05).is_err());
    assert!(thin_for_tolerance(&s, f64::NAN, 0.05).is_err());
    assert!(thin_for_tolerance(&s, 0.05, -1.0).is_err());
    // the margin is a setting
    let set = Settings::parse("tolerance_warning_margin_mm = 0.2\nconflict_tolerance_mm = 0.05\n")
        .unwrap();
    assert_eq!(set.tolerance_warning_margin_mm, 0.2);
    assert!(Settings::parse("tolerance_warning_margin_mm = 2").is_err());
    assert!(Settings::parse("tolerance_warning_margin_mm = -0.1").is_err());
}

#[test]
fn check_warns_about_thin_copper_only_with_a_tolerance() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = format!("{dir}/thin.gtl");
    let text =
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX00000Y00000D02*\nX50000Y00000D01*\nM02*\n";
    std::fs::write(&path, text).unwrap();
    let run = |extra: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
            .args(["check", &path])
            .args(extra)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (ok, plain) = run(&[]);
    assert!(ok && !plain.contains("tolerance is high"), "{plain}");
    let (ok, warned) = run(&["--conflict-tolerance-mm", "0.05"]);
    assert!(ok, "a warning is not a failure: {warned}");
    assert!(
        warned.contains("warning: the tolerance is high against a track or pad 0.100 mm wide"),
        "{warned}"
    );
}

#[test]
fn one_net_is_picked_by_a_point_and_nothing_else() {
    let s = scene(vec![
        track(0.0, 0.0, 5.0, 0.0, 0.3),
        Shape::Disc {
            x: 5.0,
            y: 0.0,
            r: 0.5,
        },
        track(0.0, 3.0, 5.0, 3.0, 0.3),
    ]);
    let a = net_scene(&s, (2.0, 0.0), 0.05).unwrap();
    assert_eq!(a.shapes.len(), 2);
    let b = net_scene(&s, (2.0, 3.0), 0.05).unwrap();
    assert_eq!(b.shapes, vec![track(0.0, 3.0, 5.0, 3.0, 0.3)]);
    // a point a little off the copper still finds it, one far from every net does not
    assert_eq!(net_scene(&s, (2.0, 0.3), 0.05).unwrap().shapes.len(), 2);
    assert!(net_scene(&s, (2.0, 1.5), 0.05)
        .unwrap_err()
        .contains("no copper within"));
    assert!(net_scene(&s, (500.0, 500.0), 0.05).is_err());
    assert!(net_scene(&s, (f64::NAN, 0.0), 0.05).is_err());
    assert!(net_scene(&Scene::default(), (0.0, 0.0), 0.05).is_err());
    // a net touching another is one net
    let joined = scene(vec![
        track(0.0, 0.0, 5.0, 0.0, 0.3),
        track(5.0, 0.0, 5.0, 3.0, 0.3),
        track(5.0, 3.0, 9.0, 3.0, 0.3),
    ]);
    assert_eq!(
        net_scene(&joined, (9.0, 3.0), 0.05).unwrap().shapes.len(),
        3
    );
}

#[test]
fn isolate_with_a_net_cuts_round_that_net_alone_with_the_net_tool() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = format!("{dir}/net.gtl");
    let out = format!("{dir}/net.plt");
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.3*%\nD10*\nX00000Y00000D02*\nX50000Y00000D01*\nX00000Y30000D02*\nX50000Y30000D01*\nM02*\n";
    std::fs::write(&path, text).unwrap();
    let go = |extra: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
            .args(["isolate", &path, &out])
            .args(extra)
            .output()
            .unwrap();
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (ok, all, err) = go(&[]);
    assert!(ok, "{err}");
    assert!(all.contains("2 paths"), "{all}");
    let (ok, one, err) = go(&["--net", "2,0", "--net-tool-mm", "0.5"]);
    assert!(ok, "{err}");
    assert!(one.contains("1 passes, 1 paths"), "{one}");
    assert!(
        err.contains("note: the net at (2, 0) has 1 shapes"),
        "{err}"
    );
    let plot = std::fs::read_to_string(&out).unwrap();
    assert!(plot.contains("PD"), "{plot}");
    let (ok, _, err) = go(&["--net", "2,1.5"]);
    assert!(!ok && err.contains("no copper within"), "{err}");
    let (ok, _, err) = go(&["--net", "2"]);
    assert!(!ok && err.contains("--net needs 2 numbers"), "{err}");
}

#[test]
fn parallel_lines_clear_an_area_with_the_tool_reaching_its_edges() {
    // 10 x 5 mm, tool 1 mm, overlap 0.2: a pitch of 0.8 mm gives lines at 0.5, 1.3 ... 4.5
    let p = rubout_lines((0.0, 0.0, 10.0, 5.0), 1.0, 0.2, Axis::Horizontal).unwrap();
    assert_eq!(p.open.len(), 6);
    assert!(p.rubout && p.paths.is_empty() && p.diameter_mm == 1.0);
    for (k, run) in p.open.iter().enumerate() {
        assert_eq!(run.len(), 2);
        assert!((run[0].1 - (0.5 + 0.8 * k as f64)).abs() < 1e-9);
        assert!((run[0].1 - run[1].1).abs() < 1e-12);
        // back and forth, from half a tool inside one edge to half a tool inside the other
        let (a, b) = if k % 2 == 0 { (0.5, 9.5) } else { (9.5, 0.5) };
        assert!((run[0].0 - a).abs() < 1e-9 && (run[1].0 - b).abs() < 1e-9);
    }
    // vertical lines, corners given in any order
    let v = rubout_lines((10.0, 5.0, 0.0, 0.0), 1.0, 0.2, Axis::Vertical).unwrap();
    assert_eq!(v.open.len(), 13);
    assert!((v.open[0][0].0 - 0.5).abs() < 1e-9 && (v.open[12][0].0 - 9.5).abs() < 1e-9);
    // a tool as wide as the area across has one line
    let one = rubout_lines((0.0, 0.0, 10.0, 1.0), 1.0, 0.2, Axis::Horizontal).unwrap();
    assert_eq!(one.open.len(), 1);
    for (area, tool, overlap) in [
        ((0.0, 0.0, 1.0, 5.0), 1.0, 0.2),
        ((0.0, 0.0, 10.0, 0.5), 1.0, 0.2),
        ((0.0, 0.0, 10.0, 5.0), 0.05, 0.2),
        ((0.0, 0.0, 10.0, 5.0), 1.0, 0.95),
        ((0.0, 0.0, f64::NAN, 5.0), 1.0, 0.2),
        ((0.0, 0.0, 5000.0, 5000.0), 0.1, 0.0),
    ] {
        assert!(
            rubout_lines(area, tool, overlap, Axis::Horizontal).is_err(),
            "{area:?} {tool} {overlap}"
        );
    }
}

#[test]
fn the_new_settings_round_trip_and_their_limits_hold() {
    let mut s = Settings::default();
    for (k, v) in [
        ("alignment_pattern", "special"),
        ("alignment_holes", "6"),
        ("alignment_special_a_mm", "36"),
        ("alignment_special_b_mm", "55"),
        ("tab_count", "3"),
        ("tab_width_mm", "1.5"),
        ("tab_min_length_mm", "6"),
        ("tolerance_warning_margin_mm", "0.08"),
        ("net_tool_mm", "1.5"),
        ("rubout_lines_tool_mm", "0.6"),
        ("rubout_lines_direction", "vertical"),
    ] {
        s.set(k, v).unwrap();
    }
    s.validate().unwrap();
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    for (text, part) in [
        ("tab_count = 2\ntab_width_mm = 0.05", "tab_width_mm"),
        (
            "tab_count = 2\ntab_min_length_mm = 1.9",
            "tab_min_length_mm",
        ),
        ("tab_count = 99", "tab_count"),
        ("alignment_pattern = special\nalignment_holes = 3", "2 or 6"),
        (
            "alignment_pattern = special\nalignment_holes = 2\nalignment_special_a_mm = 0.5",
            "alignment_special_a_mm",
        ),
        ("alignment_pattern = triangle", "normal or special"),
        ("net_tool_mm = 0.001", "net_tool_mm"),
        ("rubout_lines_tool_mm = 50", "rubout_lines_tool_mm"),
        ("rubout_lines_direction = none", "horizontal or vertical"),
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?} gave {e:?}");
    }
    // the smallest tool is 0.02 mm, with or without a tolerance
    assert!(Settings::parse("fine_tool_mm = 0.01\nres_mm = 0.002").is_err());
    assert!(Settings::parse("fine_tool_mm = 0.02\nres_mm = 0.002").is_ok());
    assert!(Settings::parse("fine_tool_mm = 0.1\nconflict_tolerance_mm = 0.09").is_err());
}

#[test]
fn rubout_area_writes_the_lines_to_a_plot_file() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let out = format!("{dir}/area.plt");
    let run = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
            .args(args)
            .output()
            .unwrap();
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (ok, text, err) = run(&[
        "rubout-area",
        &out,
        "--area",
        "0,0,10,5",
        "--rubout-lines-tool-mm",
        "1",
    ]);
    assert!(ok, "{err}");
    assert!(text.contains("6 lines of 1.000 mm"), "{text}");
    assert!(std::fs::read_to_string(&out).unwrap().contains("PD"));
    let (ok, _, err) = run(&["rubout-area", &out]);
    assert!(!ok && err.contains("--area"), "{err}");
    let (ok, _, err) = run(&["rubout-area", &out, "--area", "0,0,10"]);
    assert!(!ok && err.contains("4 numbers"), "{err}");
}
