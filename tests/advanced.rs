//! Forced isolation, lighter paths, scale and correction from reference points, the solder mask, units and the preview, each with damaged input.

use eh3d_gerberviewer::scene::{from_gerber, Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::fiducial::{
    apply_to_holes, apply_to_passes, fit, parse_pair, Affine, Limits, Model, Pair,
};
use eh3d_isocam::forced::{forced_passes, insert, Forced};
use eh3d_isocam::gcode::{write_in, Job};
use eh3d_isocam::isolate::{plan, Pass, Plan, Tool};
use eh3d_isocam::mask::{openings, write_gerber};
use eh3d_isocam::optimize::{optimize_passes, optimize_path, Optimize};
use eh3d_isocam::order::Order;
use eh3d_isocam::preview::{svg, text};
use eh3d_isocam::units::{parse_length, Unit};

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn track(y: f64) -> Shape {
    Shape::Stroke {
        x0: 0.0,
        y0: y,
        x1: 5.0,
        y1: y,
        w: 0.5,
    }
}

fn tool_plan(d: f64) -> Plan {
    Plan {
        tools: vec![Tool {
            diameter_mm: d,
            passes: 1,
            stepover_mm: 0.1,
        }],
        rubout: None,
        res_mm: 0.02,
        trim: false,
    }
}

// ---- IC-5: forced isolation -----------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_path_is_laid_along_the_middle_of_a_gap_the_tool_does_not_fit_in() {
    // two tracks 0.5 mm wide whose edges are 0.15 mm apart; the tool is 0.2 mm
    let s = scene(vec![track(0.0), track(0.65)]);
    let r = forced_passes(&s, &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert_eq!(r.passes.len(), 1);
    assert!(r.too_narrow.is_empty());
    let pass = &r.passes[0];
    assert!(pass.paths.is_empty() && !pass.open.is_empty());
    for p in pass.open.iter().flatten() {
        assert!(
            (p.1 - 0.325).abs() < 0.03,
            "the path is at y = {}, not in the middle of the gap",
            p.1
        );
    }
    let length: f64 = pass
        .open
        .iter()
        .map(|o| {
            o.windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum::<f64>()
        })
        .sum();
    assert!(length > 3.5, "only {length} mm of path");
    assert_eq!(r.forced.len(), pass.open.len());
    let main = r
        .forced
        .iter()
        .max_by(|a, b| a.length_mm.total_cmp(&b.length_mm))
        .unwrap();
    assert!((main.gap_mm - 0.15).abs() < 0.04, "{main:?}");
    assert!(main.length_mm > 3.5);
}

#[test]
fn a_gap_narrower_than_the_minimum_gets_no_path_and_is_listed() {
    let s = scene(vec![track(0.0), track(0.55)]);
    let r = forced_passes(&s, &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).unwrap();
    // only the short stretches at the ends, where the round ends of the tracks open the gap out, are wide enough
    assert!(r.forced.iter().all(|s| s.length_mm < 0.5), "{:?}", r.forced);
    assert!(!r.too_narrow.is_empty());
    assert!(r.too_narrow[0].gap_mm < 0.1);
    assert!((r.too_narrow[0].y - 0.275).abs() < 0.05);
}

#[test]
fn nothing_is_forced_when_the_tool_fits_or_there_is_one_piece() {
    let wide = scene(vec![track(0.0), track(1.0)]);
    let r = forced_passes(&wide, &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert!(r.passes.is_empty() && r.forced.is_empty() && r.too_narrow.is_empty());
    let one = scene(vec![track(0.0)]);
    let r = forced_passes(&one, &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert_eq!(r, Default::default());
}

#[test]
fn forced_isolation_refuses_bad_values_and_an_empty_layer() {
    let s = scene(vec![track(0.0), track(0.65)]);
    for gap in [0.5, 0.2, 0.0, -1.0, f64::NAN, 0.01, 0.03] {
        let e = forced_passes(&s, &tool_plan(0.2), &Forced { min_gap_mm: gap }).unwrap_err();
        assert!(e.contains("forced_min_gap_mm"), "{gap}: {e}");
    }
    assert!(forced_passes(&scene(vec![]), &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).is_err());
    // a shape with a broken number is refused, not drawn
    let bad = scene(vec![Shape::Disc {
        x: f64::NAN,
        y: 0.0,
        r: 1.0,
    }]);
    assert!(forced_passes(&bad, &tool_plan(0.2), &Forced { min_gap_mm: 0.1 }).is_err());
}

#[test]
fn the_forced_pass_goes_before_the_rings_of_the_leftover_copper() {
    let ring = Pass {
        rubout: true,
        ..Pass::default()
    };
    let mut passes = vec![Pass::default(), ring.clone()];
    insert(
        &mut passes,
        vec![Pass {
            offset_mm: 0.07,
            ..Pass::default()
        }],
    );
    assert_eq!(passes.len(), 3);
    assert!((passes[1].offset_mm - 0.07).abs() < 1e-12 && passes[2].rubout);
    let mut none = vec![Pass::default()];
    insert(&mut none, vec![Pass::default()]);
    assert_eq!(none.len(), 2);
}

// ---- IC-7: lighter paths ---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn spikes_are_taken_out_and_points_on_a_line_are_joined() {
    let o = Optimize {
        tolerance_mm: 0.001,
        spike_max_mm: 0.05,
        spike_angle_deg: 170.0,
    };
    let path = [(0.0, 0.0), (2.0, 0.0), (1.99, 0.0), (4.0, 0.0)];
    let (v, spikes) = optimize_path(&path, false, &o).unwrap();
    assert_eq!(spikes, 1);
    assert_eq!(v, vec![(0.0, 0.0), (4.0, 0.0)]);
    // without the spike rule the spike stays
    let only_line = Optimize {
        spike_max_mm: 0.0,
        ..o
    };
    let (_, spikes) = optimize_path(&path, false, &only_line).unwrap();
    assert_eq!(spikes, 0);
    // a long out and back is not a spike
    let long = [(0.0, 0.0), (2.0, 0.0), (0.5, 0.0), (4.0, 0.0)];
    assert_eq!(optimize_path(&long, false, &o).unwrap().1, 0);
}

#[test]
fn the_points_of_a_line_within_the_tolerance_are_one_move_and_a_square_stays_a_square() {
    let o = Optimize {
        tolerance_mm: 0.01,
        ..Optimize::default()
    };
    let noisy: Vec<(f64, f64)> = (0..=10)
        .map(|i| (i as f64, if i % 2 == 0 { 0.0 } else { 0.004 }))
        .collect();
    let (v, _) = optimize_path(&noisy, false, &o).unwrap();
    assert_eq!(v.len(), 2);
    let square = [
        (0.0, 0.0),
        (1.0, 0.0),
        (2.0, 0.0),
        (2.0, 1.0),
        (2.0, 2.0),
        (1.0, 2.0),
        (0.0, 2.0),
        (0.0, 1.0),
    ];
    let (v, _) = optimize_path(&square, true, &o).unwrap();
    assert_eq!(v.len(), 4);
    // the path is never farther from the original than the tolerance
    for p in &square {
        let d = v
            .iter()
            .zip(v.iter().cycle().skip(1))
            .map(|(a, b)| {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let t =
                    (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
            })
            .fold(f64::INFINITY, f64::min);
        assert!(d <= 0.01 + 1e-9);
    }
}

#[test]
fn repeated_points_go_and_a_closing_point_is_not_kept_twice() {
    let o = Optimize::default();
    let (v, _) = optimize_path(
        &[(0.0, 0.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)],
        true,
        &o,
    )
    .unwrap();
    assert_eq!(v, vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
    assert_eq!(
        optimize_path(&[], false, &o).unwrap().0,
        Vec::<(f64, f64)>::new()
    );
}

#[test]
fn damaged_paths_and_settings_are_refused_with_the_place() {
    let o = Optimize {
        tolerance_mm: 0.01,
        ..Optimize::default()
    };
    let e = optimize_path(&[(0.0, 0.0), (f64::NAN, 1.0)], false, &o).unwrap_err();
    assert!(e.contains("point 2"), "{e}");
    let e = optimize_path(&[(0.0, f64::INFINITY)], false, &o).unwrap_err();
    assert!(e.contains("point 1"), "{e}");
    for bad in [
        Optimize {
            tolerance_mm: -1.0,
            ..o
        },
        Optimize {
            tolerance_mm: 5.0,
            ..o
        },
        Optimize {
            spike_max_mm: f64::NAN,
            ..o
        },
        Optimize {
            spike_angle_deg: 10.0,
            ..o
        },
    ] {
        assert!(bad.validate().is_err());
        assert!(optimize_path(&[(0.0, 0.0), (1.0, 1.0)], false, &bad).is_err());
    }
    let mut passes = vec![Pass {
        paths: vec![vec![(0.0, 0.0), (1.0, 1.0), (f64::NAN, 0.0)]],
        ..Pass::default()
    }];
    let e = optimize_passes(&mut passes, &o).unwrap_err();
    assert!(e.contains("pass 1") && e.contains("point 3"), "{e}");
}

#[test]
fn a_path_left_with_too_few_points_is_dropped_and_counted() {
    let o = Optimize {
        spike_max_mm: 0.1,
        ..Optimize::default()
    };
    let mut passes = vec![Pass {
        paths: vec![
            vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
            vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        ],
        open: vec![vec![(0.0, 0.0), (0.0, 0.0)]],
        ..Pass::default()
    }];
    let st = optimize_passes(&mut passes, &o).unwrap();
    assert_eq!(st.dropped_paths, 2);
    assert_eq!(passes[0].paths.len(), 1);
    assert!(passes[0].open.is_empty());
    assert!(st.points_after < st.points_before);
}

// ---- IC-7: scale and reference points --------------------------------------------------------------------------------------------------------------

fn pair(d: (f64, f64), m: (f64, f64)) -> Pair {
    Pair {
        design: d,
        measured: m,
    }
}

fn near(a: (f64, f64), b: (f64, f64)) {
    assert!(
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6,
        "{a:?} != {b:?}"
    );
}

#[test]
fn two_reference_points_give_the_scale_the_turn_and_the_shift() {
    let truth = Affine {
        a: 1.01 * 0.0174f64.cos(),
        b: -1.01 * 0.0174f64.sin(),
        c: 1.01 * 0.0174f64.sin(),
        d: 1.01 * 0.0174f64.cos(),
        e: 0.3,
        f: -0.2,
    };
    let (p, q) = ((10.0, 10.0), (90.0, 70.0));
    let f = fit(
        &[pair(p, truth.apply(p)), pair(q, truth.apply(q))],
        Model::Auto,
        &Limits::default(),
    )
    .unwrap();
    near(f.affine.apply((50.0, 40.0)), truth.apply((50.0, 40.0)));
    assert!(
        (f.scale_x - 1.01).abs() < 1e-9 && (f.rotation_deg - 0.0174f64.to_degrees()).abs() < 1e-6
    );
    assert!(f.residuals_mm.iter().all(|r| *r < 1e-9));
}

#[test]
fn three_reference_points_give_an_affine_fit_and_more_are_averaged() {
    let truth = Affine {
        a: 1.002,
        b: 0.003,
        c: -0.001,
        d: 0.996,
        e: 0.5,
        f: 0.25,
    };
    let pts = [(0.0, 0.0), (100.0, 0.0), (0.0, 80.0), (100.0, 80.0)];
    let pairs: Vec<Pair> = pts.iter().map(|p| pair(*p, truth.apply(*p))).collect();
    let f = fit(&pairs[..3], Model::Auto, &Limits::default()).unwrap();
    near(f.affine.apply((37.0, 11.0)), truth.apply((37.0, 11.0)));
    let f = fit(&pairs, Model::Affine, &Limits::default()).unwrap();
    near(f.affine.apply((37.0, 11.0)), truth.apply((37.0, 11.0)));
    // the same marks with a similarity: not exact, but within what the measurement allows
    let f = fit(
        &pairs,
        Model::Similarity,
        &Limits {
            max_residual_mm: 1.0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(f.residuals_mm.iter().any(|r| *r > 1e-4));
    assert_eq!(f.affine.b, -f.affine.c);
}

#[test]
fn bad_reference_points_are_refused() {
    let l = Limits::default();
    let ok = pair((0.0, 0.0), (0.0, 0.0));
    let e = fit(&[ok], Model::Auto, &l).unwrap_err();
    assert!(e.contains("at least 2"), "{e}");
    let e = fit(&[ok, pair((0.0, 0.0), (1.0, 1.0))], Model::Auto, &l).unwrap_err();
    assert!(e.contains("same place"), "{e}");
    let e = fit(&[ok, pair((1.0, 0.0), (1.0, 0.0))], Model::Affine, &l).unwrap_err();
    assert!(e.contains("at least 3"), "{e}");
    let line = [
        ok,
        pair((1.0, 0.0), (1.0, 0.0)),
        pair((2.0, 0.0), (2.0, 0.0)),
    ];
    let e = fit(&line, Model::Affine, &l).unwrap_err();
    assert!(e.contains("line"), "{e}");
    let e = fit(&[ok, pair((f64::NAN, 0.0), (1.0, 0.0))], Model::Auto, &l).unwrap_err();
    assert!(e.contains("fiducial 2"), "{e}");
    // marks mismatched: a scale far from 1
    let e = fit(&[ok, pair((10.0, 0.0), (20.0, 0.0))], Model::Auto, &l).unwrap_err();
    assert!(e.contains("scale"), "{e}");
    // a board turned over
    let mirrored = [
        ok,
        pair((10.0, 0.0), (10.0, 0.0)),
        pair((0.0, 10.0), (0.0, -10.0)),
    ];
    assert!(fit(
        &mirrored,
        Model::Affine,
        &Limits {
            max_scale_deviation: 0.5,
            ..l
        }
    )
    .is_err());
    // one wrong measurement among good ones
    let pts = [(0.0, 0.0), (100.0, 0.0), (0.0, 80.0), (100.0, 80.0)];
    let mut pairs: Vec<Pair> = pts.iter().map(|p| pair(*p, *p)).collect();
    pairs[3].measured = (101.0, 81.0);
    let e = fit(&pairs, Model::Affine, &l).unwrap_err();
    assert!(e.contains("from where the fit puts it"), "{e}");
}

#[test]
fn a_transformation_that_collapses_or_breaks_is_not_applied() {
    let mut passes = vec![Pass {
        paths: vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        ..Pass::default()
    }];
    for t in [
        Affine::scale(0.0, 1.0),
        Affine::scale(-1.0, 1.0),
        Affine::scale(f64::NAN, 1.0),
        Affine::scale(f64::INFINITY, 1.0),
    ] {
        assert!(apply_to_passes(&mut passes, &t).is_err());
        assert!(apply_to_holes(&mut [], &t).is_err());
    }
    assert_eq!(passes[0].paths[0][1], (1.0, 0.0));
    apply_to_passes(&mut passes, &Affine::scale(2.0, 3.0)).unwrap();
    assert_eq!(passes[0].paths[0][2], (2.0, 3.0));
    let big = Affine::scale(1e308, 1e308);
    assert!(apply_to_passes(&mut passes, &big).is_err());
    let composed = Affine::scale(2.0, 2.0).then(&Affine {
        e: 1.0,
        ..Affine::identity()
    });
    assert_eq!(composed.apply((1.0, 1.0)), (3.0, 2.0));
}

#[test]
fn reference_points_are_read_with_their_units_and_refused_when_wrong() {
    let p = parse_pair("1, 2, 1.5, 2.5", Unit::Mm).unwrap();
    assert_eq!((p.design, p.measured), ((1.0, 2.0), (1.5, 2.5)));
    let p = parse_pair("1000mil 0 1in 0", Unit::Mm).unwrap();
    assert!((p.design.0 - 25.4).abs() < 1e-9 && (p.measured.0 - 25.4).abs() < 1e-9);
    for bad in ["", "1,2,3", "1,2,3,4,5", "1,2,x,4", "1,2,3,nan"] {
        assert!(parse_pair(bad, Unit::Mm).is_err(), "{bad}");
    }
}

// ---- IC-8: solder mask -----------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_openings_come_from_the_pads_of_the_copper_with_the_margin() {
    let copper = scene(vec![
        Shape::Disc {
            x: 1.0,
            y: 1.0,
            r: 0.4,
        },
        Shape::Rect {
            x: 3.0,
            y: 1.0,
            w: 1.0,
            h: 0.6,
        },
        Shape::Obround {
            x: 5.0,
            y: 1.0,
            w: 1.2,
            h: 0.6,
        },
        track(3.0),
    ]);
    let o = openings(&copper, None, 0.05).unwrap();
    assert_eq!(o.shapes.len(), 3);
    assert_eq!(
        o.shapes[0],
        Shape::Disc {
            x: 1.0,
            y: 1.0,
            r: 0.45
        }
    );
    match &o.shapes[1] {
        Shape::Rect { w, h, .. } => assert!((w - 1.1).abs() < 1e-9 && (h - 0.7).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
    assert!(o.notes.iter().any(|n| n.contains("1 tracks")));
}

#[test]
fn the_openings_come_from_the_mask_layer_when_there_is_one() {
    let copper = scene(vec![Shape::Disc {
        x: 0.0,
        y: 0.0,
        r: 1.0,
    }]);
    let mask = scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 1.2,
        },
        Shape::Stroke {
            x0: 0.0,
            y0: 5.0,
            x1: 2.0,
            y1: 5.0,
            w: 0.4,
        },
        Shape::Region {
            pts: vec![(10.0, 10.0), (12.0, 10.0), (12.0, 12.0)],
        },
    ]);
    let o = openings(&copper, Some(&mask), 0.1).unwrap();
    assert!(o.shapes.contains(&Shape::Disc {
        x: 0.0,
        y: 0.0,
        r: 1.3
    }));
    assert!(o.shapes.iter().any(
        |s| matches!(s, Shape::Stroke { w, y0, .. } if (*w - 0.6).abs() < 1e-9 && *y0 == 5.0)
    ));
    // the region keeps its shape and gets the margin as a pen along its edge
    assert!(
        o.shapes
            .iter()
            .filter(|s| matches!(s, Shape::Stroke { w, .. } if (*w - 0.2).abs() < 1e-9))
            .count()
            == 3
    );
}

#[test]
fn a_negative_margin_can_close_an_opening_and_bad_margins_are_refused() {
    let copper = scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 0.03,
        },
        Shape::Disc {
            x: 3.0,
            y: 0.0,
            r: 0.5,
        },
    ]);
    let o = openings(&copper, None, -0.05).unwrap();
    assert_eq!(o.shapes.len(), 1);
    assert!(o.notes.iter().any(|n| n.contains("1 openings are gone")));
    let e = openings(
        &scene(vec![Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 0.03,
        }]),
        None,
        -0.05,
    )
    .unwrap_err();
    assert!(e.contains("no pads"), "{e}");
    for m in [f64::NAN, 2.0, -2.0, f64::INFINITY] {
        assert!(openings(&copper, None, m)
            .unwrap_err()
            .contains("mask_margin_mm"));
    }
    assert!(openings(&scene(vec![track(0.0)]), None, 0.05).is_err());
    let e = openings(
        &scene(vec![Shape::Disc {
            x: f64::NAN,
            y: 0.0,
            r: 1.0,
        }]),
        None,
        0.05,
    )
    .unwrap_err();
    assert!(e.contains("shape 1"), "{e}");
    let e = openings(&copper, Some(&scene(vec![])), 0.0).unwrap_err();
    assert!(e.contains("no openings"), "{e}");
}

