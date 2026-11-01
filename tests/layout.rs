//! Turning the board over, panels and alignment holes, the design rules, and the command line.

use eh3d_gerberviewer::scene::{Bounds, Scene, Shape};
use eh3d_isocam::drc::{check, Kind, Rules};
use eh3d_isocam::drill::{read_drills, Hole};
use eh3d_isocam::transform::{
    alignment_holes, lay_out, mirror_holes, mirror_scene, panel_holes, panel_offsets, panel_scene,
    rotate_scene, Axis, Board, Layout,
};
use std::process::Command;

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn track(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Shape {
    Shape::Stroke { x0, y0, x1, y1, w }
}

fn hole(x: f64, y: f64, d: f64) -> Hole {
    Hole {
        tool: 1,
        x_mm: x,
        y_mm: y,
        diameter_mm: d,
    }
}

fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    Bounds {
        min_x: x0,
        min_y: y0,
        max_x: x1,
        max_y: y1,
    }
}

fn rules() -> Rules {
    Rules {
        min_width_mm: 0.2,
        min_clearance_mm: 0.2,
        annular_ring_mm: 0.15,
        tool_mm: 0.2,
        holes_on_copper: false,
    }
}

// ---- IC-4 -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn turning_over_twice_gives_the_board_back_and_once_swaps_the_sides() {
    let s = scene(vec![
        disc(1.0, 2.0, 0.5),
        Shape::Rect {
            x: 3.0,
            y: 4.0,
            w: 2.0,
            h: 1.0,
        },
        Shape::Obround {
            x: 5.0,
            y: 1.0,
            w: 1.0,
            h: 3.0,
        },
        track(0.0, 0.0, 4.0, 3.0, 0.3),
        Shape::Region {
            pts: vec![(0.0, 0.0), (2.0, 0.0), (1.0, 3.0)],
        },
    ]);
    let once = mirror_scene(&s, Axis::Vertical, 5.0);
    assert_eq!(once.shapes[0], disc(9.0, 2.0, 0.5));
    assert_eq!(
        once.shapes[1],
        Shape::Rect {
            x: 7.0,
            y: 4.0,
            w: 2.0,
            h: 1.0
        }
    );
    assert_eq!(once.shapes[3], track(10.0, 0.0, 6.0, 3.0, 0.3));
    assert_ne!(once, s);
    assert_eq!(mirror_scene(&once, Axis::Vertical, 5.0), s);
    let flat = mirror_scene(&s, Axis::Horizontal, 2.0);
    assert_eq!(flat.shapes[0], disc(1.0, 2.0, 0.5));
    assert_eq!(
        flat.shapes[1],
        Shape::Rect {
            x: 3.0,
            y: 0.0,
            w: 2.0,
            h: 1.0
        }
    );
    assert_eq!(mirror_scene(&flat, Axis::Horizontal, 2.0), s);
    let h = mirror_holes(&[hole(3.0, 4.0, 0.8)], Axis::Vertical, 5.0);
    assert_eq!((h[0].x_mm, h[0].y_mm, h[0].diameter_mm), (7.0, 4.0, 0.8));
}

#[test]
fn quarter_turns_move_the_shapes_and_swap_the_sides_of_pads() {
    let s = scene(vec![
        Shape::Rect {
            x: 1.0,
            y: 0.0,
            w: 4.0,
            h: 2.0,
        },
        disc(2.0, 0.0, 0.5),
    ]);
    let q1 = rotate_scene(&s, 1, (0.0, 0.0));
    assert_eq!(
        q1.shapes[0],
        Shape::Rect {
            x: 0.0,
            y: 1.0,
            w: 2.0,
            h: 4.0
        }
    );
    assert_eq!(q1.shapes[1], disc(0.0, 2.0, 0.5));
    let q2 = rotate_scene(&s, 2, (0.0, 0.0));
    assert_eq!(q2.shapes[1], disc(-2.0, 0.0, 0.5));
    let q4 = rotate_scene(
        &rotate_scene(
            &rotate_scene(&rotate_scene(&s, 1, (1.0, 1.0)), 1, (1.0, 1.0)),
            1,
            (1.0, 1.0),
        ),
        1,
        (1.0, 1.0),
    );
    assert_eq!(q4, s);
    assert_eq!(
        rotate_scene(&s, 3, (0.0, 0.0)).shapes[1],
        disc(0.0, -2.0, 0.5)
    );
    assert_eq!(rotate_scene(&s, 0, (5.0, 5.0)), s);
}

