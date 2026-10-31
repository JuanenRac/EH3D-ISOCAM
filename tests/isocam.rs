use eh3d_gerberviewer::scene::{from_gerber, Scene, Shape};
use eh3d_isocam::bitmap::from_scene;
use eh3d_isocam::contour::simplify;
use eh3d_isocam::edt::{distance_mm, squared_distance};
use eh3d_isocam::gcode::{length, write, Job};
use eh3d_isocam::isolate::{close_pieces, isolate, Params};

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn radius_range(path: &[(f64, f64)], c: (f64, f64)) -> (f64, f64) {
    let r: Vec<f64> = path
        .iter()
        .map(|p| ((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt())
        .collect();
    (
        r.iter().copied().fold(f64::INFINITY, f64::min),
        r.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

#[test]
fn the_distance_transform_is_exact() {
    let (w, h) = (23, 17);
    let mut on = vec![false; w * h];
    for &(x, y) in &[(3usize, 4usize), (18, 2), (10, 14), (21, 15)] {
        on[y * w + x] = true;
    }
    let d = squared_distance(&on, w, h);
    for y in 0..h {
        for x in 0..w {
            let brute = [(3i64, 4i64), (18, 2), (10, 14), (21, 15)]
                .iter()
                .map(|&(a, b)| ((x as i64 - a).pow(2) + (y as i64 - b).pow(2)) as f32)
                .fold(f32::INFINITY, f32::min);
            assert_eq!(d[y * w + x], brute, "at ({x}, {y})");
        }
    }
}

#[test]
fn a_grid_with_no_copper_has_no_distance_and_one_cell_is_a_distance_of_the_size_of_the_cell() {
    let none = distance_mm(&[false; 9], 3, 3, 0.1);
    assert!(none.iter().all(|d| *d == f32::MAX));
    let mut on = vec![false; 9];
    on[4] = true;
    let d = distance_mm(&on, 3, 3, 0.1);
    assert_eq!(d[4], 0.0);
    assert!((d[5] - 0.1).abs() < 1e-6 && (d[0] - 0.1 * 2f32.sqrt()).abs() < 1e-6);
}

#[test]
fn the_copper_is_drawn_on_a_grid() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let b = from_scene(&s, 0.5, 0.05).unwrap();
    let area = b.count() as f64 * 0.05 * 0.05;
    assert!((area - std::f64::consts::PI).abs() < 0.08, "{area}");
    assert!(from_scene(&scene(vec![]), 0.5, 0.05).is_err());
    assert!(
        from_scene(&s, 0.5, 0.0).is_err()
            && from_scene(&s, 0.5, 5.0).is_err()
            && from_scene(&s, 0.5, f64::NAN).is_err()
    );
    assert!(from_scene(
        &scene(vec![Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 1000.0
        }]),
        0.0,
        0.02
    )
    .unwrap_err()
    .contains("more than"));
}

#[test]
fn the_first_pass_is_a_circle_one_tool_radius_from_a_disc() {
    let s = scene(vec![disc(5.0, 5.0, 1.0)]);
    let passes = isolate(
        &s,
        &Params {
            tool_diameter_mm: 0.2,
            passes: 1,
            stepover_mm: 0.1,
            res_mm: 0.02,
        },
    )
    .unwrap();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].paths.len(), 1, "one disc has one outline");
    let (lo, hi) = radius_range(&passes[0].paths[0], (5.0, 5.0));
    assert!(
        lo > 1.1 - 0.03 && hi < 1.1 + 0.03,
        "the radius is 1 + 0.1 within a cell: {lo}..{hi}"
    );
}