#[test]
fn the_mask_is_written_as_a_gerber_layer_that_reads_back() {
    let o = scene(vec![
        Shape::Disc {
            x: 1.0,
            y: 2.0,
            r: 0.45,
        },
        Shape::Disc {
            x: 4.0,
            y: 2.0,
            r: 0.45,
        },
        Shape::Rect {
            x: 3.0,
            y: 1.0,
            w: 1.1,
            h: 0.7,
        },
        Shape::Obround {
            x: 5.0,
            y: 1.0,
            w: 1.3,
            h: 0.7,
        },
        Shape::Stroke {
            x0: 0.0,
            y0: 5.0,
            x1: 2.0,
            y1: 5.0,
            w: 0.6,
        },
        Shape::Region {
            pts: vec![(10.0, 10.0), (12.0, 10.0), (12.0, 12.0)],
        },
    ]);
    let text = write_gerber(&o).unwrap();
    assert!(text.starts_with("%FSLAX46Y46*%") && text.trim_end().ends_with("M02*"));
    // the two equal discs share one aperture
    assert_eq!(text.matches("%ADD").count(), 4);
    let back = from_gerber(&eh3d_parser_gerber::parse(&text));
    assert_eq!(back.shapes.len(), o.shapes.len(), "{:?}", back.notes);
    for (a, b) in o.shapes.iter().zip(&back.shapes) {
        assert_eq!(
            std::mem::discriminant(a),
            std::mem::discriminant(b),
            "{a:?} / {b:?}"
        );
    }
    let (ba, bb) = (o.bounds(), back.bounds());
    assert!((ba.min_x - bb.min_x).abs() < 1e-6 && (ba.max_y - bb.max_y).abs() < 1e-6);
}