#[test]
fn a_panel_repeats_the_board_with_a_gap() {
    let b = bounds(0.0, 0.0, 10.0, 5.0);
    let o = panel_offsets(&b, 2, 3, 2.0).unwrap();
    assert_eq!(o.len(), 6);
    assert_eq!(o[0], (0.0, 0.0));
    assert_eq!(o[1], (12.0, 0.0));
    assert_eq!(o[2], (0.0, 7.0));
    assert_eq!(o[5], (12.0, 14.0));
    let s = scene(vec![disc(1.0, 1.0, 0.5)]);
    let p = panel_scene(&s, &o);
    assert_eq!(p.shapes.len(), 6);
    assert_eq!(p.shapes[5], disc(13.0, 15.0, 0.5));
    let h = panel_holes(&[hole(1.0, 1.0, 0.8)], &o);
    assert_eq!((h.len(), h[3].x_mm, h[3].y_mm), (6, 13.0, 8.0));
    for (c, r, g) in [
        (0, 1, 1.0),
        (1, 0, 1.0),
        (11, 10, 1.0),
        (1, 1, -1.0),
        (1, 1, f64::NAN),
    ] {
        assert!(panel_offsets(&b, c, r, g).is_err(), "{c} {r} {g}");
    }
    assert!(panel_offsets(&Bounds::empty(), 1, 1, 1.0).is_err());
}

#[test]
fn the_alignment_holes_are_where_the_board_finds_them_after_it_is_turned_over() {
    let b = bounds(0.0, 0.0, 40.0, 20.0);
    for (axis, at) in [(Axis::Vertical, 20.0), (Axis::Horizontal, 10.0)] {
        for count in [2, 3] {
            let holes = alignment_holes(&b, 3.0, 5.0, count, axis, 90).unwrap();
            assert_eq!(holes.len(), count);
            let turned = mirror_holes(&holes, axis, at);
            // the same holes, in the same places, whichever way the board lies
            for t in &turned {
                assert!(
                    holes
                        .iter()
                        .any(|h| (h.x_mm - t.x_mm).abs() < 1e-9 && (h.y_mm - t.y_mm).abs() < 1e-9),
                    "{axis:?} {count}: {t:?} is not one of {holes:?}"
                );
            }
            // all of them are outside the board
            for h in &holes {
                assert!(h.x_mm < 0.0 || h.x_mm > 40.0 || h.y_mm < 0.0 || h.y_mm > 20.0);
                assert_eq!((h.tool, h.diameter_mm), (90, 3.0));
            }
        }
    }
    // the third hole tells the board from the same board turned half a turn
    let three = alignment_holes(&b, 3.0, 5.0, 3, Axis::Vertical, 90).unwrap();
    let half_turn: Vec<(f64, f64)> = three
        .iter()
        .map(|h| (40.0 - h.x_mm, 20.0 - h.y_mm))
        .collect();
    assert!(half_turn.iter().any(|p| !three
        .iter()
        .any(|h| (h.x_mm - p.0).abs() < 1e-9 && (h.y_mm - p.1).abs() < 1e-9)));
    assert!(alignment_holes(&Bounds::empty(), 3.0, 5.0, 2, Axis::Vertical, 1).is_err());
    for (d, m, n) in [
        (0.0, 5.0, 2),
        (3.0, 0.0, 2),
        (f64::NAN, 5.0, 2),
        (3.0, 5.0, 1),
        (3.0, 5.0, 4),
    ] {
        assert!(
            alignment_holes(&b, d, m, n, Axis::Vertical, 1).is_err(),
            "{d} {m} {n}"
        );
    }
}

