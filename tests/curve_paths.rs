//! Arcs fitted to the paths: where a run of the short moves lies on a circle one `G2` or `G3` replaces it, within a tolerance of the settings, and the program is walked with the
//! interpreter of the suite and compared with the program of straight pieces. Nothing here has run on a machine.

use eh3d_isocam::arcs::{self, Pt, Seg};
use eh3d_isocam::board;
use eh3d_isocam::config::Settings;
use eh3d_isocam::gcode::{write_fitted, write_in, Job, Surface};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::Order;
use eh3d_isocam::simulate;
use eh3d_isocam::units::Unit;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A generator of numbers with a fixed seed, so that every run of the test draws the same circles.
struct Seed(u64);

impl Seed {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

fn circle(centre: Pt, r: f64, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            (centre.0 + r * a.cos(), centre.1 + r * a.sin())
        })
        .collect()
}

fn pass_of_loop(points: &[Pt]) -> Pass {
    // a closed path does not repeat its first point
    Pass {
        paths: vec![points[..points.len() - 1].to_vec()],
        diameter_mm: 0.2,
        ..Pass::default()
    }
}

fn straight(p: &Pass) -> String {
    write_in(
        std::slice::from_ref(p),
        &Job::default(),
        &Order::default(),
        Unit::Mm,
    )
    .unwrap()
}

fn fitted(p: &Pass, tol: f64) -> String {
    write_fitted(
        std::slice::from_ref(p),
        &Job::default(),
        &Order::default(),
        Unit::Mm,
        None,
        tol,
    )
    .unwrap()
}

fn arcs_in(text: &str) -> usize {
    text.lines()
        .filter(|l| l.starts_with("G2 ") || l.starts_with("G3 "))
        .count()
}

fn cut_lines(text: &str) -> usize {
    text.lines()
        .filter(|l| l.starts_with("G1 X") || l.starts_with("G2 ") || l.starts_with("G3 "))
        .count()
}

#[test]
fn the_path_follows_the_curve_within_the_tolerance() {
    let mut seed = Seed(0x9E37_79B9_7F4A_7C15);
    for _ in 0..40 {
        let r = seed.between(2.0, 20.0);
        let n = 24 + (seed.next() * 100.0) as usize;
        let centre = (seed.between(-30.0, 30.0), seed.between(-30.0, 30.0));
        let pts = circle(centre, r, n);
        for tol in [0.005, 0.02, 0.1] {
            let segs = arcs::fit(&pts, tol);
            let dev = arcs::deviation_mm(&pts, &segs);
            assert!(dev <= tol + 1e-9, "r {r} n {n} tol {tol}: {dev}");
            assert_eq!(segs.last().unwrap().end(), *pts.last().unwrap());
            // when the pieces bulge from their circle by less than the tolerance, arcs must have been found
            let sagitta = r * (1.0 - (std::f64::consts::PI / n as f64).cos());
            if sagitta < tol * 0.8 {
                assert!(
                    segs.len() < pts.len() - 1,
                    "r {r} n {n} tol {tol}: no arc at all"
                );
            }
        }
    }
}

#[test]
fn points_with_noise_inside_the_tolerance_still_fit_within_it() {
    let mut seed = Seed(0x1234_5678_9ABC_DEF1);
    let tol = 0.02;
    for _ in 0..30 {
        let r = seed.between(3.0, 15.0);
        let mut pts = circle((0.0, 0.0), r, 90);
        for p in pts.iter_mut() {
            p.0 += seed.between(-tol / 8.0, tol / 8.0);
            p.1 += seed.between(-tol / 8.0, tol / 8.0);
        }
        let segs = arcs::fit(&pts, tol);
        assert!(arcs::deviation_mm(&pts, &segs) <= tol + 1e-9);
    }
}

#[test]
fn a_fine_circle_is_written_with_far_fewer_moves_than_the_polygon() {
    let p = pass_of_loop(&circle((10.0, 10.0), 8.0, 128));
    let (poly, arcs_text) = (straight(&p), fitted(&p, 0.01));
    assert_eq!(arcs_in(&poly), 0);
    assert!(arcs_in(&arcs_text) >= 1);
    assert!(
        cut_lines(&arcs_text) * 4 < cut_lines(&poly),
        "{} against {}",
        cut_lines(&arcs_text),
        cut_lines(&poly)
    );
    // the settings default keeps the straight pieces: the same text as before
    assert_eq!(fitted(&p, 0.0), poly);
}