#[test]
fn a_mask_that_cannot_be_written_says_which_shape() {
    let e = write_gerber(&scene(vec![
        Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 1.0,
        },
        Shape::Rect {
            x: f64::NAN,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
    ]))
    .unwrap_err();
    assert!(e.contains("shape 2"), "{e}");
    let e = write_gerber(&scene(vec![Shape::Region {
        pts: vec![(0.0, 0.0), (1.0, 1.0)],
    }]))
    .unwrap_err();
    assert!(e.contains("shape 1") && e.contains("at least 3"), "{e}");
    let e = write_gerber(&scene(vec![Shape::Disc {
        x: 1e30,
        y: 0.0,
        r: 1.0,
    }]))
    .unwrap_err();
    assert!(e.contains("too far"), "{e}");
}

// ---- IC-9: units and preview -----------------------------------------------------------------------------------------------------------------------------

#[test]
fn lengths_are_read_in_mm_mils_and_inches() {
    let mm = |t: &str, u: Unit| parse_length(t, u).unwrap();
    assert!((mm("8mil", Unit::Mm) - 0.2032).abs() < 1e-9);
    assert!((mm("8 mils", Unit::Mm) - 0.2032).abs() < 1e-9);
    assert!((mm("0.01in", Unit::Mm) - 0.254).abs() < 1e-9);
    assert!((mm("0.01 inch", Unit::Mm) - 0.254).abs() < 1e-9);
    assert!((mm("0.2mm", Unit::Inch) - 0.2).abs() < 1e-12);
    assert!((mm("0.2", Unit::Inch) - 5.08).abs() < 1e-9);
    assert!((mm("8", Unit::Mil) - 0.2032).abs() < 1e-9);
    assert!((mm("1e-3in", Unit::Mm) - 0.0254).abs() < 1e-12);
    assert!((mm("-5MIL", Unit::Mm) + 0.127).abs() < 1e-12);
    for bad in [
        "",
        "  ",
        "abc",
        "mil",
        "1.2.3mm",
        "nanmm",
        "inf",
        "5 furlongs",
        "1,5",
    ] {
        assert!(parse_length(bad, Unit::Mm).is_err(), "'{bad}'");
    }
    assert!(Unit::parse("cubits").is_err());
    assert_eq!(Unit::parse("INCHES").unwrap(), Unit::Inch);
    assert!((Unit::Inch.from_mm(25.4) - 1.0).abs() < 1e-12);
}

