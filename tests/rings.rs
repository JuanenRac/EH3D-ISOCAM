//! Clearing an area with rings (`rubout-area` with `rubout_style = rings`): the rings follow the edge inwards in the order that is asked for, the overlap sets how far apart they are,
//! and a path that leaves the table of the machine is refused with its number before any G-code is written. Nothing here has run on a machine.

use eh3d_isocam::config::Settings;
use eh3d_isocam::simulate;
use eh3d_isocam::toolpath::{trace, Kind};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A folder of its own for one test, empty.
fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eh3d_isocam_rings_{tag}_{}", std::process::id()));
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

fn rubout(out: &Path, extra: &[&str]) -> (bool, String, String) {
    let mut args = vec![
        "rubout-area",
        out.to_str().unwrap(),
        "--area",
        "0,0,30,20",
        "--rubout-style",
        "rings",
        "--rubout-lines-tool-mm",
        "2",
        "--rubout-overlap",
        "0",
    ];
    args.extend_from_slice(extra);
    run(&args)
}

/// The box of the first cutting stroke of a program: (min X, min Y, max X, max Y).
fn first_cut(program: &str) -> (f64, f64, f64, f64) {
    let t = trace(program);
    let s = t
        .strokes
        .iter()
        .find(|s| s.kind == Kind::Cut)
        .expect("a cutting move");
    s.points
        .iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, p| {
            (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
        })
}

#[test]
fn the_settings_of_the_rings_are_read_written_and_checked() {
    let s = Settings::default();
    assert_eq!(s.rubout_style.name(), "lines");
    assert_eq!(s.rubout_ring_order.name(), "outside-in");
    let text = s.to_text();
    for line in ["rubout_style = lines\n", "rubout_ring_order = outside-in\n"] {
        assert!(text.contains(line), "{line:?} is not in\n{text}");
    }
    let example = std::fs::read_to_string(root().join("examples").join("isocam.cfg")).unwrap();
    for line in ["rubout_style = lines", "rubout_ring_order = outside-in"] {
        assert!(
            example.contains(line),
            "{line} is not in examples/isocam.cfg"
        );
    }
    let mut s = Settings::default();
    s.set("rubout_style", "rings").unwrap();
    s.set("rubout_ring_order", "inside_out").unwrap();
    s.validate().unwrap();
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    assert!(Settings::parse("rubout_style = spiral\n")
        .unwrap_err()
        .contains("lines or rings"));
    assert!(Settings::parse("rubout_ring_order = up\n")
        .unwrap_err()
        .contains("outside-in or inside-out"));
}