#[test]
fn more_passes_go_one_stepover_farther_each() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let passes = isolate(
        &s,
        &Params {
            tool_diameter_mm: 0.2,
            passes: 3,
            stepover_mm: 0.15,
            res_mm: 0.02,
        },
    )
    .unwrap();
    assert_eq!(passes.len(), 3);
    for (k, p) in passes.iter().enumerate() {
        assert!((p.offset_mm - (0.1 + 0.15 * k as f64)).abs() < 1e-9);
        let (lo, hi) = radius_range(&p.paths[0], (0.0, 0.0));
        let want = 1.0 + p.offset_mm;
        assert!(
            lo > want - 0.03 && hi < want + 0.03,
            "pass {k}: {lo}..{hi} against {want}"
        );
    }
}

#[test]
fn overlapping_tracks_have_one_outline_and_a_hole_gets_its_own() {
    // two tracks that cross and a ring (a disc with a hole made by a second pass on the inside is not possible, so a closed track loop)
    let cross = scene(vec![
        Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 0.0,
            w: 0.5,
        },
        Shape::Stroke {
            x0: 5.0,
            y0: -5.0,
            x1: 5.0,
            y1: 5.0,
            w: 0.5,
        },
    ]);
    let one = isolate(&cross, &Params::default()).unwrap();
    assert_eq!(
        one[0].paths.len(),
        1,
        "a cross is one piece of copper with one outline"
    );
    let square = scene(vec![
        Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 6.0,
            y1: 0.0,
            w: 0.4,
        },
        Shape::Stroke {
            x0: 6.0,
            y0: 0.0,
            x1: 6.0,
            y1: 6.0,
            w: 0.4,
        },
        Shape::Stroke {
            x0: 6.0,
            y0: 6.0,
            x1: 0.0,
            y1: 6.0,
            w: 0.4,
        },
        Shape::Stroke {
            x0: 0.0,
            y0: 6.0,
            x1: 0.0,
            y1: 0.0,
            w: 0.4,
        },
    ]);
    let two = isolate(&square, &Params::default()).unwrap();
    assert_eq!(
        two[0].paths.len(),
        2,
        "a closed loop of copper has an outside and an inside"
    );
}

#[test]
fn separate_pieces_each_have_their_own_outline() {
    let s = scene(vec![
        disc(0.0, 0.0, 0.5),
        disc(5.0, 0.0, 0.5),
        Shape::Rect {
            x: 10.0,
            y: 0.0,
            w: 1.0,
            h: 2.0,
        },
    ]);
    let p = isolate(&s, &Params::default()).unwrap();
    assert_eq!(p[0].paths.len(), 3);
}