#[test]
fn the_gcode_can_be_written_in_inches() {
    let pass = Pass {
        offset_mm: 0.1,
        paths: vec![vec![(25.4, 0.0), (25.4, 12.7), (0.0, 12.7)]],
        tool: 0,
        diameter_mm: 0.2,
        ..Pass::default()
    };
    let job = Job::default();
    let g = write_in(
        std::slice::from_ref(&pass),
        &job,
        &Order::default(),
        Unit::Inch,
    )
    .unwrap();
    assert!(g.starts_with("G20\n") && !g.contains("G21"));
    assert!(g.contains("X1 Y0") || g.contains("X1 Y0.5"), "{g}");
    assert!(g.contains("Z-0.00236"), "{g}");
    let mm = write_in(
        std::slice::from_ref(&pass),
        &job,
        &Order::default(),
        Unit::Mm,
    )
    .unwrap();
    assert!(mm.starts_with("G21\n") && mm.contains("X25.4"));
    let e = write_in(&[pass], &job, &Order::default(), Unit::Mil).unwrap_err();
    assert!(e.contains("mils"), "{e}");
}

fn drawn() -> (Scene, Vec<Pass>) {
    let s = scene(vec![
        Shape::Disc {
            x: 5.0,
            y: 5.0,
            r: 1.0,
        },
        Shape::Rect {
            x: 10.0,
            y: 5.0,
            w: 2.0,
            h: 1.0,
        },
        Shape::Obround {
            x: 15.0,
            y: 5.0,
            w: 2.0,
            h: 1.0,
        },
        track(8.0),
        Shape::Region {
            pts: vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0)],
        },
    ]);
    let passes = plan(&s, &tool_plan(0.2)).unwrap();
    (s, passes)
}