#[test]
fn an_area_is_cleared_with_rings_that_stay_inside_it() {
    let d = dir("area");
    let nc = d.join("rings.nc");
    let (ok, out, err) = rubout(&nc, &[]);
    assert!(ok, "{out}{err}");
    let program = std::fs::read_to_string(&nc).unwrap();
    // 30 x 20 mm with a tool of 2 mm and no overlap: rings at 1, 3, 5, 7 and 9 mm from the edge (the last one is the middle line)
    let t = trace(&program);
    let cuts = t.strokes.iter().filter(|s| s.kind == Kind::Cut).count();
    assert!((4..=7).contains(&cuts), "{cuts} cutting strokes");
    // the tool centre never leaves the area less half a tool
    let report = simulate::run(&program, 3000.0);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    for s in t.strokes.iter().filter(|s| s.kind == Kind::Cut) {
        for p in &s.points {
            assert!(p.0 > 0.9 && p.1 > 0.9 && p.0 < 29.1 && p.1 < 19.1, "{p:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_order_of_the_rings_is_kept_whatever_the_ordering_of_the_paths_says() {
    let d = dir("order");
    let outside = d.join("out.nc");
    let inside = d.join("in.nc");
    let (ok, out, err) = rubout(&outside, &["--rubout-ring-order", "outside-in"]);
    assert!(ok, "{out}{err}");
    let (ok, out, err) = rubout(&inside, &["--rubout-ring-order", "inside-out"]);
    assert!(ok, "{out}{err}");
    let a = first_cut(&std::fs::read_to_string(&outside).unwrap());
    let b = first_cut(&std::fs::read_to_string(&inside).unwrap());
    // the first ring cut is the one at the edge, or the one in the middle
    assert!(a.0 < 1.2 && a.2 > 28.8, "{a:?}");
    assert!(b.0 > 5.0, "{b:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_smaller_overlap_means_more_rings_and_a_plot_is_written_for_a_plt_name() {
    let d = dir("overlap");
    let count = |overlap: &str| {
        let nc = d.join(format!("o{overlap}.nc"));
        let (ok, out, err) = rubout(&nc, &["--rubout-overlap", overlap]);
        assert!(ok, "{out}{err}");
        trace(&std::fs::read_to_string(&nc).unwrap())
            .strokes
            .iter()
            .filter(|s| s.kind == Kind::Cut)
            .count()
    };
    let (none, half) = (count("0"), count("0.5"));
    assert!(half >= 2 * none - 2, "{half} rings against {none}");
    let plt = d.join("rings.plt");
    let (ok, out, err) = rubout(&plt, &[]);
    assert!(ok, "{out}{err}");
    assert!(std::fs::read_to_string(&plt).unwrap().contains("PD"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_path_that_leaves_the_table_is_refused_by_number_unless_forced() {
    let d = dir("table");
    let nc = d.join("big.nc");
    // the table is 20 x 20 mm: the rings of an area 30 mm wide leave it
    let table = ["--machine-x-mm", "20", "--machine-y-mm", "20"];
    let (ok, out, err) = rubout(&nc, &table);
    assert!(!ok, "{out}");
    assert!(
        err.contains("path ")
            && err.contains("(pass 1)")
            && err.contains("outside the table (20 x 20 mm"),
        "{err}"
    );
    assert!(err.contains("--force writes it anyway"), "{err}");
    assert!(!nc.exists(), "nothing is written");
    // the same for lines
    let lines: Vec<&str> = table
        .into_iter()
        .chain(["--rubout-style", "lines"])
        .collect();
    let (ok, _, err) = rubout(&nc, &lines);
    assert!(!ok && err.contains("outside the table"), "{err}");
    // forced, it is written and the notice comes back
    let forced: Vec<&str> = table.into_iter().chain(["--force"]).collect();
    let (ok, out, err) = rubout(&nc, &forced);
    assert!(ok, "{out}{err}");
    assert!(err.contains("warning: path "), "{err}");
    assert!(nc.exists());
    // an area that fits is written with the same table
    let small = d.join("small.nc");
    let (ok, out, err) = run(&[
        "rubout-area",
        small.to_str().unwrap(),
        "--area",
        "2,2,12,12",
        "--rubout-style",
        "rings",
        "--machine-x-mm",
        "20",
        "--machine-y-mm",
        "20",
    ]);
    assert!(ok, "{out}{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn rings_do_not_cut_to_a_layer_and_a_contour_file_is_filled() {
    let d = dir("contour");
    let nc = d.join("c.nc");
    let (ok, _, err) = run(&[
        "rubout-area",
        nc.to_str().unwrap(),
        "examples/sample.gtl",
        "--area",
        "0,0,30,20",
        "--rubout-style",
        "rings",
    ]);
    assert!(!ok && err.contains("does not cut them to a layer"), "{err}");
    // no area and no contour
    let (ok, _, err) = run(&[
        "rubout-area",
        nc.to_str().unwrap(),
        "--rubout-style",
        "rings",
    ]);
    assert!(!ok && err.contains("--area"), "{err}");
    // an area narrower than the tool
    let (ok, _, err) = run(&[
        "rubout-area",
        nc.to_str().unwrap(),
        "--area",
        "0,0,30,1",
        "--rubout-style",
        "rings",
        "--rubout-lines-tool-mm",
        "2",
    ]);
    assert!(!ok && err.contains("narrower"), "{err}");
    // a contour from a Gerber outline: the sample outline of the window tests
    let outline = d.join("o.gko");
    std::fs::write(&outline, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX300000Y0D01*\nX300000Y200000D01*\nX0Y200000D01*\nX0Y0D01*\nM02*\n").unwrap();
    let (ok, out, err) = run(&[
        "rubout-area",
        nc.to_str().unwrap(),
        "--contour",
        outline.to_str().unwrap(),
        "--rubout-style",
        "rings",
        "--rubout-lines-tool-mm",
        "2",
    ]);
    assert!(ok, "{out}{err}");
    assert!(first_cut(&std::fs::read_to_string(&nc).unwrap()).0 < 1.3);
    let _ = std::fs::remove_dir_all(&d);
}