#[test]
fn a_rectangle_is_surrounded_at_the_radius_of_the_tool() {
    let s = scene(vec![Shape::Rect {
        x: 0.0,
        y: 0.0,
        w: 4.0,
        h: 2.0,
    }]);
    let p = isolate(
        &s,
        &Params {
            tool_diameter_mm: 0.4,
            ..Params::default()
        },
    )
    .unwrap();
    let xs: Vec<f64> = p[0].paths[0].iter().map(|q| q.0).collect();
    let (min, max) = (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    assert!(
        (min + 2.2).abs() < 0.03 && (max - 2.2).abs() < 0.03,
        "{min}..{max}"
    );
    assert!(
        p[0].paths[0].len() < 60,
        "the straight sides are not made of hundreds of points: {}",
        p[0].paths[0].len()
    );
}

#[test]
fn the_tool_never_gets_closer_to_the_copper_than_its_radius() {
    let s = scene(vec![
        disc(0.0, 0.0, 0.8),
        disc(3.0, 0.5, 0.6),
        Shape::Stroke {
            x0: -2.0,
            y0: 2.0,
            x1: 4.0,
            y1: 3.0,
            w: 0.3,
        },
    ]);
    let params = Params {
        tool_diameter_mm: 0.3,
        ..Params::default()
    };
    let passes = isolate(&s, &params).unwrap();
    let nearest = |q: (f64, f64)| {
        s.shapes
            .iter()
            .map(|sh| match sh {
                Shape::Disc { x, y, r } => ((q.0 - x).powi(2) + (q.1 - y).powi(2)).sqrt() - r,
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    let (dx, dy) = (x1 - x0, y1 - y0);
                    let t =
                        (((q.0 - x0) * dx + (q.1 - y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                    ((q.0 - (x0 + t * dx)).powi(2) + (q.1 - (y0 + t * dy)).powi(2)).sqrt() - w / 2.0
                }
                _ => f64::INFINITY,
            })
            .fold(f64::INFINITY, f64::min)
    };
    for path in &passes[0].paths {
        for q in path {
            let d = nearest(*q);
            assert!(
                d > 0.15 - 0.03 && d < 0.15 + 0.03,
                "a point of the path is {d} mm from the copper, it should be 0.15"
            );
        }
    }
}

#[test]
fn the_simplified_path_stays_within_the_tolerance() {
    let circle: Vec<(f64, f64)> = (0..400)
        .map(|i| (f64::from(i) * std::f64::consts::TAU / 400.0).sin_cos())
        .map(|(s, c)| (c * 10.0, s * 10.0))
        .collect();
    let simple = simplify(&circle, 0.01);
    assert!(
        simple.len() < circle.len() / 2 && simple.len() >= 8,
        "{}",
        simple.len()
    );
    for p in &circle {
        let near = simple
            .iter()
            .zip(simple.iter().cycle().skip(1))
            .map(|(a, b)| {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-12))
                    .clamp(0.0, 1.0);
                ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
            });
        assert!(near.fold(f64::INFINITY, f64::min) < 0.011);
    }
    assert_eq!(simplify(&circle[..3], 1.0).len(), 3);
    assert_eq!(simplify(&circle, 0.0).len(), 400);
}

#[test]
fn two_pieces_closer_than_the_tool_are_reported_and_distant_ones_are_not() {
    let close = scene(vec![disc(0.0, 0.0, 1.0), disc(2.2, 0.0, 1.0)]); // 0.2 mm apart
    let c = close_pieces(&close, 0.3, 0.02).unwrap();
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!(c[0].pieces, 2);
    assert!((c[0].x - 1.1).abs() < 0.2 && c[0].y.abs() < 0.2, "{c:?}");
    assert!(
        close_pieces(&close, 0.15, 0.02).unwrap().is_empty(),
        "a tool narrower than the gap fits"
    );
    let far = scene(vec![disc(0.0, 0.0, 1.0), disc(5.0, 0.0, 1.0)]);
    assert!(close_pieces(&far, 0.3, 0.02).unwrap().is_empty());
    assert!(close_pieces(&scene(vec![disc(0.0, 0.0, 1.0)]), 0.3, 0.02)
        .unwrap()
        .is_empty());
    assert!(close_pieces(&far, 0.0, 0.02).is_err() && close_pieces(&far, f64::NAN, 0.02).is_err());
}

#[test]
fn the_parameters_are_checked() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    for bad in [
        Params {
            tool_diameter_mm: 0.0,
            ..Params::default()
        },
        Params {
            passes: 0,
            ..Params::default()
        },
        Params {
            passes: 11,
            ..Params::default()
        },
        Params {
            stepover_mm: f64::NAN,
            ..Params::default()
        },
        Params {
            res_mm: 0.5,
            ..Params::default()
        },
        Params {
            res_mm: 0.0,
            ..Params::default()
        },
    ] {
        assert!(isolate(&s, &bad).is_err(), "{bad:?}");
    }
}

