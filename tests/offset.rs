//! Where the work goes on the table: `offset_x_mm` and `offset_y_mm` move every output (the paths of the copper, the holes and the cut round the board) by the same amount, after the
//! scale and the reference points. Nothing here has run on a machine.

use eh3d_bungard_formats::hpgl as plotter;
use eh3d_isocam::config::Settings;
use eh3d_isocam::drill::read_drills;
use eh3d_isocam::toolpath::{trace, Kind};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A folder of its own for one test, empty.
fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eh3d_isocam_offset_{tag}_{}", std::process::id()));
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

/// The box of the cutting moves of a program: (min X, min Y, max X, max Y).
fn cut_box(program: &str) -> (f64, f64, f64, f64) {
    let t = trace(program);
    t.strokes
        .iter()
        .filter(|s| s.kind == Kind::Cut)
        .flat_map(|s| s.points.iter())
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, p| {
            (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
        })
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.005
}

/// Equal to the step of a drill file (a mil) or of a plot (a fortieth of a millimetre).
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.06
}

fn write(path: &Path, extra: &[&str], cmd: &[&str]) -> String {
    let mut args: Vec<&str> = cmd.to_vec();
    args.push(path.to_str().unwrap());
    args.extend_from_slice(extra);
    let (ok, out, err) = run(&args);
    assert!(ok, "{out}{err}");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_settings_of_the_offset_are_read_written_and_limited() {
    let s = Settings::default();
    assert_eq!((s.offset_x_mm, s.offset_y_mm), (0.0, 0.0));
    let text = s.to_text();
    for line in ["offset_x_mm = 0\n", "offset_y_mm = 0\n"] {
        assert!(text.contains(line), "{line:?} is not in\n{text}");
    }
    let example = std::fs::read_to_string(root().join("examples").join("isocam.cfg")).unwrap();
    assert!(example.contains("offset_x_mm = 0") && example.contains("offset_y_mm = 0"));
    let mut s = Settings::default();
    s.set("offset_x_mm", "120.5").unwrap();
    s.set("offset_y_mm", "-30").unwrap();
    s.validate().unwrap();
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    for (text, part) in [
        ("offset_x_mm = 20000\n", "offset_x_mm"),
        ("offset_y_mm = -20000\n", "offset_y_mm"),
        ("offset_x_mm = abc\n", "offset_x_mm"),
    ] {
        let e = Settings::parse(text)
            .and_then(|s| s.validate().map(|_| s))
            .unwrap_err();
        assert!(e.contains(part), "{text:?}: {e}");
    }
}

#[test]
fn the_paths_of_the_copper_move_by_the_offset_and_by_nothing_else() {
    let d = dir("copper");
    let base = write(&d.join("a.nc"), &[], &["isolate", "examples/sample.gtl"]);
    let moved = write(
        &d.join("b.nc"),
        &["--offset-x-mm", "100", "--offset-y-mm", "-2.5"],
        &["isolate", "examples/sample.gtl"],
    );
    let (a, b) = (cut_box(&base), cut_box(&moved));
    assert!(
        same(b.0 - a.0, 100.0) && same(b.2 - a.2, 100.0),
        "{a:?} {b:?}"
    );
    assert!(
        same(b.1 - a.1, -2.5) && same(b.3 - a.3, -2.5),
        "{a:?} {b:?}"
    );
    // the same number of moves: the offset changes where, not what
    assert_eq!(trace(&base).strokes.len(), trace(&moved).strokes.len());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_holes_move_by_the_offset() {
    let d = dir("holes");
    let input = d.join("in.drl");
    std::fs::write(
        &input,
        "M48\nMETRIC\nT1C0.8\n%\nT1\nX10.0Y20.0\nX30.5Y5.25\nM30\n",
    )
    .unwrap();
    let hole_box = |extra: &[&str]| {
        let out = d.join(format!("out{}.ncd", extra.len()));
        let mut args = vec!["drills", input.to_str().unwrap()];
        args.push(out.to_str().unwrap());
        args.extend_from_slice(extra);
        let (ok, o, e) = run(&args);
        assert!(ok, "{o}{e}");
        let read = read_drills(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let mut h: Vec<(f64, f64)> = read.holes.iter().map(|h| (h.x_mm, h.y_mm)).collect();
        h.sort_by(|a, b| a.partial_cmp(b).unwrap());
        h
    };
    let base = hole_box(&[]);
    let moved = hole_box(&["--offset-x-mm", "50", "--offset-y-mm", "7"]);
    assert_eq!(base.len(), 2);
    for (a, b) in base.iter().zip(&moved) {
        assert!(
            close(b.0 - a.0, 50.0) && close(b.1 - a.1, 7.0),
            "{a:?} {b:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_cut_round_the_board_moves_by_the_offset() {
    let d = dir("route");
    let outline = d.join("o.gko");
    std::fs::write(&outline, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX300000Y0D01*\nX300000Y200000D01*\nX0Y200000D01*\nX0Y0D01*\nM02*\n").unwrap();
    let bounds = |extra: &[&str], name: &str| {
        let plt = d.join(name);
        let mut args = vec!["route", outline.to_str().unwrap(), plt.to_str().unwrap()];
        args.extend_from_slice(extra);
        let (ok, o, e) = run(&args);
        assert!(ok, "{o}{e}");
        plotter::parse(&std::fs::read_to_string(&plt).unwrap())
            .bounds_mm()
            .unwrap()
    };
    let a = bounds(&[], "a.plt");
    let b = bounds(&["--offset-x-mm", "40", "--offset-y-mm", "15"], "b.plt");
    assert!(
        close(b.0 - a.0, 40.0) && close(b.2 - a.2, 40.0),
        "{a:?} {b:?}"
    );
    assert!(
        close(b.1 - a.1, 15.0) && close(b.3 - a.3, 15.0),
        "{a:?} {b:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_board_moved_off_the_table_is_refused_with_the_machine_limits() {
    let d = dir("table");
    let nc = d.join("c.nc");
    let table = ["--machine-profile", "ccd2-mtc"];
    // on the table at the origin, off it when moved 400 mm to the right
    let mut args = vec!["isolate", "examples/sample.gtl", nc.to_str().unwrap()];
    args.extend_from_slice(&table);
    let (ok, out, err) = run(&args);
    assert!(ok, "{out}{err}");
    args.extend_from_slice(&["--offset-x-mm", "400"]);
    let (ok, _, err) = run(&args);
    assert!(!ok && err.contains("outside the table"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}
