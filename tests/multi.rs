//! Several tools, the removal of the leftover copper, the direction of the cut, the starting points and the order of the paths, and the settings file.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::gcode::{write, write_with, Job};
use eh3d_isocam::hpgl::{write_with as hpgl_with, PenBy};
use eh3d_isocam::isolate::{plan, Pass, Plan, Rubout, Tool};
use eh3d_isocam::order::{
    arrange, directions, is_clockwise, travel_mm, Cut, Direction, LoopStart, Order, Strategy,
};

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn one(d: f64, passes: usize) -> Tool {
    Tool {
        diameter_mm: d,
        passes,
        stepover_mm: 0.1,
    }
}

fn plan_of(tools: Vec<Tool>, rubout: Option<Rubout>, trim: bool) -> Plan {
    Plan {
        tools,
        rubout,
        res_mm: 0.02,
        trim,
    }
}

fn square(x: f64, y: f64, s: f64) -> Cut {
    Cut {
        pts: vec![(x, y), (x + s, y), (x + s, y + s), (x, y + s)],
        closed: true,
    }
}

// ---- IC-2 -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_widest_tool_cuts_first_and_each_pass_remembers_its_tool() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let passes = plan(&s, &plan_of(vec![one(0.2, 1), one(0.4, 2)], None, false)).unwrap();
    assert_eq!(passes.len(), 3);
    assert_eq!(
        passes.iter().map(|p| p.diameter_mm).collect::<Vec<_>>(),
        vec![0.4, 0.4, 0.2]
    );
    assert_eq!(
        passes.iter().map(|p| p.tool).collect::<Vec<_>>(),
        vec![1, 1, 0],
        "the tool is the place in the plan, not in the order of cutting"
    );
    // without trimming every tool goes all the way round
    assert!(passes
        .iter()
        .all(|p| p.paths.len() == 1 && p.open.is_empty()));
    // the first pass of the wide tool is 0.2 mm from the copper
    let r: Vec<f64> = passes[0].paths[0].iter().map(|p| p.0.hypot(p.1)).collect();
    assert!(r.iter().all(|v| (v - 1.2).abs() < 0.03), "{r:?}");
}

#[test]
fn where_the_wide_tool_has_cut_the_fine_tool_does_not_go_again() {
    // two discs 0.3 mm apart: a tool of 0.4 mm does not fit in the gap, one of 0.2 mm does
    let s = scene(vec![disc(0.0, 0.0, 1.0), disc(2.3, 0.0, 1.0)]);
    let passes = plan(&s, &plan_of(vec![one(0.2, 1), one(0.4, 1)], None, true)).unwrap();
    let (wide, fine) = (&passes[0], &passes[1]);
    assert_eq!(wide.diameter_mm, 0.4);
    assert_eq!(wide.paths.len(), 1, "one path round both discs");
    assert!(
        fine.paths.is_empty(),
        "no path of the fine tool is whole any more"
    );
    assert!(
        !fine.open.is_empty(),
        "what is left is the stretch in the gap"
    );
    for run in &fine.open {
        assert!(run.len() >= 2);
        for p in run {
            assert!(p.0 > 0.2 && p.0 < 2.1 && p.1.abs() < 0.8, "{p:?}");
        }
    }
    // the stretches are along the gap: they are all near the line between the discs
    let length: f64 = fine.cuts().iter().map(Cut::length_mm).sum();
    assert!(length < 4.0, "{length}");
    // without trimming the fine tool goes all the way round both discs
    let whole = plan(&s, &plan_of(vec![one(0.2, 1), one(0.4, 1)], None, false)).unwrap();
    assert_eq!(whole[1].paths.len(), 2);
    assert!(whole[1].open.is_empty());
}

#[test]
fn far_from_the_gap_the_fine_tool_is_left_out_altogether() {
    let s = scene(vec![disc(0.0, 0.0, 1.0), disc(10.0, 0.0, 1.0)]);
    let passes = plan(&s, &plan_of(vec![one(0.2, 1), one(0.4, 1)], None, true)).unwrap();
    assert_eq!(passes[0].paths.len(), 2);
    assert!(passes[1].paths.is_empty() && passes[1].open.is_empty());
}