#[test]
fn the_preview_is_an_svg_drawing_with_every_pass() {
    let (s, passes) = drawn();
    let svg = svg(&s, &passes).unwrap();
    assert!(svg.starts_with("<svg ") && svg.trim_end().ends_with("</svg>"));
    assert_eq!(svg.matches("<g").count(), svg.matches("</g>").count());
    assert!(svg.contains("<circle") && svg.contains("<polygon") && svg.contains("<title>pass 1"));
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
}

#[test]
fn the_preview_is_a_text_picture_with_a_key() {
    let (s, passes) = drawn();
    let t = text(&s, &passes, 60).unwrap();
    assert!(t.contains('#') && t.contains('1'));
    assert!(t.contains("1 pass 1 (0.2 mm tool"));
    for line in t
        .lines()
        .filter(|l| !l.starts_with("# copper") && !l.contains(" pass "))
    {
        assert!(line.chars().count() <= 62, "'{line}'");
    }
}

#[test]
fn the_preview_refuses_nothing_to_draw_bad_widths_and_broken_points() {
    let (s, passes) = drawn();
    assert!(text(&s, &passes, 5).is_err() && text(&s, &passes, 1000).is_err());
    assert!(svg(&scene(vec![]), &[])
        .unwrap_err()
        .contains("nothing to draw"));
    assert!(text(&scene(vec![]), &[], 50).is_err());
    let broken = vec![Pass {
        paths: vec![vec![(0.0, 0.0), (f64::NAN, 1.0), (2.0, 2.0)]],
        ..Pass::default()
    }];
    assert!(svg(&s, &broken).unwrap_err().contains("pass 1"));
    assert!(text(&s, &broken, 50).is_err());
    // the passes alone, with no copper, and a path that is a single point
    let alone = vec![Pass {
        open: vec![vec![(0.0, 0.0), (3.0, 0.0)]],
        ..Pass::default()
    }];
    assert!(svg(&scene(vec![]), &alone).is_ok());
    assert!(text(&scene(vec![]), &alone, 40).unwrap().contains('1'));
    let dot = vec![Pass {
        open: vec![vec![(1.0, 1.0)]],
        ..Pass::default()
    }];
    assert!(text(&scene(vec![]), &dot, 40).is_ok());
}