#[test]
fn a_board_is_laid_out_with_its_outline_holes_and_copper_together() {
    let board = Board {
        scene: scene(vec![disc(3.0, 5.0, 1.0)]),
        holes: vec![hole(3.0, 5.0, 0.8)],
        outline: vec![vec![(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]],
    };
    let (turned, whole) = lay_out(
        &board,
        &Layout {
            mirror: Some(Axis::Vertical),
            ..Layout::default()
        },
    )
    .unwrap();
    assert_eq!(turned.scene.shapes[0], disc(17.0, 5.0, 1.0));
    assert_eq!((turned.holes[0].x_mm, turned.holes[0].y_mm), (17.0, 5.0));
    assert!(turned.outline[0].contains(&(0.0, 0.0)) && turned.outline[0].contains(&(20.0, 10.0)));
    assert_eq!((whole.min_x, whole.max_x), (0.0, 20.0));
    // a quarter turn makes it 10 wide and 20 high, about the same middle
    let (q, qb) = lay_out(
        &board,
        &Layout {
            quarters: 1,
            ..Layout::default()
        },
    )
    .unwrap();
    assert!(
        (qb.width() - 10.0).abs() < 1e-9 && (qb.height() - 20.0).abs() < 1e-9,
        "{qb:?}"
    );
    assert_eq!(q.scene.shapes.len(), 1);
    // a panel of 2 x 2 is twice as wide and twice as high, plus the gap
    let (p, pb) = lay_out(
        &board,
        &Layout {
            cols: 2,
            rows: 2,
            gap_mm: 3.0,
            ..Layout::default()
        },
    )
    .unwrap();
    assert_eq!(
        (p.scene.shapes.len(), p.holes.len(), p.outline.len()),
        (4, 4, 4)
    );
    assert!(
        (pb.width() - 43.0).abs() < 1e-9 && (pb.height() - 23.0).abs() < 1e-9,
        "{pb:?}"
    );
    assert!(lay_out(
        &board,
        &Layout {
            quarters: 4,
            ..Layout::default()
        }
    )
    .is_err());
    assert!(lay_out(&Board::default(), &Layout::default()).is_err());
    // with no outline the box comes from the copper, and with no copper from the holes
    let only_holes = Board {
        holes: vec![hole(1.0, 1.0, 0.8), hole(5.0, 3.0, 0.8)],
        ..Board::default()
    };
    assert_eq!(only_holes.reference(), bounds(1.0, 1.0, 5.0, 3.0));
}

// ---- IC-6 -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn copper_closer_than_the_rule_is_reported_where_it_is() {
    let near = scene(vec![disc(0.0, 0.0, 1.0), disc(2.15, 0.0, 1.0)]);
    let found = check(&near, &[], &rules(), 0.02).unwrap();
    let clearance: Vec<_> = found.iter().filter(|v| v.kind == Kind::Clearance).collect();
    assert_eq!(clearance.len(), 1, "{found:?}");
    assert!(
        (clearance[0].x - 1.075).abs() < 0.2 && clearance[0].y.abs() < 0.2,
        "{:?}",
        clearance[0]
    );
    assert!(clearance[0].describe().contains("clearance"));
    let fine = scene(vec![disc(0.0, 0.0, 1.0), disc(2.5, 0.0, 1.0)]);
    assert!(check(&fine, &[], &rules(), 0.02).unwrap().is_empty());
}

#[test]
fn a_gap_the_tool_cannot_enter_is_told_apart_from_one_the_rule_allows() {
    let gap = scene(vec![disc(0.0, 0.0, 1.0), disc(2.3, 0.0, 1.0)]);
    let r = Rules {
        tool_mm: 0.4,
        ..rules()
    };
    let found = check(&gap, &[], &r, 0.02).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, Kind::ClearanceForTool);
    // a tool no wider than the rule adds nothing
    assert!(check(&gap, &[], &rules(), 0.02).unwrap().is_empty());
}

#[test]
fn copper_narrower_than_the_rule_is_reported_and_a_corner_is_not() {
    // a track of 0.1 mm, one of 0.3 mm, a square pad and a round pad
    let thin = scene(vec![track(0.0, 0.0, 6.0, 0.0, 0.1)]);
    let found = check(&thin, &[], &rules(), 0.02).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, Kind::Width);
    assert!(
        (found[0].x - 3.0).abs() < 0.2 && found[0].y.abs() < 0.1,
        "{:?}",
        found[0]
    );
    let fine = scene(vec![
        track(0.0, 0.0, 6.0, 0.0, 0.3),
        Shape::Rect {
            x: 10.0,
            y: 0.0,
            w: 2.0,
            h: 2.0,
        },
        disc(15.0, 0.0, 1.0),
    ]);
    assert!(check(&fine, &[], &rules(), 0.02).unwrap().is_empty());
    // a neck between two pads
    let neck = scene(vec![
        Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 2.0,
            h: 2.0,
        },
        track(1.0, 0.0, 4.0, 0.0, 0.1),
        Shape::Rect {
            x: 5.0,
            y: 0.0,
            w: 2.0,
            h: 2.0,
        },
    ]);
    let found = check(&neck, &[], &rules(), 0.02).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].x > 1.0 && found[0].x < 4.0);
}