#[test]
fn the_rings_of_leftover_copper_overlap_and_keep_off_the_copper() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let rub = Rubout {
        tool_mm: 0.6,
        width_mm: 1.0,
        overlap: 0.2,
    };
    let passes = plan(&s, &plan_of(vec![one(0.2, 1)], Some(rub), false)).unwrap();
    let rings: Vec<&Pass> = passes.iter().filter(|p| p.rubout).collect();
    assert!(rings.len() >= 2, "{}", rings.len());
    assert!(!passes[0].rubout, "the isolation comes first");
    let rr = 0.3;
    // the tool never gets closer to the copper than its radius
    for ring in &rings {
        assert_eq!(ring.diameter_mm, 0.6);
        assert_eq!(ring.tool, 1);
        for p in ring.paths.iter().flatten() {
            assert!(p.0.hypot(p.1) - 1.0 >= rr - 0.03, "{p:?}");
        }
    }
    // no strip is left between the isolation and the first ring, nor between rings
    let edge = 0.2;
    assert!(rings[0].offset_mm - rr <= edge);
    for w in rings.windows(2) {
        assert!(w[1].offset_mm - rr <= w[0].offset_mm + rr);
        assert!(w[1].offset_mm > w[0].offset_mm);
    }
    // and they stop at the width asked for
    let last = rings[rings.len() - 1];
    assert!(last.offset_mm - rr < 1.0);
    assert!(last.offset_mm - rr >= 1.0 - 0.6 * 0.8 - 1e-9);
}

#[test]
fn a_ring_that_does_not_reach_a_narrow_place_is_simply_not_there() {
    // two discs 1 mm apart: the rings of a tool of 0.6 mm go in the gap only until they are squeezed out
    let s = scene(vec![disc(0.0, 0.0, 1.0), disc(3.0, 0.0, 1.0)]);
    let rub = Rubout {
        tool_mm: 0.6,
        width_mm: 2.0,
        overlap: 0.2,
    };
    let passes = plan(&s, &plan_of(vec![one(0.2, 1)], Some(rub), false)).unwrap();
    let rings: Vec<&Pass> = passes.iter().filter(|p| p.rubout).collect();
    assert!(rings.iter().any(|r| r.paths.len() == 2));
    assert!(
        rings.iter().any(|r| r.paths.len() == 1),
        "the outer rings go round both discs"
    );
}

#[test]
fn the_plan_is_checked() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let good = plan_of(vec![one(0.2, 1)], None, false);
    assert!(plan(&s, &good).is_ok());
    let mut bad = vec![
        Plan {
            tools: vec![],
            ..good.clone()
        },
        Plan {
            tools: vec![one(0.2, 1); 5],
            ..good.clone()
        },
        Plan {
            tools: vec![one(f64::NAN, 1)],
            ..good.clone()
        },
        Plan {
            tools: vec![one(0.2, 0)],
            ..good.clone()
        },
        Plan {
            tools: vec![one(0.2, 11)],
            ..good.clone()
        },
        Plan {
            tools: vec![one(0.2, 10); 5],
            ..good.clone()
        },
        Plan {
            res_mm: 0.5,
            ..good.clone()
        },
    ];
    for (tool, width, overlap) in [
        (0.05, 1.0, 0.2),
        (0.6, 0.0, 0.2),
        (0.6, 1.0, 0.95),
        (0.6, 1.0, -0.1),
        (f64::NAN, 1.0, 0.2),
    ] {
        bad.push(Plan {
            rubout: Some(Rubout {
                tool_mm: tool,
                width_mm: width,
                overlap,
            }),
            ..good.clone()
        });
    }
    for b in &bad {
        assert!(plan(&s, b).is_err(), "{b:?}");
    }
    // a layer with no copper is an error too
    assert!(plan(&scene(vec![]), &good).is_err());
}

#[test]
fn the_gcode_stops_for_a_tool_change_and_the_plot_file_gives_each_tool_its_pen() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let passes = plan(&s, &plan_of(vec![one(0.2, 1), one(0.4, 1)], None, false)).unwrap();
    let g = write(&passes, &Job::default()).unwrap();
    assert_eq!(g.matches("M0\n").count(), 1, "{g}");
    assert!(g.contains("(change to a tool of 0.2 mm;"));
    // the tool goes up to the height for the change before it stops, and back to the safe height after it, with the zero to be set again
    assert!(
        g.contains(
            "G0 Z30
(change to a tool"
        ),
        "{g}"
    );
    assert!(g.contains("set the Z zero again"), "{g}");
    let first_change = g.find("M0\n").unwrap();
    assert!(g.find("pass 1 at 0.2 mm").unwrap() < first_change);
    let h = hpgl_with(&passes, &Order::default(), PenBy::Tool).unwrap();
    // the wide tool is the second of the plan, so it has pen 2; the fine one pen 1
    assert!(h.find("SP2;").unwrap() < h.find("SP1;").unwrap(), "{h}");
    let by_pass = hpgl_with(&passes, &Order::default(), PenBy::Pass).unwrap();
    assert!(by_pass.contains("SP1;") && by_pass.contains("SP2;"));
    let many = vec![Pass {
        tool: 8,
        ..Pass::default()
    }];
    assert!(hpgl_with(&many, &Order::default(), PenBy::Tool).is_err());
}