// ---- the settings file -----------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_new_settings_are_read_written_back_and_checked() {
    let text = "forced_isolation = yes\nforced_min_gap_mm = 4mil\noptimize_tolerance_mm = 0.005\nspike_max_mm = 0.05\nscale_x = 1.001\nscale_y = 0.999\nfiducial = 0, 0, 0.1, 0.05\nfiducial = 100, 0, 100.2, 0.05\nmask_margin_mm = 0.1\npreview_cols = 80\noutput_units = inch\nreport_units = mil\nfiducial_model = similarity\n";
    let s = Settings::parse(text).unwrap();
    assert!(s.forced_isolation && (s.forced_min_gap_mm - 0.1016).abs() < 1e-9);
    assert_eq!(s.fiducials.len(), 2);
    assert_eq!((s.output_units, s.report_units), (Unit::Inch, Unit::Mil));
    let again = Settings::parse(&s.to_text()).unwrap();
    assert_eq!(again, s);
    let (t, fit) = s.transform().unwrap();
    assert!(fit.is_some() && !t.is_identity());
    assert!(s.forced().is_some() && !s.optimize().is_off());
    let d = Settings::default();
    assert!(
        d.forced().is_none() && d.optimize().is_off() && d.transform().unwrap().0.is_identity()
    );
}