#[test]
fn the_ring_of_copper_round_a_hole_is_checked() {
    let pad = scene(vec![disc(0.0, 0.0, 0.8)]);
    // a hole of 0.8 leaves 0.4 mm of copper
    assert!(check(&pad, &[hole(0.0, 0.0, 0.8)], &rules(), 0.02)
        .unwrap()
        .is_empty());
    // a hole of 1.4 leaves 0.1 mm, less than 0.15
    let found = check(&pad, &[hole(0.0, 0.0, 1.4)], &rules(), 0.02).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, Kind::AnnularRing);
    assert_eq!((found[0].x, found[0].y), (0.0, 0.0));
    // a hole off the middle of its pad
    let found = check(&pad, &[hole(0.3, 0.0, 0.8)], &rules(), 0.02).unwrap();
    assert_eq!(found.len(), 1);
    // a hole in the bare board is not looked at, unless the rule says every hole is on copper
    let bare = hole(30.0, 30.0, 3.0);
    assert!(check(&pad, &[bare], &rules(), 0.02).unwrap().is_empty());
    let strict = Rules {
        holes_on_copper: true,
        ..rules()
    };
    let found = check(&pad, &[bare], &strict, 0.02).unwrap();
    assert_eq!(found[0].kind, Kind::HoleOffCopper);
    // a hole with no diameter cannot be checked, and the rule off lets it pass
    let e = check(&pad, &[hole(0.0, 0.0, 0.0)], &rules(), 0.02).unwrap_err();
    assert!(e.contains("no diameter"), "{e}");
    let no_ring = Rules {
        annular_ring_mm: 0.0,
        ..rules()
    };
    assert!(check(&pad, &[hole(0.0, 0.0, 0.0)], &no_ring, 0.02)
        .unwrap()
        .is_empty());
}

#[test]
fn the_rules_are_checked_and_empty_layers_do_not_panic() {
    let pad = scene(vec![disc(0.0, 0.0, 1.0)]);
    for r in [
        Rules {
            min_width_mm: 0.0,
            ..rules()
        },
        Rules {
            min_clearance_mm: f64::NAN,
            ..rules()
        },
        Rules {
            annular_ring_mm: -1.0,
            ..rules()
        },
        Rules {
            tool_mm: 50.0,
            ..rules()
        },
    ] {
        assert!(check(&pad, &[], &r, 0.02).is_err(), "{r:?}");
    }
    assert!(
        check(&pad, &[], &rules(), 0.5).is_err(),
        "a cell too coarse for the rules"
    );
    assert!(check(&pad, &[], &rules(), 0.0).is_err());
    assert!(check(&scene(vec![]), &[], &rules(), 0.02).is_err());
    let _ = check(
        &scene(vec![Shape::Region { pts: vec![] }]),
        &[],
        &rules(),
        0.02,
    );
}

// ---- the command line -----------------------------------------------------------------------------------------------------------------------------------

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

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");

#[test]
fn the_command_line_writes_and_reads_the_configuration() {
    let (ok, out, _) = run(&["config"]);
    assert!(ok);
    assert_eq!(
        eh3d_isocam::config::Settings::parse(&out).unwrap(),
        eh3d_isocam::config::Settings::default()
    );
    let file = tmp("isocam_test.cfg");
    std::fs::write(&file, "fine_tool_mm = 0.15\ndirection = conventional\n").unwrap();
    let (ok, out, _) = run(&["config", "--config", &file, "--ordering", "optimized"]);
    assert!(ok);
    assert!(
        out.contains("fine_tool_mm = 0.15")
            && out.contains("direction = conventional")
            && out.contains("ordering = optimized")
    );
    let (ok, _, err) = run(&["config", "--config", &tmp("not_there.cfg")]);
    assert!(!ok && err.contains("not_there.cfg"));
    let (ok, _, err) = run(&["config", "--nonsense", "1"]);
    assert!(!ok && err.contains("unknown setting"), "{err}");
    let (ok, _, err) = run(&["config", "--fine-tool-mm"]);
    assert!(!ok && err.contains("needs a value"));
}

#[test]
fn copper_that_is_not_drawn_is_not_isolated_unless_forced() {
    let layer = tmp("isocam_macro.gtl");
    std::fs::write(
        &layer,
        "%FSLAX24Y24*%\n%MOMM*%\n%AMRR*1,1,1.6,0,0*%\n%ADD10C,0.3*%\n%ADD13RR*%\nD10*\nX20000Y20000D02*\nX200000Y20000D01*\nD13*\nX100000Y100000D03*\nM02*\n",
    )
    .unwrap();
    let out = tmp("isocam_macro.nc");
    let (ok, _, err) = run(&["isolate", &layer, &out]);
    assert!(!ok && err.contains("not drawn"), "{err}");
    let (ok, _, err) = run(&["isolate", &layer, &out, "--force"]);
    assert!(ok, "{err}");
}