#[test]
fn open_runs_are_cut_without_closing_them() {
    let pass = Pass {
        diameter_mm: 0.2,
        open: vec![vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0)]],
        ..Pass::default()
    };
    let g = write(std::slice::from_ref(&pass), &Job::default()).unwrap();
    assert!(
        g.contains("G0 X1 Y1\nG1 Z-0.06 F60\nG1 X2 Y1 F300\nG1 X2 Y2\nG0 Z2\n"),
        "{g}"
    );
    let h = hpgl_with(std::slice::from_ref(&pass), &Order::default(), PenBy::Pass).unwrap();
    assert!(h.contains("PU40,40;\nPD80,40,80,80;"), "{h}");
}

// ---- IC-3 -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_copper_is_on_the_right_of_the_tool_round_an_island_and_on_the_right_in_a_hole() {
    // a hollow square of copper: four tracks
    let t = |x0, y0, x1, y1| Shape::Stroke {
        x0,
        y0,
        x1,
        y1,
        w: 0.5,
    };
    let frame = scene(vec![
        t(0.0, 0.0, 6.0, 0.0),
        t(6.0, 0.0, 6.0, 6.0),
        t(6.0, 6.0, 0.0, 6.0),
        t(0.0, 6.0, 0.0, 0.0),
    ]);
    let passes = plan(&frame, &plan_of(vec![one(0.2, 1)], None, false)).unwrap();
    assert_eq!(passes[0].paths.len(), 2);
    let (outer, inner): (Vec<_>, Vec<_>) = passes[0]
        .paths
        .iter()
        .partition(|p| p.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max) > 6.0);
    assert!(
        is_clockwise(outer[0]),
        "round the outside of the frame the tool goes clockwise"
    );
    assert!(
        !is_clockwise(inner[0]),
        "inside the frame the copper is outside the path, so it goes counterclockwise"
    );
    // the same for a plain disc
    let d = plan(
        &scene(vec![disc(0.0, 0.0, 1.0)]),
        &plan_of(vec![one(0.2, 1)], None, false),
    )
    .unwrap();
    assert!(is_clockwise(&d[0].paths[0]));
}

