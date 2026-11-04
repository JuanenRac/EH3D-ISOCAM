//! The holes and the outline of a board: Excellon read and written, the outline read from Gerber, DXF-less plotter files, the path of the tool round the board.

use eh3d_bungard_formats::{hpgl as plotter, ncd};
use eh3d_isocam::drill::{
    add_tabs, join_runs, read_drills, read_outline, route_outline, write_drills,
    write_route_excellon, Hole, Outline, Tabs,
};
use eh3d_isocam::hpgl::{write_with, PenBy};
use eh3d_isocam::order::{arrange, is_clockwise, Cut, Order};

fn hole(tool: u8, x: f64, y: f64, d: f64) -> Hole {
    Hole {
        tool,
        x_mm: x,
        y_mm: y,
        diameter_mm: d,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

#[test]
fn a_bungard_drill_file_in_mils_is_read_with_its_tools() {
    let text = "M48\nINCH\nT1C0.0315\nT2C0.0394\n%\nT1\nX1000Y2000\nX-500Y0\nT2\nX3937Y1\nM30\n";
    let d = read_drills(text).unwrap();
    assert_eq!(d.holes.len(), 3);
    assert!((d.holes[0].x_mm - 25.4).abs() < 1e-9 && (d.holes[0].y_mm - 50.8).abs() < 1e-9);
    assert!((d.holes[1].x_mm + 12.7).abs() < 1e-9);
    assert!((d.holes[0].diameter_mm - 0.8001).abs() < 1e-3);
    assert_eq!(d.holes[2].tool, 2);
    assert!((d.holes[2].diameter_mm - 1.0).abs() < 0.01);
    // the bare body of an old teach-in file has no diameters
    let bare = read_drills("%\nT06\nX100Y200\n").unwrap();
    assert_eq!((bare.holes[0].tool, bare.holes[0].diameter_mm), (6, 0.0));
}

#[test]
fn a_metric_file_with_decimal_coordinates_is_read_with_the_reader_of_the_suite() {
    let text = "M48\nMETRIC,TZ\nT1C0.8\nT2C3.2\n%\nT1\nX10.5Y20.25\nX11Y21\nT2\nX1.5Y2.5\nM30\n";
    let d = read_drills(text).unwrap();
    assert_eq!(d.holes.len(), 3);
    assert!((d.holes[0].x_mm - 10.5).abs() < 1e-9 && (d.holes[0].y_mm - 20.25).abs() < 1e-9);
    assert!((d.holes[0].diameter_mm - 0.8).abs() < 1e-9);
    assert!((d.holes[2].diameter_mm - 3.2).abs() < 1e-9);
}

#[test]
fn drills_written_are_read_back_the_same() {
    let holes = vec![
        hole(1, 5.08, 7.62, 0.8),
        hole(1, 10.16, -2.54, 0.8),
        hole(2, 0.0, 0.0, 3.0),
        hole(7, 100.0, 50.0, 0.0),
    ];
    let text = write_drills(&holes).unwrap();
    let back = read_drills(&text).unwrap();
    assert_eq!(back.holes.len(), 4);
    for (a, b) in holes.iter().zip(&back.holes) {
        assert_eq!(a.tool, b.tool);
        assert!(
            (a.x_mm - b.x_mm).abs() <= ncd::MIL_MM / 2.0 + 1e-9,
            "{a:?} {b:?}"
        );
        assert!((a.y_mm - b.y_mm).abs() <= ncd::MIL_MM / 2.0 + 1e-9);
        assert!((a.diameter_mm - b.diameter_mm).abs() < 0.01);
    }
}

#[test]
fn damaged_drill_files_are_errors_and_never_panic() {
    for text in [
        "",
        "%",
        "M48\n%\n",
        "T1\nXabc\n",
        "T1C\n%\nT1\nX1Y1\n",
        "X10Y10\n",
        "\u{0}\u{1}\u{2}",
        "M48\nMETRIC\nT1C0.8\n%\nT1\nX99999999999Y1\nM30\n",
        "M48\nINCH\nT999999999999\n%\n",
    ] {
        let _ = read_drills(text);
    }
    assert!(read_drills("").is_err() && read_drills("M48\n%\n").is_err());
    // a hole farther away than any board is refused
    assert!(read_drills("%\nT1\nX999999999Y1\n").is_err());
}

#[test]
fn drills_that_cannot_be_written_say_why() {
    assert!(write_drills(&[]).unwrap_err().contains("no holes"));
    assert!(write_drills(&[hole(0, 1.0, 1.0, 0.8)])
        .unwrap_err()
        .contains("tool number 0"));
    assert!(
        write_drills(&[hole(1, 1.0, 1.0, 0.8), hole(1, 2.0, 2.0, 1.2)])
            .unwrap_err()
            .contains("tool 1")
    );
    assert!(write_drills(&[hole(1, f64::NAN, 1.0, 0.8)]).is_err());
    assert!(write_drills(&[hole(1, 1.0, 1e9, 0.8)]).is_err());
    assert!(write_drills(&[hole(1, 1.0, 1.0, -1.0)]).is_err());
}

#[test]
fn the_routing_excellon_puts_the_tool_down_moves_and_lifts() {
    let cut = Cut {
        pts: rect(0.0, 0.0, 10.0, 5.0),
        closed: true,
    };
    let open = Cut {
        pts: vec![(1.0, 1.0), (2.0, 1.0)],
        closed: false,
    };
    let text = write_route_excellon(&[cut, open], 1.0, 3).unwrap();
    assert!(text.starts_with("M48\nINCH\nT3C0.0394\n%\nT3\n"), "{text}");
    assert_eq!(text.matches("M15").count(), 2);
    assert_eq!(text.matches("M16").count(), 2);
    // the closed path comes back to where it began: 4 corners and the first again; the open run has its two points
    assert_eq!(text.matches("G01").count(), 4 + 1);
    assert!(text.contains("G00X0Y0\n") && text.ends_with("M30\n"));
    // the reader of the drill files finds no holes in it
    assert!(ncd::parse(&text).unwrap().holes.is_empty());
    assert!(write_route_excellon(&[], 1.0, 1).is_err());
    assert!(write_route_excellon(
        &[Cut {
            pts: rect(0.0, 0.0, 1.0, 1.0),
            closed: true
        }],
        0.0,
        1
    )
    .is_err());
    assert!(write_route_excellon(
        &[Cut {
            pts: rect(0.0, 0.0, 1.0, 1.0),
            closed: true
        }],
        1.0,
        0
    )
    .is_err());
    assert!(write_route_excellon(
        &[Cut {
            pts: vec![(f64::NAN, 0.0), (1.0, 1.0)],
            closed: false
        }],
        1.0,
        1
    )
    .is_err());
}

#[test]
fn strokes_drawn_one_by_one_join_into_a_closed_outline() {
    // the four sides of a rectangle, out of order, some of them backwards, and a loose run
    let runs = vec![
        vec![(10.0, 0.0), (10.0, 5.0)],
        vec![(0.0, 0.0), (10.0, 0.0)],
        vec![(0.0, 5.0), (10.0, 5.0)],
        vec![(0.0, 0.0), (0.0, 5.0)],
        vec![(50.0, 50.0), (60.0, 50.0)],
    ];
    let o = join_runs(runs, 0.01);
    assert_eq!(o.closed.len(), 1, "{o:?}");
    assert_eq!(o.closed[0].len(), 4);
    assert_eq!(o.open.len(), 1);
    let b = o.bounds();
    assert!((b.min_x, b.min_y, b.max_x, b.max_y) == (0.0, 0.0, 60.0, 50.0));
    assert!(join_runs(vec![], 0.01).closed.is_empty());
    assert!(join_runs(vec![vec![(0.0, 0.0)]], 0.01).open.is_empty());
}

#[test]
fn an_outline_is_read_from_gerber_and_from_a_plotter_file() {
    let g = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";
    let o = read_outline("board.gko", g).unwrap();
    assert_eq!(o.closed.len(), 1);
    let b = o.bounds();
    assert!((b.max_x - 50.0).abs() < 0.03 && (b.max_y - 30.0).abs() < 0.03);
    // 800 x 400 units of 0.025 mm
    let p = read_outline("board.PLT", "IN;SP1;PU0,0;PD800,0,800,400,0,400,0,0;").unwrap();
    assert_eq!(p.closed.len(), 1);
    assert!((p.bounds().max_x - 20.0).abs() < 1e-9 && (p.bounds().max_y - 10.0).abs() < 1e-9);
    for bad in ["", "nothing", "%FSLAX24Y24*%\nM02*"] {
        assert!(read_outline("x.gbr", bad).is_err(), "{bad:?}");
    }
    assert!(read_outline("x.plt", "").is_err());
    assert!(read_outline("x.dxf", "garbage").is_err());
}

#[test]
fn the_tool_goes_round_the_board_outside_and_inside_the_cutouts() {
    let o = Outline {
        closed: vec![rect(0.0, 0.0, 20.0, 10.0), rect(8.0, 3.0, 12.0, 7.0)],
        open: vec![],
    };
    let pass = route_outline(&o, 1.0, 0.05).unwrap();
    assert_eq!(pass.paths.len(), 2, "round the board and inside the cutout");
    let extent = |p: &Vec<(f64, f64)>| {
        (
            p.iter().map(|q| q.0).fold(f64::INFINITY, f64::min),
            p.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max),
            p.iter().map(|q| q.1).fold(f64::INFINITY, f64::min),
            p.iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (outer, inner) = if extent(&pass.paths[0]).1 > 15.0 {
        (&pass.paths[0], &pass.paths[1])
    } else {
        (&pass.paths[1], &pass.paths[0])
    };
    let e = extent(outer);
    // the centre of the tool is half a tool outside the board
    assert!(
        (e.0 + 0.5).abs() < 0.06 && (e.1 - 20.5).abs() < 0.06,
        "{e:?}"
    );
    assert!(
        (e.2 + 0.5).abs() < 0.06 && (e.3 - 10.5).abs() < 0.06,
        "{e:?}"
    );
    let i = extent(inner);
    // inside the cutout the tool stays half a tool away from the board, so the path is smaller than the hole
    assert!(
        (i.0 - 8.5).abs() < 0.06 && (i.1 - 11.5).abs() < 0.06,
        "{i:?}"
    );
    assert!(
        (i.2 - 3.5).abs() < 0.06 && (i.3 - 6.5).abs() < 0.06,
        "{i:?}"
    );
    // the board stays on the right of the tool: clockwise round the board, counterclockwise in the cutout
    assert!(is_clockwise(outer) && !is_clockwise(inner));
    assert!((pass.diameter_mm - 1.0).abs() < 1e-12);
}

#[test]
fn the_route_is_refused_when_it_cannot_be_made() {
    let closed = Outline {
        closed: vec![rect(0.0, 0.0, 5.0, 5.0)],
        open: vec![],
    };
    assert!(route_outline(&Outline::default(), 1.0, 0.05).is_err());
    assert!(route_outline(
        &Outline {
            closed: vec![],
            open: vec![rect(0.0, 0.0, 5.0, 5.0)]
        },
        1.0,
        0.05
    )
    .unwrap_err()
    .contains("no closed path"));
    for (tool, res) in [
        (0.0, 0.05),
        (f64::NAN, 0.05),
        (1.0, 0.0),
        (1.0, 0.9),
        (20.0, 0.05),
    ] {
        assert!(route_outline(&closed, tool, res).is_err(), "{tool} {res}");
    }
    // a board too big for the grid says so instead of running out of memory
    let huge = Outline {
        closed: vec![rect(0.0, 0.0, 5000.0, 5000.0)],
        open: vec![],
    };
    assert!(route_outline(&huge, 1.0, 0.002).is_err());
}

#[test]
fn the_route_as_hpgl_is_read_back_by_the_reader_of_the_plot_files() {
    let o = Outline {
        closed: vec![rect(0.0, 0.0, 20.0, 10.0)],
        open: vec![],
    };
    let pass = route_outline(&o, 1.0, 0.05).unwrap();
    let order = Order::default();
    let text = write_with(std::slice::from_ref(&pass), &order, PenBy::Pass).unwrap();
    let plot = plotter::parse(&text);
    assert_eq!(plot.paths.len(), 1);
    assert_eq!(plot.paths[0].pen, 1);
    // the path was moved so that nothing is negative: its box is 21 x 11 mm
    let (x0, y0, x1, y1) = plot.bounds_mm().unwrap();
    assert!(x0 >= 0.0 && y0 >= 0.0);
    assert!(((x1 - x0) - 21.0).abs() < 0.1 && ((y1 - y0) - 11.0).abs() < 0.1);
    // the same path as an Excellon routing file
    let (cuts, _) = arrange(&pass.cuts(), &order, order.start);
    assert!(write_route_excellon(&cuts, 1.0, 1).unwrap().contains("M15"));
}

fn path_length(p: &[(f64, f64)], closed: bool) -> f64 {
    let mut s: f64 = p
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum();
    if closed {
        s += (p[0].0 - p[p.len() - 1].0).hypot(p[0].1 - p[p.len() - 1].1);
    }
    s
}

#[test]
fn bridges_open_the_cut_round_the_board_into_runs_with_gaps() {
    let o = Outline {
        closed: vec![rect(0.0, 0.0, 40.0, 20.0)],
        open: vec![],
    };
    let mut pass = route_outline(&o, 1.0, 0.05).unwrap();
    let whole = path_length(&pass.paths[0], true);
    let tabs = Tabs {
        count: 4,
        width_mm: 2.0,
        min_length_mm: 4.0,
    };
    let r = add_tabs(&mut pass, 1.0, &tabs).unwrap();
    assert_eq!((r.bridges, r.paths_cut, r.paths_left_closed), (4, 1, 0));
    assert!(pass.paths.is_empty());
    assert_eq!(pass.open.len(), 4);
    // every bridge takes its width and one tool out of the path
    let left: f64 = pass.open.iter().map(|p| path_length(p, false)).sum();
    assert!((whole - left - 4.0 * 3.0).abs() < 1e-6, "{whole} {left}");
    // the runs are cuts like any other: they reach the routing file
    let text = write_route_excellon(&pass.cuts(), 1.0, 1).unwrap();
    assert_eq!(text.matches("M15").count(), 4);
}

#[test]
fn a_path_gets_fewer_bridges_when_the_pieces_would_be_too_short() {
    let o = Outline {
        closed: vec![rect(0.0, 0.0, 40.0, 20.0)],
        open: vec![],
    };
    let base = route_outline(&o, 1.0, 0.05).unwrap();
    let total = path_length(&base.paths[0], true);
    // a piece of at least 50 mm: two bridges at most on about 124 mm
    let mut pass = base.clone();
    let tabs = Tabs {
        count: 6,
        width_mm: 1.0,
        min_length_mm: 50.0,
    };
    let r = add_tabs(&mut pass, 1.0, &tabs).unwrap();
    assert_eq!(r.bridges, (total / 50.0).floor() as usize);
    // longer than the path itself: nothing is opened and it is said
    let mut pass = base.clone();
    let tabs = Tabs {
        count: 3,
        width_mm: 1.0,
        min_length_mm: 500.0,
    };
    let r = add_tabs(&mut pass, 1.0, &tabs).unwrap();
    assert_eq!((r.bridges, r.paths_left_closed), (0, 1));
    assert_eq!(pass.paths.len(), 1);
    // a bridge wider than the span between two bridges takes the bridges back
    let mut pass = base.clone();
    let tabs = Tabs {
        count: 4,
        width_mm: 40.0,
        min_length_mm: 4.0,
    };
    let r = add_tabs(&mut pass, 1.0, &tabs).unwrap();
    assert!(r.bridges < 4 && r.bridges >= 1, "{r:?}");
}

#[test]
fn bridges_have_limits() {
    let o = Outline {
        closed: vec![rect(0.0, 0.0, 40.0, 20.0)],
        open: vec![],
    };
    let base = route_outline(&o, 1.0, 0.05).unwrap();
    let ok = Tabs {
        count: 2,
        width_mm: 1.0,
        min_length_mm: 4.0,
    };
    // none asked for: nothing changes
    let mut pass = base.clone();
    let none = Tabs { count: 0, ..ok };
    assert_eq!(add_tabs(&mut pass, 1.0, &none).unwrap().bridges, 0);
    assert_eq!(pass, base);
    for (tool, tabs) in [
        (0.0, ok),
        (
            1.0,
            Tabs {
                width_mm: 0.05,
                ..ok
            },
        ),
        (
            1.0,
            Tabs {
                width_mm: f64::NAN,
                ..ok
            },
        ),
        (
            1.0,
            Tabs {
                min_length_mm: 1.9,
                ..ok
            },
        ),
    ] {
        assert!(
            add_tabs(&mut base.clone(), tool, &tabs).is_err(),
            "{tool} {tabs:?}"
        );
    }
}

#[test]
fn a_tool_number_above_255_is_renumbered_with_a_notice_and_two_tools_never_merge() {
    let text = "M48\nMETRIC\nT1C0.8\nT300C1.2\nT301C1.6\n%\nT1\nX1.0Y1.0\nT300\nX2.0Y2.0\nT301\nX3.0Y3.0\nM30\n";
    let d = read_drills(text).unwrap();
    assert_eq!(d.holes.len(), 3);
    let tools: Vec<u8> = d.holes.iter().map(|h| h.tool).collect();
    assert_eq!(tools[0], 1);
    assert!(
        tools[1] != tools[2] && tools[1] != 1 && tools[2] != 1,
        "{tools:?}"
    );
    // each hole keeps its own diameter
    assert!((d.holes[1].diameter_mm - 1.2).abs() < 1e-9);
    assert!((d.holes[2].diameter_mm - 1.6).abs() < 1e-9);
    let notices = d
        .notes
        .iter()
        .filter(|n| n.starts_with("tool-number-too-large"))
        .count();
    assert_eq!(notices, 2, "{:?}", d.notes);
    // a file within the range says nothing
    assert!(read_drills("M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n")
        .unwrap()
        .notes
        .iter()
        .all(|n| !n.starts_with("tool-number-too-large")));
}

#[test]
fn fifty_thousand_segments_in_any_order_join_into_one_path_in_moments() {
    let n = 50_000_usize;
    // the segments of a zig-zag line, in an order that is not the order of the line
    let mut order: Vec<usize> = (0..n).collect();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        order.swap(i, (state % (i as u64 + 1)) as usize);
    }
    let point = |k: usize| (k as f64 * 0.5, if k % 2 == 0 { 0.0 } else { 0.3 });
    let runs: Vec<Vec<(f64, f64)>> = order.iter().map(|&k| vec![point(k), point(k + 1)]).collect();
    let started = std::time::Instant::now();
    let o = join_runs(runs, 0.01);
    assert!(
        started.elapsed().as_secs() < 30,
        "{:?} for {n} segments",
        started.elapsed()
    );
    assert!(o.closed.is_empty());
    assert_eq!(o.open.len(), 1);
    assert_eq!(o.open[0].len(), n + 1);
}

#[test]
fn a_file_without_units_and_with_tiny_holes_needs_the_units_and_with_them_it_is_read() {
    use eh3d_isocam::drill::read_drills_with_units;
    use eh3d_parser_excellon::Units;
    let text = "M48\nT1C0.03\n%\nT1\nX0.5Y0.5\nX2.5Y1.5\nM30\n";
    let err = read_drills(text).unwrap_err();
    assert!(err.contains("--units"), "{err}");
    let inches = read_drills_with_units(text, Some(Units::Inches)).unwrap();
    assert!((inches.holes[0].x_mm - 12.7).abs() < 1e-9, "{:?}", inches.holes);
    let mm = read_drills_with_units(text, Some(Units::Millimetres)).unwrap();
    assert!((mm.holes[0].x_mm - 0.5).abs() < 1e-9);
}