#[test]
fn isolate_info_and_drc_run_on_the_sample_layer() {
    let (ok, out, err) = run(&[
        "info",
        SAMPLE,
        "--coarse-tool-mm",
        "0.4",
        "--rubout-tool-mm",
        "0.6",
        "--ordering",
        "optimized",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("pass 1") && out.contains("ring") && out.contains("travel through the air"),
        "{out}"
    );
    for ext in ["nc", "plt"] {
        let file = tmp(&format!("isocam_sample.{ext}"));
        let (ok, out, err) = run(&[
            "isolate",
            SAMPLE,
            &file,
            "--coarse-tool-mm",
            "0.4",
            "--trim-fine",
            "no",
            "--direction",
            "conventional",
            "--start-x-mm",
            "30",
        ]);
        assert!(ok, "{err}");
        assert!(out.contains("written"), "{out}");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.len() > 100);
        if ext == "plt" {
            assert!(
                text.starts_with("IN;\n") && text.contains("SP1;") && text.contains("SP2;"),
                "{text}"
            );
        } else {
            assert!(text.contains("M0\n") && text.contains("(change to a tool of"));
        }
    }
    let (ok, _, err) = run(&["isolate", SAMPLE]);
    assert!(!ok && err.contains("output file"));
    let (_, out, _) = run(&["drc", SAMPLE]);
    assert!(out.contains("rules") || out.contains("near ("), "{out}");
    let (ok, _, err) = run(&["isolate", tmp("missing.gtl").as_str(), "x.nc"]);
    assert!(!ok && err.contains("missing.gtl"));
    let (ok, _, err) = run(&["isolate", SAMPLE, &tmp("o.nc"), "--direction", "up"]);
    assert!(!ok && err.contains("climb, conventional"));
    let (ok, _, err) = run(&["frobnicate"]);
    assert!(!ok && err.contains("unknown command"));
}

#[test]
fn drills_are_read_turned_over_and_given_alignment_holes() {
    let input = tmp("isocam_in.ncd");
    std::fs::write(
        &input,
        "M48\nINCH\nT1C0.0315\n%\nT1\nX1000Y1000\nX3000Y2000\nM30\n",
    )
    .unwrap();
    let outline = tmp("isocam_outline.gko");
    std::fs::write(&outline, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX1000000Y0D01*\nX1000000Y500000D01*\nX0Y500000D01*\nX0Y0D01*\nM02*\n").unwrap();
    let out = tmp("isocam_out.ncd");
    let (ok, text, err) = run(&[
        "drills",
        &input,
        &out,
        "--outline",
        &outline,
        "--mirror",
        "vertical",
        "--align",
        "--alignment-holes",
        "3",
    ]);
    assert!(ok, "{err}");
    assert!(text.contains("5 holes"), "{text}");
    let back = read_drills(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(back.holes.len(), 5);
    // the board is 100 x 50 mm: the hole at x = 25.4 is at 74.6 once turned over
    assert!(
        back.holes
            .iter()
            .any(|h| (h.x_mm - 74.6).abs() < 0.05 && (h.y_mm - 25.4).abs() < 0.05),
        "{:?}",
        back.holes
    );
    // the alignment holes are outside the board and have their own tool
    let extra: Vec<_> = back.holes.iter().filter(|h| h.tool == 90).collect();
    assert_eq!(extra.len(), 3);
    assert!(extra
        .iter()
        .all(|h| h.x_mm < 0.0 || h.x_mm > 100.0 || h.y_mm < 0.0 || h.y_mm > 50.0));
    // a panel of two
    let (ok, text, err) = run(&[
        "drills",
        &input,
        &out,
        "--outline",
        &outline,
        "--panel-cols",
        "2",
    ]);
    assert!(ok, "{err}");
    assert!(text.contains("4 holes"));
    let (ok, _, err) = run(&["drills", &tmp("nope.ncd"), &out]);
    assert!(!ok && err.contains("nope.ncd"));
}

#[test]
fn the_board_outline_is_routed_to_a_plot_file_and_a_drill_file() {
    let outline = tmp("isocam_route.gko");
    std::fs::write(&outline, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n").unwrap();
    for ext in ["plt", "ncd"] {
        let file = tmp(&format!("isocam_route.{ext}"));
        let (ok, out, err) = run(&["route", &outline, &file, "--route-tool-mm", "2"]);
        assert!(ok, "{err}");
        assert!(out.contains("1 paths"), "{out}");
        let text = std::fs::read_to_string(&file).unwrap();
        if ext == "ncd" {
            assert!(text.contains("M15") && text.contains("T1C0.0787"), "{text}");
        } else {
            assert!(text.contains("PD"));
        }
    }
    let (ok, _, err) = run(&["route", &outline]);
    assert!(!ok && err.contains("output file"));
}