#[test]
fn the_gcode_cuts_each_closed_path_at_the_depth_and_lifts_between() {
    let s = scene(vec![disc(0.0, 0.0, 1.0), disc(5.0, 0.0, 1.0)]);
    let passes = isolate(&s, &Params::default()).unwrap();
    let g = write(&passes, &Job::default()).unwrap();
    assert!(g.starts_with("G21\nG90\nG0 Z2\nM3 S24000\n") && g.ends_with("M5\nM2\n"));
    assert_eq!(g.matches("G1 Z-0.06 F60").count(), 2);
    assert_eq!(
        g.matches("G0 Z2\n").count(),
        3,
        "once at the start and once after each path"
    );
    assert!(length(&passes[0].paths) > 2.0 * std::f64::consts::PI * 1.1 * 2.0 * 0.95);
    for bad in [
        Job {
            depth_mm: 0.0,
            ..Job::default()
        },
        Job {
            feed_mm_min: -1.0,
            ..Job::default()
        },
        Job {
            safe_z_mm: f64::NAN,
            ..Job::default()
        },
        Job {
            spindle_rpm: 10.0,
            ..Job::default()
        },
    ] {
        assert!(write(&passes, &bad).is_err());
    }
}

#[test]
fn a_gerber_layer_goes_all_the_way_to_the_gcode() {
    let layer = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.4*%\n%ADD11R,1.6X1.6*%\nD10*\nX20000Y20000D02*\nX80000Y20000D01*\nD11*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";
    let sc = from_gerber(&eh3d_parser_gerber::parse(layer));
    let passes = isolate(
        &sc,
        &Params {
            tool_diameter_mm: 0.2,
            passes: 2,
            stepover_mm: 0.1,
            res_mm: 0.02,
        },
    )
    .unwrap();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].paths.len(),
        1,
        "the two pads and the track are one piece"
    );
    let g = write(&passes, &Job::default()).unwrap();
    assert!(
        g.contains("(pass 1 at 0.1 mm from the copper, 1 paths)")
            && g.contains("(pass 2 at 0.2 mm")
    );
}

#[test]
fn garbage_layers_do_not_panic() {
    for text in [
        "",
        "G36*G37*",
        "%ADD10C,0*%D10*X0Y0D03*",
        "%FSLAX24Y24*%%MOMM*%%ADD10C,1*%D10*X0Y0D02*X0Y0D01*M02*",
    ] {
        let sc = from_gerber(&eh3d_parser_gerber::parse(text));
        let _ = isolate(&sc, &Params::default());
        let _ = close_pieces(&sc, 0.2, 0.02);
    }
}

#[test]
fn the_passes_as_hpgl_use_one_pen_for_each_pass_and_forty_units_to_the_millimetre() {
    let s = scene(vec![disc(5.0, 5.0, 1.0)]);
    let passes = isolate(
        &s,
        &Params {
            passes: 2,
            ..Params::default()
        },
    )
    .unwrap();
    let h = eh3d_isocam::hpgl::write(&passes).unwrap();
    assert!(
        h.starts_with(
            "IN;
SP1;
PU"
        ) && h.ends_with(
            "PU0,0;SP0;
"
        )
    );
    assert!(h.contains("SP2;"));
    assert_eq!(h.matches("PD").count(), 2, "one closed path in each pass");
    // the first pass is 1.1 mm from the centre of the disc: the point at the right is at 6.1 mm = 244 units
    assert!(h.contains(",244") || h.contains("244,"), "{h}");
    // no coordinate is negative in a plot file
    let moved = eh3d_isocam::hpgl::write(
        &isolate(&scene(vec![disc(0.0, 0.0, 1.0)]), &Params::default()).unwrap(),
    )
    .unwrap();
    assert!(
        !moved.contains(",-")
            && !moved.contains("PU-")
            && !moved.contains("PD-")
            && !moved.contains(";-"),
        "{moved}"
    );
}

#[test]
fn more_passes_than_pens_are_refused() {
    let many: Vec<eh3d_isocam::isolate::Pass> = (0..9)
        .map(|k| eh3d_isocam::isolate::Pass {
            offset_mm: 0.1 * f64::from(k),
            ..Default::default()
        })
        .collect();
    assert!(eh3d_isocam::hpgl::write(&many)
        .unwrap_err()
        .contains("pens"));
}
