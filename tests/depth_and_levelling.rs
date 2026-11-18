//! Cutting in steps of depth and cutting along a height map of the table: the G-code is read back with the interpreter of the suite and every Z is compared with the surface it
//! should follow. Nothing here has run on a machine.

use eh3d_isocam::config::Settings;
use eh3d_isocam::gcode::{write_in, write_levelled, Job, Surface};
use eh3d_isocam::heightmap::HeightMap;
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::Order;
use eh3d_isocam::units::Unit;
use eh3d_parser_gcode::{interpret, parse, Dialect, Motion};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eh3d_isocam_level_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(root())
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn isolate(out: &Path, extra: &[&str]) -> (bool, String, String) {
    let mut args = vec!["isolate", "examples/sample.gtl", out.to_str().unwrap()];
    args.extend_from_slice(extra);
    run(&args)
}

fn square() -> Pass {
    Pass {
        paths: vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)]],
        diameter_mm: 0.2,
        ..Pass::default()
    }
}

fn run_of(points: Vec<(f64, f64)>) -> Pass {
    Pass {
        open: vec![points],
        diameter_mm: 0.2,
        ..Pass::default()
    }
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

fn count(text: &str, line: &str) -> usize {
    text.lines().filter(|l| *l == line).count()
}

fn write(p: &Pass, job: &Job) -> String {
    write_in(std::slice::from_ref(p), job, &Order::default(), Unit::Mm).unwrap()
}

// ---- steps of depth ---------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_levels_go_down_by_the_step_and_end_at_the_whole_depth() {
    let job = |depth: f64, step: f64| Job {
        depth_mm: depth,
        depth_step_mm: step,
        ..Job::default()
    };
    assert_eq!(job(0.06, 0.0).levels(), vec![0.06]);
    assert_eq!(job(0.06, 0.03).levels(), vec![0.03, 0.06]);
    assert_eq!(job(0.06, 0.025).levels(), vec![0.025, 0.05, 0.06]);
    assert_eq!(job(0.06, 0.06).levels(), vec![0.06]);
    assert_eq!(job(0.06, 0.5).levels(), vec![0.06]);
    assert_eq!(job(0.3, 0.1).levels().len(), 3);
    assert_eq!(*job(0.3, 0.1).levels().last().unwrap(), 0.3);
    // a step that is below 0.005 mm or above 2 mm is refused
    for bad in [-0.1, 0.001, 2.5, f64::NAN] {
        let e = job(0.06, bad).validate().unwrap_err();
        assert!(e.contains("depth_step_mm"), "{e}");
    }
}

#[test]
fn with_no_step_the_program_is_the_one_of_a_single_plunge() {
    let g = write(&square(), &Job::default());
    let want = "G21\nG90\nG0 Z2\nM3 S24000\n(pass 1 at 0 mm from the copper, 1 paths)\nG0 X0 Y0\nG1 Z-0.06 F60\nG1 X5 Y0 F300\nG1 X5 Y5\nG1 X0 Y5\nG1 X0 Y0\nG0 Z2\nM5\nM2\n";
    assert_eq!(g, want);
}

#[test]
fn a_closed_path_goes_round_again_at_each_depth_without_lifting_the_tool() {
    let job = Job {
        depth_step_mm: 0.03,
        ..Job::default()
    };
    let g = write(&square(), &job);
    assert_eq!(count(&g, "G0 X0 Y0"), 1, "{g}");
    assert_eq!(count(&g, "G1 Z-0.03 F60"), 1, "{g}");
    assert_eq!(count(&g, "G1 Z-0.06 F60"), 1, "{g}");
    assert_eq!(count(&g, "G1 X5 Y0 F300"), 2, "{g}");
    assert_eq!(count(&g, "G0 Z2"), 2, "{g}"); // the start of the program and after the path
    let l = lines(&g);
    let at = l.iter().position(|x| *x == "G1 Z-0.06 F60").unwrap();
    // it plunges deeper where the first round ended: the start of the path
    assert_eq!(l[at - 1], "G1 X0 Y0", "{g}");
    // read back: every cutting move is under the surface, none rapid, and it ends
    let trace = interpret(&parse(&g, Dialect::Grbl));
    assert!(trace.ended);
    let deepest = trace
        .segments
        .iter()
        .filter(|s| !matches!(s.motion, Motion::Rapid))
        .map(|s| s.to.z)
        .fold(0.0, f64::min);
    assert!((deepest + 0.06).abs() < 1e-9, "{deepest}");
}