#[test]
fn the_direction_can_be_shown_and_changed() {
    let d = plan(
        &scene(vec![disc(0.0, 0.0, 1.0), disc(5.0, 0.0, 1.0)]),
        &plan_of(vec![one(0.2, 1)], None, false),
    )
    .unwrap();
    let cuts = d[0].cuts();
    let order = |direction| Order {
        direction,
        ..Order::default()
    };
    assert_eq!(
        directions(&arrange(&cuts, &order(Direction::Climb), (0.0, 0.0)).0),
        (2, 0, 0)
    );
    assert_eq!(
        directions(&arrange(&cuts, &order(Direction::Conventional), (0.0, 0.0)).0),
        (0, 2, 0)
    );
    assert_eq!(
        directions(&arrange(&cuts, &order(Direction::CounterClockwise), (0.0, 0.0)).0),
        (0, 2, 0)
    );
    assert_eq!(
        directions(&arrange(&cuts, &order(Direction::Clockwise), (0.0, 0.0)).0),
        (2, 0, 0)
    );
    // forcing clockwise on paths that go the other way turns them
    let ccw: Vec<Cut> = cuts
        .iter()
        .map(|c| Cut {
            pts: c.pts.iter().rev().copied().collect(),
            closed: true,
        })
        .collect();
    assert_eq!(
        directions(&arrange(&ccw, &order(Direction::Clockwise), (0.0, 0.0)).0),
        (2, 0, 0)
    );
    // an open run keeps its way when a way round is forced, and is turned by conventional milling
    let run = Cut {
        pts: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        closed: false,
    };
    assert_eq!(
        arrange(
            std::slice::from_ref(&run),
            &order(Direction::Clockwise),
            (0.0, 0.0)
        )
        .0[0]
            .pts,
        run.pts
    );
    let turned = &arrange(
        std::slice::from_ref(&run),
        &order(Direction::Conventional),
        (0.0, 0.0),
    )
    .0[0];
    assert_eq!(turned.pts, vec![(1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]);
    // in the G-code the other direction makes different moves
    let climb = write_with(&d, &Job::default(), &order(Direction::Climb)).unwrap();
    let conventional = write_with(&d, &Job::default(), &order(Direction::Conventional)).unwrap();
    assert_ne!(climb, conventional);
}

#[test]
fn each_closed_path_starts_where_the_settings_say() {
    let c = square(2.0, 3.0, 4.0);
    let rotated = Cut {
        pts: vec![c.pts[2], c.pts[3], c.pts[0], c.pts[1]],
        closed: true,
    };
    let with = |loop_start| Order {
        loop_start,
        ..Order::default()
    };
    let first =
        |cut: &Cut, o: &Order, from| arrange(std::slice::from_ref(cut), o, from).0[0].pts[0];
    assert_eq!(
        first(&rotated, &with(LoopStart::Original), (0.0, 0.0)),
        (6.0, 7.0)
    );
    assert_eq!(
        first(&rotated, &with(LoopStart::LowerLeft), (50.0, 50.0)),
        (2.0, 3.0)
    );
    assert_eq!(
        first(&rotated, &with(LoopStart::Nearest), (50.0, 50.0)),
        (6.0, 7.0)
    );
    assert_eq!(
        first(&rotated, &with(LoopStart::Nearest), (0.0, 0.0)),
        (2.0, 3.0)
    );
    // the path is the same one, only started elsewhere
    let r = arrange(
        std::slice::from_ref(&rotated),
        &with(LoopStart::LowerLeft),
        (0.0, 0.0),
    )
    .0
    .remove(0);
    let mut a = r.pts.clone();
    let mut b = rotated.pts.clone();
    a.sort_by(|x, y| x.partial_cmp(y).unwrap());
    b.sort_by(|x, y| x.partial_cmp(y).unwrap());
    assert_eq!(a, b);
}

#[test]
fn the_tool_starts_from_the_point_chosen() {
    let cuts = vec![square(0.0, 0.0, 1.0), square(100.0, 100.0, 1.0)];
    let near_origin = arrange(&cuts, &Order::default(), (0.0, 0.0)).0;
    assert_eq!(near_origin[0].pts[0], (0.0, 0.0));
    let from_far = arrange(
        &cuts,
        &Order {
            start: (200.0, 200.0),
            ..Order::default()
        },
        (200.0, 200.0),
    )
    .0;
    assert!(
        from_far[0].pts[0].0 >= 100.0,
        "the nearest to the far corner is cut first"
    );
    let g = write_with(
        &plan(
            &scene(vec![disc(0.0, 0.0, 1.0), disc(50.0, 50.0, 1.0)]),
            &plan_of(vec![one(0.2, 1)], None, false),
        )
        .unwrap(),
        &Job::default(),
        &Order {
            start: (60.0, 60.0),
            ..Order::default()
        },
    )
    .unwrap();
    let first_move = g.lines().find(|l| l.starts_with("G0 X")).unwrap();
    let x: f64 = first_move[4..first_move.find(" Y").unwrap()]
        .parse()
        .unwrap();
    assert!(x > 40.0, "{first_move}");
    let bad = Order {
        start: (f64::NAN, 0.0),
        ..Order::default()
    };
    assert!(write_with(&[], &Job::default(), &bad).is_err());
}

/// A grid of small squares visited in a scrambled order.
fn scrambled(n: usize) -> Vec<Cut> {
    let mut v: Vec<(usize, usize)> = (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).collect();
    let mut seed = 12345u64;
    for i in (1..v.len()).rev() {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        v.swap(i, (seed >> 33) as usize % (i + 1));
    }
    v.into_iter()
        .map(|(i, j)| square(i as f64 * 5.0, j as f64 * 5.0, 1.0))
        .collect()
}

#[test]
fn ordering_shortens_the_travel_through_the_air() {
    let cuts = scrambled(7);
    let o = |strategy| Order {
        strategy,
        loop_start: LoopStart::Original,
        ..Order::default()
    };
    let original = arrange(&cuts, &o(Strategy::Original), (0.0, 0.0)).0;
    let nearest = arrange(&cuts, &o(Strategy::Nearest), (0.0, 0.0)).0;
    let optimized = arrange(&cuts, &o(Strategy::Optimized), (0.0, 0.0)).0;
    let (a, b, c) = (
        travel_mm(&original, (0.0, 0.0)),
        travel_mm(&nearest, (0.0, 0.0)),
        travel_mm(&optimized, (0.0, 0.0)),
    );
    assert!(b < a / 3.0, "nearest {b} against {a}");
    assert!(c <= b + 1e-9, "optimized {c} against nearest {b}");
    // every path is still cut exactly once
    for ordered in [&original, &nearest, &optimized] {
        assert_eq!(ordered.len(), cuts.len());
        let mut s: Vec<_> = ordered.iter().map(|c| format!("{:?}", c.pts)).collect();
        let mut t: Vec<_> = cuts.iter().map(|c| format!("{:?}", c.pts)).collect();
        s.sort();
        t.sort();
        assert_eq!(s, t);
    }
    // too many paths for the exchange of stretches: the nearest order is kept
    let limited = Order {
        optimize_max_paths: 5,
        ..o(Strategy::Optimized)
    };
    assert_eq!(arrange(&cuts, &limited, (0.0, 0.0)).0, nearest);
}

#[test]
fn the_travel_is_measured_between_where_a_cut_ends_and_the_next_begins() {
    let a = square(0.0, 0.0, 1.0);
    let b = square(10.0, 0.0, 1.0);
    // a closed path ends where it began
    assert!((travel_mm(&[a.clone(), b.clone()], (0.0, 0.0)) - 10.0).abs() < 1e-12);
    let run = Cut {
        pts: vec![(0.0, 0.0), (3.0, 4.0)],
        closed: false,
    };
    // an open run ends at its last point
    assert!(
        (travel_mm(&[run, b], (0.0, 0.0)) - (0.0 + (7.0f64.powi(2) + 4.0f64.powi(2)).sqrt())).abs()
            < 1e-12
    );
    assert_eq!(travel_mm(&[], (1.0, 1.0)), 0.0);
}

#[test]
fn damaged_cuts_are_dropped_and_nothing_panics() {
    let cuts = vec![
        Cut {
            pts: vec![],
            closed: true,
        },
        Cut {
            pts: vec![(0.0, 0.0), (1.0, 1.0)],
            closed: true,
        },
        Cut {
            pts: vec![(f64::NAN, 0.0), (1.0, 1.0), (2.0, 0.0)],
            closed: true,
        },
        Cut {
            pts: vec![(0.0, 0.0)],
            closed: false,
        },
        square(0.0, 0.0, 1.0),
    ];
    for strategy in [Strategy::Original, Strategy::Nearest, Strategy::Optimized] {
        let (out, end) = arrange(
            &cuts,
            &Order {
                strategy,
                ..Order::default()
            },
            (3.0, 3.0),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(end, out[0].pts[0]);
    }
    let (none, end) = arrange(&[], &Order::default(), (7.0, 8.0));
    assert!(none.is_empty() && end == (7.0, 8.0));
}

// ---- the settings file ----------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_settings_are_written_and_read_back() {
    let d = Settings::default();
    assert_eq!(Settings::parse(&d.to_text()).unwrap(), d);
    let text = "# my machine\nfine_tool_mm = 0.15   # a V bit\ncoarse_tool_mm = 0.5\ndirection = conventional\nordering = optimized\nloop_start = lower_left\nstart_x_mm = 10\nmirror = vertical\npanel_cols = 3\nrubout_tool_mm = 0.8\ntrim_fine = no\n";
    let s = Settings::parse(text).unwrap();
    assert_eq!(s.fine_tool_mm, 0.15);
    assert_eq!(s.direction, Direction::Conventional);
    assert_eq!(s.order().strategy, Strategy::Optimized);
    assert_eq!(s.order().loop_start, LoopStart::LowerLeft);
    assert_eq!(s.order().start, (10.0, 0.0));
    assert_eq!(s.plan().unwrap().tools.len(), 2);
    assert!(s.plan().unwrap().rubout.is_some() && !s.plan().unwrap().trim);
    assert_eq!(s.layout().cols, 3);
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    // the names of the old options and the dashes of the command line
    let mut t = Settings::default();
    t.set("--tool", "0.3").unwrap();
    t.set("--fine-stepover-mm", "0.05").unwrap();
    t.set("rpm", "30000").unwrap();
    assert_eq!(
        (t.fine_tool_mm, t.fine_stepover_mm, t.spindle_rpm),
        (0.3, 0.05, 30000.0)
    );
}

#[test]
fn a_bad_setting_is_an_error_with_its_line_and_never_a_panic() {
    for (text, part) in [
        ("fine_tool_mm = abc", "line 1"),
        ("\n\nnothing here", "line 3"),
        ("colour = red", "unknown setting"),
        ("direction = sideways", "climb, conventional"),
        ("ordering = random", "original, nearest or optimized"),
        ("loop_start = middle", "lower_left"),
        ("mirror = diagonal", "vertical"),
        ("trim_fine = maybe", "yes or no"),
        ("fine_passes = -1", "whole number"),
        ("fine_tool_mm = 0", "tool_diameter_mm"),
        ("fine_tool_mm = 0.2\nres_mm = 0.5", "too coarse"),
        ("alignment_holes = 4", "2 or 3"),
        ("panel_cols = 0", "panel"),
        ("panel_cols = 20\npanel_rows = 20", "at most 100"),
        ("rotate_quarters = 4", "0 to 3"),
        ("rubout_tool_mm = 0.6\nrubout_overlap = 1", "rubout_overlap"),
        ("depth_mm = 0", "depth_mm"),
        ("optimize_max_paths = 0", "optimize_max_paths"),
        ("rule_min_width_mm = 100", "rule_min_width_mm"),
        ("start_x_mm = nan", "not a number"),
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?} gave {e:?}");
    }
    for text in ["\u{0}", "=", "= =", "a=b=c", "\r\n\r\n", "#"] {
        let _ = Settings::parse(text);
    }
}