#[test]
fn the_program_with_arcs_reads_back_with_no_finding_and_the_same_cut() {
    let mut seed = Seed(0xDEAD_BEEF_0000_0001);
    for _ in 0..12 {
        let p = pass_of_loop(&circle(
            (seed.between(5.0, 40.0), seed.between(5.0, 40.0)),
            seed.between(2.0, 12.0),
            72,
        ));
        let (poly, arcs_text) = (straight(&p), fitted(&p, 0.01));
        let (a, b) = (simulate::run(&arcs_text, 0.0), simulate::run(&poly, 0.0));
        assert!(a.warnings.is_empty(), "{:?}", a.warnings);
        // the arc is longer than its chord by a sagitta at most: the two cuts differ by little
        assert!(
            (a.cut_mm - b.cut_mm).abs() / b.cut_mm < 0.01,
            "{} against {}",
            a.cut_mm,
            b.cut_mm
        );
        // the box of the machine is the same to the tolerance
        for (x, y) in [
            (a.min.0, b.min.0),
            (a.max.0, b.max.0),
            (a.min.1, b.min.1),
            (a.max.1, b.max.1),
        ] {
            assert!((x - y).abs() <= 0.02, "{x} against {y}");
        }
    }
}

#[test]
fn an_arc_written_in_inches_is_read_back_in_inches() {
    let p = pass_of_loop(&circle((0.0, 0.0), 10.0, 96));
    let text = write_fitted(
        std::slice::from_ref(&p),
        &Job::default(),
        &Order::default(),
        Unit::Inch,
        None,
        0.01,
    )
    .unwrap();
    assert!(text.starts_with("G20"));
    assert!(arcs_in(&text) >= 1);
    let r = simulate::run(&text, 0.0);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // the circle is 10 mm in radius: its box is 20 mm wide in mm once the interpreter has converted it
    assert!(
        (r.max.0 - r.min.0 - 20.0).abs() < 0.05,
        "{:?} {:?}",
        r.min,
        r.max
    );
}

#[test]
fn an_open_run_cut_back_at_the_next_depth_turns_the_other_way() {
    let half: Vec<Pt> = circle((0.0, 0.0), 6.0, 96)[..49].to_vec();
    let p = Pass {
        open: vec![half],
        diameter_mm: 0.2,
        ..Pass::default()
    };
    let job = Job {
        depth_step_mm: 0.03,
        depth_mm: 0.06,
        ..Job::default()
    };
    let text = write_fitted(
        std::slice::from_ref(&p),
        &job,
        &Order::default(),
        Unit::Mm,
        None,
        0.01,
    )
    .unwrap();
    let ccw = text.lines().filter(|l| l.starts_with("G3 ")).count();
    let cw = text.lines().filter(|l| l.starts_with("G2 ")).count();
    assert!(ccw >= 1 && cw >= 1, "{text}");
    assert_eq!(ccw, cw);
    let r = simulate::run(&text, 0.0);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}

#[test]
fn a_tolerance_out_of_range_or_with_a_height_map_is_refused() {
    for v in ["0.0005", "0.6", "-0.01"] {
        let mut s = Settings::default();
        s.set("arc_fit_tolerance_mm", v).unwrap();
        let e = s.validate().unwrap_err();
        assert!(e.contains("arc_fit_tolerance_mm"), "{e}");
    }
    let mut s = Settings::default();
    s.set("arc_fit_tolerance_mm", "0.01").unwrap();
    assert!(s.validate().is_ok());
    assert!(s.to_text().contains("arc_fit_tolerance_mm = 0.01\n"));
    assert!(Settings::default()
        .to_text()
        .contains("arc_fit_tolerance_mm = 0\n"));
    let p = pass_of_loop(&circle((0.0, 0.0), 5.0, 64));
    let flat = |_: f64, _: f64| 0.0;
    let surface = Surface {
        height_mm: &flat,
        max_segment_mm: 1.0,
    };
    let e = write_fitted(
        std::slice::from_ref(&p),
        &Job::default(),
        &Order::default(),
        Unit::Mm,
        Some(&surface),
        0.01,
    )
    .unwrap_err();
    assert!(e.contains("height map"), "{e}");
    let e = write_fitted(
        std::slice::from_ref(&p),
        &Job::default(),
        &Order::default(),
        Unit::Mm,
        None,
        f64::NAN,
    )
    .unwrap_err();
    assert!(e.contains("arc_fit_tolerance_mm"), "{e}");
}

#[test]
fn the_comparison_with_the_polygon_counts_the_moves_and_the_arcs() {
    let sample = root().join("examples/sample.gtl");
    let mut s = Settings::default();
    let layer = eh3d_isocam::window::read_layer(&sample, &s).unwrap();
    let passes =
        eh3d_isocam::isolate::plan_with_pads(&layer.scene, &s.plan().unwrap(), s.around_pads())
            .unwrap();
    // off by default: nothing to compare
    assert!(board::arc_comparison(&passes, &s).unwrap().is_none());
    s.set("arc_fit_tolerance_mm", "0.01").unwrap();
    let c = board::arc_comparison(&passes, &s).unwrap().unwrap();
    assert!(c.arcs >= 1, "{c:?}");
    assert!(c.fitted_moves < c.polygon_moves, "{c:?}");
    assert!(
        (c.fitted_cut_mm - c.polygon_cut_mm).abs() / c.polygon_cut_mm < 0.01,
        "{c:?}"
    );
}