#[test]
fn units_in_the_file_count_wherever_input_units_is_written() {
    let s =
        Settings::parse("fine_tool_mm = 8\ninput_units = mil\ndepth_mm = 2\nfeed_mm_min = 300\n")
            .unwrap();
    assert!((s.fine_tool_mm - 0.2032).abs() < 1e-9 && (s.depth_mm - 0.0508).abs() < 1e-9);
    assert_eq!(s.feed_mm_min, 300.0);
    let s = Settings::parse("input_units = inch\nroute_tool_mm = 0.04\nsafe_z_mm = 2mm\n").unwrap();
    assert!((s.route_tool_mm - 1.016).abs() < 1e-9 && (s.safe_z_mm - 2.0).abs() < 1e-12);
    let mut s = Settings::default();
    s.set("--fine-tool-mm", "8mil").unwrap();
    assert!((s.fine_tool_mm - 0.2032).abs() < 1e-9);
}

#[test]
fn bad_new_settings_say_the_line_and_the_reason() {
    let cases = [
        ("forced_isolation = maybe", "line 1", "yes or no"),
        (
            "forced_isolation = yes\nforced_min_gap_mm = 5",
            "",
            "forced_min_gap_mm",
        ),
        ("forced_min_gap_mm = abc", "line 1", "not a length"),
        ("output_units = mil", "line 1", "mm or inch"),
        ("report_units = parsec", "line 1", "unit"),
        ("input_units = parsec", "line 1", "unit"),
        ("scale_x = 9", "", "scale_x"),
        ("scale_y = nan", "line 1", "not a number"),
        ("fiducial = 1,2,3", "line 1", "four numbers"),
        ("fiducial = 0,0,0,0\nfiducial = 10,0,15,0", "", "scale"),
        ("fiducial = 0,0,0,0", "", "at least 2"),
        (
            "fiducial_model = cubic",
            "line 1",
            "auto, similarity or affine",
        ),
        ("preview_cols = 3", "", "preview_cols"),
        ("preview_cols = -1", "line 1", "whole number"),
        ("spike_angle_deg = 10", "", "spike_angle_deg"),
        ("optimize_tolerance_mm = 7", "", "optimize_tolerance_mm"),
        ("mask_margin_mm = 4", "", "mask_margin_mm"),
        (
            "fiducial_max_residual_mm = 0",
            "",
            "fiducial_max_residual_mm",
        ),
    ];
    for (text, line, why) in cases {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(line) && e.contains(why), "{text:?}: {e}");
    }
    let many = "fiducial = 0,0,0,0\n".repeat(51);
    assert!(Settings::parse(&many).unwrap_err().contains("at most 50"));
}