#[test]
fn the_example_settings_file_holds_every_default() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/isocam.cfg"))
        .unwrap();
    assert_eq!(Settings::parse(&text).unwrap(), Settings::default());
}

// ---- tool round pads only ---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_wide_tool_cuts_only_round_the_big_pads_when_asked() {
    use eh3d_isocam::isolate::plan_with_pads;
    let s = scene(vec![
        disc(0.0, 0.0, 1.0),
        Shape::Stroke {
            x0: 6.0,
            y0: 0.0,
            x1: 9.0,
            y1: 0.0,
            w: 0.2,
        },
    ]);
    let p = plan_of(vec![one(0.2, 1), one(0.4, 1)], None, true);
    let all = plan(&s, &p).unwrap();
    let near = |pass: &Pass| {
        pass.paths
            .iter()
            .chain(pass.open.iter())
            .flatten()
            .all(|q| q.0.hypot(q.1) < 3.0)
    };
    // without the setting the wide tool also goes round the track
    assert!(all
        .iter()
        .filter(|x| x.diameter_mm == 0.4)
        .any(|x| !near(x)));
    let around = plan_with_pads(&s, &p, Some(0.8)).unwrap();
    let wide: Vec<&Pass> = around.iter().filter(|x| x.diameter_mm == 0.4).collect();
    assert!(!wide.is_empty() && wide.iter().all(|x| near(x)));
    // the fine tool still goes round the track
    let fine: Vec<&Pass> = around.iter().filter(|x| x.diameter_mm == 0.2).collect();
    assert!(fine.iter().any(|x| !near(x)));
}

#[test]
fn small_pads_are_left_to_the_fine_tool_and_a_bad_minimum_is_refused() {
    use eh3d_isocam::isolate::plan_with_pads;
    let s = scene(vec![disc(0.0, 0.0, 0.3)]);
    let p = plan_of(vec![one(0.2, 1), one(0.4, 1)], None, true);
    let a = plan_with_pads(&s, &p, Some(1.0)).unwrap();
    assert!(a
        .iter()
        .filter(|x| x.diameter_mm == 0.4)
        .all(|x| x.paths.is_empty() && x.open.is_empty()));
    assert!(plan_with_pads(&s, &p, Some(0.0)).is_err());
    assert!(plan_with_pads(&s, &p, Some(f64::NAN)).is_err());
}

#[test]
fn the_setting_around_pads_min_is_read_and_printed() {
    let mut st = Settings::default();
    assert_eq!(st.around_pads(), None);
    st.set("around_pads_min_mm", "1.5").unwrap();
    assert_eq!(st.around_pads(), Some(1.5));
    assert!(st.to_text().contains("around_pads_min_mm = 1.5"));
}
