//! The shape of the cut round the board: rounded corners, small cutouts left out, and the table of the machine, through the command line and the shared job.

use eh3d_gerberviewer::scene::Bounds;
use eh3d_isocam::board::{route_job, shape_outline};
use eh3d_isocam::config::Settings;
use std::process::Command;

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"))
}

/// A board of 50 x 30 mm with a square cutout of 4 mm inside it, as a Gerber outline layer.
const BOARD_WITH_CUTOUT: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nX200000Y100000D02*\nX240000Y100000D01*\nX240000Y140000D01*\nX200000Y140000D01*\nX200000Y100000D01*\nM02*\n";

fn board() -> Vec<Vec<(f64, f64)>> {
    vec![
        vec![(0.0, 0.0), (50.0, 0.0), (50.0, 30.0), (0.0, 30.0)],
        vec![(20.0, 10.0), (24.0, 10.0), (24.0, 14.0), (20.0, 14.0)],
    ]
}

fn whole() -> Bounds {
    Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 50.0,
        max_y: 30.0,
    }
}

#[test]
fn both_settings_are_off_by_default_and_leave_the_outline_as_it_was() {
    let s = Settings::default();
    assert_eq!(s.outline_corner_radius_mm, 0.0);
    assert_eq!(s.outline_min_cutout_mm, 0.0);
    let (paths, dropped, rounded, limited) = shape_outline(&board(), &s);
    assert_eq!(paths, board());
    assert_eq!((dropped, rounded, limited), (0, 0, 0));
}

#[test]
fn the_shape_follows_the_settings() {
    let mut s = Settings::default();
    s.set("outline_min_cutout_mm", "5").unwrap();
    let (paths, dropped, rounded, _) = shape_outline(&board(), &s);
    assert_eq!((paths.len(), dropped, rounded), (1, 1, 0));
    s.set("outline_corner_radius_mm", "3").unwrap();
    let (paths, dropped, rounded, limited) = shape_outline(&board(), &s);
    assert_eq!((paths.len(), dropped, rounded, limited), (1, 1, 4, 0));
    assert!(paths[0].len() > 4 * 5);
    // the cutout is rounded too when it stays: its sides of 4 mm take a radius of 3 down to 2
    s.set("outline_min_cutout_mm", "0").unwrap();
    let (paths, dropped, rounded, limited) = shape_outline(&board(), &s);
    assert_eq!((paths.len(), dropped, rounded, limited), (2, 0, 8, 4));
}

#[test]
fn rounding_the_corners_shortens_the_cut_and_a_cutout_that_is_left_out_leaves_no_path() {
    let sharp = route_job(&board(), &whole(), &Settings::default(), false).unwrap();
    let mut s = Settings::default();
    s.set("outline_corner_radius_mm", "5").unwrap();
    let round = route_job(&board(), &whole(), &s, false).unwrap();
    assert_eq!((round.corners_rounded, round.corners_limited), (8, 4));
    // an arc of radius R takes (8 - 2 pi) R off the way round a corner of a convex board: well over 4 mm for the four corners of the board alone
    assert!(
        round.cut_mm < sharp.cut_mm - 4.0,
        "{} against {}",
        round.cut_mm,
        sharp.cut_mm
    );
    let mut t = Settings::default();
    t.set("outline_min_cutout_mm", "5").unwrap();
    let without = route_job(&board(), &whole(), &t, false).unwrap();
    assert_eq!(without.cutouts_dropped, 1);
    assert!(
        without.paths < sharp.paths,
        "{} against {}",
        without.paths,
        sharp.paths
    );
    assert_eq!(sharp.cutouts_dropped + sharp.corners_rounded, 0);
}

#[test]
fn the_cut_is_checked_against_the_table_of_the_machine() {
    let mut s = Settings::default();
    // no table described: nothing is checked
    assert!(route_job(&board(), &whole(), &s, false)
        .unwrap()
        .outside_table
        .is_none());
    s.set("machine_x_mm", "100").unwrap();
    s.set("machine_y_mm", "60").unwrap();
    // the tool goes half a tool beyond the edge of a board that sits on the origin
    let at_origin = route_job(&board(), &whole(), &s, false).unwrap();
    assert!(
        at_origin
            .outside_table
            .as_deref()
            .is_some_and(|w| w.contains("outside the table")),
        "{:?}",
        at_origin.outside_table
    );
    // moved on the table, it fits
    s.set("offset_x_mm", "10").unwrap();
    s.set("offset_y_mm", "10").unwrap();
    assert!(route_job(&board(), &whole(), &s, false)
        .unwrap()
        .outside_table
        .is_none());
    // and a table smaller than the board refuses it again
    s.set("machine_x_mm", "40").unwrap();
    assert!(route_job(&board(), &whole(), &s, false)
        .unwrap()
        .outside_table
        .is_some());
}

#[test]
fn route_says_what_it_did_to_the_outline_and_refuses_a_cut_off_the_table() {
    let outline = tmp("isocam_shape.gko");
    std::fs::write(&outline, BOARD_WITH_CUTOUT).unwrap();
    let plain = tmp("isocam_shape_plain.ncd");
    let (ok, out, err) = run(&["route", &outline, &plain]);
    assert!(ok, "{err}");
    assert!(out.contains("2 paths"), "{out}");
    assert!(
        !err.contains("corners") && !err.contains("cutouts"),
        "{err}"
    );
    let shaped = tmp("isocam_shape_round.ncd");
    let (ok, out, err) = run(&[
        "route",
        &outline,
        &shaped,
        "--outline-corner-radius-mm",
        "5",
        "--outline-min-cutout-mm",
        "5",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("1 paths"), "{out}");
    assert!(err.contains("1 cutouts smaller than"), "{err}");
    assert!(err.contains("4 corners rounded with a radius of"), "{err}");
    assert_ne!(
        std::fs::read_to_string(&plain).unwrap(),
        std::fs::read_to_string(&shaped).unwrap()
    );
    // a radius that is too big for the sides is cut down and said
    let (ok, _, err) = run(&[
        "route",
        &outline,
        &shaped,
        "--outline-corner-radius-mm",
        "40",
    ]);
    assert!(ok, "{err}");
    assert!(err.contains("smaller radius"), "{err}");
    // a table that the cut leaves is refused, and --force writes it with a warning
    let table = tmp("isocam_shape_table.ncd");
    let small = ["--machine-x-mm", "40", "--machine-y-mm", "40"];
    let mut args = vec!["route", outline.as_str(), table.as_str()];
    args.extend(small);
    let (ok, _, err) = run(&args);
    assert!(
        !ok && err.contains("outside the table") && err.contains("--force"),
        "{err}"
    );
    args.push("--force");
    let (ok, _, err) = run(&args);
    assert!(
        ok && err.contains("warning:") && err.contains("outside the table"),
        "{err}"
    );
}

#[test]
fn the_limits_of_the_two_settings_are_refused_with_their_names() {
    let mut s = Settings::default();
    for (key, bad) in [
        ("outline_corner_radius_mm", "-1"),
        ("outline_corner_radius_mm", "101"),
        ("outline_min_cutout_mm", "-0.5"),
        ("outline_min_cutout_mm", "1001"),
    ] {
        s.set(key, bad).unwrap();
        let e = s.validate().unwrap_err();
        assert!(e.contains(key), "{e}");
        s.set(key, "0").unwrap();
    }
    s.set("outline_corner_radius_mm", "2.5").unwrap();
    s.set("outline_min_cutout_mm", "3").unwrap();
    s.validate().unwrap();
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    assert!(s.to_text().contains("outline_corner_radius_mm = 2.5"));
}