#[test]
fn an_open_run_is_cut_back_the_way_it_came_at_the_next_depth() {
    let job = Job {
        depth_step_mm: 0.02,
        ..Job::default()
    };
    let g = write(&run_of(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0)]), &job);
    let l = lines(&g);
    let first = l.iter().position(|x| *x == "G1 Z-0.02 F60").unwrap();
    assert_eq!(
        &l[first + 1..first + 4],
        ["G1 X10 Y0 F300", "G1 X10 Y4", "G1 Z-0.04 F60"]
    );
    // the second depth starts where the first ended and goes back to the start, the third forward again
    assert_eq!(
        &l[first + 4..first + 7],
        ["G1 X10 Y0 F300", "G1 X0 Y0", "G1 Z-0.06 F60"]
    );
    assert_eq!(
        &l[first + 7..first + 10],
        ["G1 X10 Y0 F300", "G1 X10 Y4", "G0 Z2"]
    );
    assert_eq!(count(&g, "G0 X0 Y0"), 1, "{g}");
}

#[test]
fn the_steps_of_depth_reach_the_command_line() {
    let d = dir("step");
    let nc = d.join("a.nc");
    let (ok, _, err) = isolate(&nc, &["--depth-step-mm", "0.03"]);
    assert!(ok, "{err}");
    let g = std::fs::read_to_string(&nc).unwrap();
    assert!(
        g.contains("G1 Z-0.03 F60") && g.contains("G1 Z-0.06 F60"),
        "{g}"
    );
    let (ok, _, err) = isolate(&nc, &["--depth-step-mm", "0.001"]);
    assert!(!ok && err.contains("depth_step_mm is 0.001"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- a surface --------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_cut_follows_the_surface_in_pieces_no_longer_than_asked() {
    let slope = |x: f64, _y: f64| 0.001 * x;
    let surface = Surface {
        height_mm: &slope,
        max_segment_mm: 2.0,
    };
    let g = write_levelled(
        &[run_of(vec![(0.0, 0.0), (10.0, 0.0)])],
        &Job::default(),
        &Order::default(),
        Unit::Mm,
        Some(&surface),
    )
    .unwrap();
    let trace = interpret(&parse(&g, Dialect::Grbl));
    let cuts: Vec<_> = trace
        .segments
        .iter()
        .filter(|s| matches!(s.motion, Motion::Linear) && (s.from.x != s.to.x))
        .collect();
    assert_eq!(cuts.len(), 5, "{g}");
    for s in &cuts {
        assert!(((s.to.x - s.from.x) - 2.0).abs() < 1e-9, "{s:?}");
        assert!((s.to.z - (-0.06 + 0.001 * s.to.x)).abs() < 1e-4, "{s:?}");
    }
    // the plunge goes to the depth under the surface where the cut starts
    assert!(g.contains("G1 Z-0.06 F60\n"), "{g}");
    // the first cutting move carries the feed, the others none
    assert_eq!(count(&g, "G1 X2 Y0 Z-0.058 F300"), 1, "{g}");
    assert!(g.contains("G1 X4 Y0 Z-0.056\n"), "{g}");
}

#[test]
fn a_surface_and_steps_of_depth_work_together() {
    let bump = |x: f64, y: f64| 0.02 * (x + y) / 10.0;
    let surface = Surface {
        height_mm: &bump,
        max_segment_mm: 1.0,
    };
    let job = Job {
        depth_step_mm: 0.03,
        ..Job::default()
    };
    let g = write_levelled(
        &[square()],
        &job,
        &Order::default(),
        Unit::Mm,
        Some(&surface),
    )
    .unwrap();
    let trace = interpret(&parse(&g, Dialect::Grbl));
    assert!(trace.ended);
    let mut deeper = false;
    for s in &trace.segments {
        if matches!(s.motion, Motion::Linear) && (s.from.x != s.to.x || s.from.y != s.to.y) {
            let h = bump(s.to.x, s.to.y);
            let under = h - s.to.z;
            assert!(
                (under - 0.03).abs() < 1e-3 || (under - 0.06).abs() < 1e-3,
                "{s:?} is {under} under the surface"
            );
            deeper |= (under - 0.06).abs() < 1e-3;
        }
    }
    assert!(deeper, "the second depth was not cut");
}

#[test]
fn a_surface_with_no_piece_length_is_refused() {
    let flat = |_x: f64, _y: f64| 0.0;
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let surface = Surface {
            height_mm: &flat,
            max_segment_mm: bad,
        };
        let e = write_levelled(
            &[square()],
            &Job::default(),
            &Order::default(),
            Unit::Mm,
            Some(&surface),
        )
        .unwrap_err();
        assert!(e.contains("longest piece"), "{e}");
    }
}

// ---- the height map on the command line --------------------------------------------------------------------------------------------------------------------

/// A table that rises 0.002 mm for every millimetre in X, over the area of the sample board: 5 x 2 points over 30 x 20 mm.
const MAP: &str = "levelling 1\ngrid 0 0 30 20 5 2\n0.0000 0.0150 0.0300 0.0450 0.0600\n0.0000 0.0150 0.0300 0.0450 0.0600\n";

#[test]
fn the_program_of_the_command_line_follows_the_map() {
    let d = dir("map");
    let nc = d.join("a.nc");
    let map = d.join("table.map");
    std::fs::write(&map, MAP).unwrap();
    let (ok, _, err) = isolate(&nc, &["--height-map", map.to_str().unwrap()]);
    assert!(ok, "{err}");
    let g = std::fs::read_to_string(&nc).unwrap();
    assert!(g.contains("(Z follows the height map table.map)"), "{g}");
    let map = HeightMap::parse(MAP).unwrap();
    let trace = interpret(&parse(&g, Dialect::Grbl));
    assert!(trace.ended);
    let mut checked = 0;
    for s in &trace.segments {
        if matches!(s.motion, Motion::Linear) && (s.from.x != s.to.x || s.from.y != s.to.y) {
            let want = -0.06 + map.height_at(s.to.x, s.to.y) - map.height_at(0.0, 0.0);
            assert!((s.to.z - want).abs() < 2e-3, "{s:?} wants {want}");
            checked += 1;
        }
    }
    assert!(checked > 20, "{checked} cuts");
    // no segment longer than asked (2 mm by default)
    for s in trace
        .segments
        .iter()
        .filter(|s| matches!(s.motion, Motion::Linear))
    {
        let flat = (s.to.x - s.from.x).hypot(s.to.y - s.from.y);
        assert!(flat <= 2.0 + 1e-6, "{s:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_zero_of_the_map_can_be_moved() {
    let d = dir("zero");
    let (nc_a, nc_b) = (d.join("a.nc"), d.join("b.nc"));
    let map = d.join("table.map");
    std::fs::write(&map, MAP).unwrap();
    let m = map.to_str().unwrap();
    let (ok, _, err) = isolate(&nc_a, &["--height-map", m]);
    assert!(ok, "{err}");
    let (ok, _, err) = isolate(
        &nc_b,
        &[
            "--height-map",
            m,
            "--heightmap-ref-x-mm",
            "30",
            "--heightmap-ref-y-mm",
            "0",
        ],
    );
    assert!(ok, "{err}");
    let a = std::fs::read_to_string(&nc_a).unwrap();
    let b = std::fs::read_to_string(&nc_b).unwrap();
    // with the zero at the high end the whole cut sits 0.06 mm lower
    let first = |t: &str| {
        t.lines()
            .find(|l| l.starts_with("G1 Z"))
            .unwrap()
            .trim_start_matches("G1 Z")
            .split(' ')
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    assert!(
        (first(&a) - first(&b) - 0.06).abs() < 1e-3,
        "{} {}",
        first(&a),
        first(&b)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_map_that_is_bad_or_too_far_from_flat_is_refused_and_nothing_is_written() {
    let d = dir("badmap");
    let nc = d.join("a.nc");
    let map = d.join("table.map");
    // 1 mm of difference is beyond the 0.5 mm the settings allow
    std::fs::write(&map, "levelling 1\ngrid 0 0 30 20 2 2\n0 1\n0 1\n").unwrap();
    let (ok, _, err) = isolate(&nc, &["--height-map", map.to_str().unwrap()]);
    assert!(!ok && err.contains("heightmap_max_range_mm"), "{err}");
    assert!(!nc.exists());
    let (ok, _, err) = isolate(
        &nc,
        &[
            "--height-map",
            map.to_str().unwrap(),
            "--heightmap-max-range-mm",
            "2",
        ],
    );
    assert!(ok, "{err}");
    std::fs::write(&map, "levelling 1\ngrid 0 0 30 20 2 2\n0 -\n0 1\n").unwrap();
    let (ok, _, err) = isolate(&nc, &["--height-map", map.to_str().unwrap()]);
    assert!(!ok && err.contains("not measured"), "{err}");
    let (ok, _, err) = isolate(&nc, &["--height-map", d.join("none.map").to_str().unwrap()]);
    assert!(!ok && err.contains("none.map"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_map_is_read_with_the_size_limit_of_every_other_file() {
    let d = dir("limit");
    let nc = d.join("a.nc");
    let map = d.join("table.map");
    // a file of more than one mebibyte, the least the limit can be set to, is not read
    let mut big = String::from(
        "levelling 1
",
    );
    big.push_str(&" ".repeat(1_200_000));
    std::fs::write(&map, big).unwrap();
    let (ok, _, err) = isolate(
        &nc,
        &["--height-map", map.to_str().unwrap(), "--max-input-mb", "1"],
    );
    assert!(!ok && err.contains("above the limit"), "{err}");
    assert!(!nc.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_settings_of_the_surface_have_defaults_that_change_nothing() {
    let s = Settings::default();
    assert_eq!(s.depth_step_mm, 0.0);
    assert_eq!(s.heightmap_file, "");
    assert_eq!((s.heightmap_ref_x_mm, s.heightmap_ref_y_mm), (0.0, 0.0));
    assert_eq!(
        (s.heightmap_max_segment_mm, s.heightmap_max_range_mm),
        (2.0, 0.5)
    );
    let text = s.to_text();
    for line in [
        "depth_step_mm = 0\n",
        "heightmap_file = \n",
        "heightmap_ref_x_mm = 0\n",
        "heightmap_max_segment_mm = 2\n",
        "heightmap_max_range_mm = 0.5\n",
    ] {
        assert!(text.contains(line), "{line:?} is not in\n{text}");
    }
    let t = Settings {
        heightmap_file: "C:\\tables\\bench one.map".into(),
        depth_step_mm: 0.02,
        heightmap_ref_x_mm: 12.5,
        ..Settings::default()
    };
    assert_eq!(Settings::parse(&t.to_text()).unwrap(), t);
    for (text, part) in [
        ("depth_step_mm = 0.001\n", "depth_step_mm is 0.001"),
        (
            "heightmap_max_segment_mm = 0.01\n",
            "heightmap_max_segment_mm is 0.01",
        ),
        (
            "heightmap_max_segment_mm = 80\n",
            "heightmap_max_segment_mm is 80",
        ),
        (
            "heightmap_max_range_mm = 0\n",
            "heightmap_max_range_mm is 0",
        ),
        (
            "heightmap_max_range_mm = 9\n",
            "heightmap_max_range_mm is 9",
        ),
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?} gave {e:?}");
    }
    let mut v = Settings::default();
    assert!(v
        .set("heightmap_file", "a#b")
        .unwrap_err()
        .contains("cannot hold"));
    v.set("--height-map", "table.map").unwrap();
    assert_eq!(v.heightmap_file, "table.map");
}

#[test]
fn the_example_settings_file_has_the_new_keys() {
    let text = std::fs::read_to_string(root().join("examples").join("isocam.cfg")).unwrap();
    for key in [
        "depth_step_mm",
        "heightmap_file",
        "heightmap_ref_x_mm",
        "heightmap_ref_y_mm",
        "heightmap_max_segment_mm",
        "heightmap_max_range_mm",
    ] {
        assert!(text.contains(key), "{key} is not in examples/isocam.cfg");
    }
}
