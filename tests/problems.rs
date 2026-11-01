//! Problem zones (sharp inner corners, tracks between pads that are too near), the tolerance for small conflicts and the distance of the alignment holes.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::isolate::close_pieces;
use eh3d_isocam::problems::{corner_residue_mm2, problem_zones, ProblemKind, ProblemRules};
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