#[test]
fn the_command_line_writes_arcs_and_says_what_it_saved() {
    let dir = std::env::temp_dir().join(format!("eh3d_isocam_curve_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (flat, round) = (dir.join("flat.nc"), dir.join("round.nc"));
    let run = |out: &PathBuf, extra: &[&str]| {
        let mut args = vec!["isolate", "examples/sample.gtl", out.to_str().unwrap()];
        args.extend_from_slice(extra);
        Command::new(BIN)
            .args(&args)
            .current_dir(root())
            .env("LC_ALL", "en_GB.UTF-8")
            .output()
            .unwrap()
    };
    let a = run(&flat, &[]);
    assert!(a.status.success());
    let b = run(&round, &["--arc-fit-tolerance-mm", "0.01"]);
    assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
    let (t_flat, t_round) = (
        std::fs::read_to_string(&flat).unwrap(),
        std::fs::read_to_string(&round).unwrap(),
    );
    assert_eq!(arcs_in(&t_flat), 0);
    assert!(arcs_in(&t_round) >= 1);
    assert!(cut_lines(&t_round) < cut_lines(&t_flat));
    let err = String::from_utf8_lossy(&b.stderr);
    assert!(
        err.contains("moves instead of") && err.contains("arcs fitted within 0.01 mm"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_limits_of_the_machine_see_where_an_arc_bulges() {
    // four points of a circle round (10, 10) of radius 8, at 70, 80, 100 and 110 degrees: the top of the circle (Y 18) is between two of them
    let on_circle: Vec<Pt> = [70.0_f64, 80.0, 100.0, 110.0]
        .iter()
        .map(|d| {
            (
                10.0 + 8.0 * d.to_radians().cos(),
                10.0 + 8.0 * d.to_radians().sin(),
            )
        })
        .collect();
    let p = Pass {
        open: vec![on_circle.clone()],
        diameter_mm: 0.2,
        ..Pass::default()
    };
    assert!(matches!(arcs::fit(&on_circle, 0.5)[0], Seg::Arc { .. }));
    let limits = eh3d_isocam::machine::Limits {
        table_mm: Some((20.0, 17.95)),
        ..eh3d_isocam::machine::Limits::default()
    };
    // the points are inside the table and the straight pieces stay inside; the arc goes out through the top
    assert!(eh3d_isocam::machine::check_program(&straight(&p), &limits).is_ok());
    assert!(eh3d_isocam::machine::check_program(&fitted(&p, 0.5), &limits).is_err());
}

#[test]
fn an_arc_inside_the_table_passes_whichever_way_it_turns_and_one_that_leaves_it_is_caught() {
    use eh3d_isocam::machine::{check_program, Limits};
    let table = |x: f64, y: f64| Limits {
        table_mm: Some((x, y)),
        ..Limits::default()
    };
    // a half circle of radius 5 about (10, 10), counter-clockwise from (15, 10) over the top to (5, 10): it reaches Y 15, the lower half does not
    let ccw = "G21\nG90\nM3 S1000\nG0 X15 Y10\nG1 Z-0.05 F60\nG3 X5 Y10 I-5 J0 F300\nM5\nM2\n";
    let cw = "G21\nG90\nM3 S1000\nG0 X15 Y10\nG1 Z-0.05 F60\nG2 X5 Y10 I-5 J0 F300\nM5\nM2\n";
    assert!(check_program(ccw, &table(20.0, 15.5)).is_ok());
    assert!(check_program(ccw, &table(20.0, 14.5)).is_err());
    // the other way round the arc goes under, to Y 5
    assert!(check_program(cw, &table(20.0, 10.5)).is_ok());
    let lower = Limits {
        table_mm: Some((20.0, 20.0)),
        margin_mm: 5.5,
        ..Limits::default()
    };
    assert!(check_program(cw, &lower).is_err());
    // a start that is the end is a whole circle
    let circle = "G21\nG90\nM3 S1000\nG0 X15 Y10\nG1 Z-0.05 F60\nG2 X15 Y10 I-5 J0 F300\nM5\nM2\n";
    assert!(check_program(circle, &table(20.0, 14.0)).is_err());
    assert!(check_program(circle, &table(20.0, 15.5)).is_ok());
}
