//! The silkscreen layer engraved: strokes joined end to end, regions cut round their edge, the depth and the width of the tool of their own, and the warning that the layer is cut and not
//! printed. The programs are read back with the interpreter of the suite. Nothing here has run on a machine.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::silk;
use eh3d_isocam::simulate;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

/// Two lines of text 10 and 15 mm long drawn with a 0.2 mm pen, a thick line of 0.6 mm, and a pad stamped once.
const SILK: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.2*%\n%ADD11C,0.6*%\n%ADD12C,1.0*%\nD10*\nX0Y0D02*\nX100000Y0D01*\nX100000Y50000D01*\nX0Y100000D02*\nX150000Y100000D01*\nD11*\nX0Y200000D02*\nX200000Y200000D01*\nD12*\nX300000Y300000D03*\nM02*\n";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eh3d_isocam_silk_{tag}_{}", std::process::id()));
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

fn layer(d: &Path) -> String {
    let f = d.join("board.gto");
    std::fs::write(&f, SILK).unwrap();
    f.to_str().unwrap().to_string()
}

struct Seed(u64);

impl Seed {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn the_silkscreen_layer_is_written() {
    let d = dir("written");
    let (gto, out) = (layer(&d), d.join("silk.nc"));
    let (ok, stdout, err) = run(&[
        "silk",
        &gto,
        out.to_str().unwrap(),
        "--silk-depth-mm",
        "0.04",
    ]);
    assert!(ok, "{err}");
    assert!(
        stdout.contains("4 strokes and 0 regions engraved"),
        "{stdout}"
    );
    let text = std::fs::read_to_string(&out).unwrap();
    // the silkscreen is cut at its own depth, with its own tool, and the file reads back clean
    assert!(text.contains("G1 Z-0.04"), "{text}");
    assert!(!text.contains("G1 Z-0.06"), "{text}");
    let report = simulate::run(&text, 0.0);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!((report.deepest_mm - 0.04).abs() < 1e-9);
    // the layer is told to be engraved and not printed, and a stroke wider than the tool is told too
    assert!(err.contains("engraved, not printed"), "{err}");
    assert!(
        err.contains("1 of 4 strokes are drawn 0.2 to 0.6 mm wide"),
        "{err}"
    );
    assert!(err.contains("1 flashes"), "{err}");
    // the two strokes that touch make one run of three points, and the other two are runs of two points
    let moves = text.lines().filter(|l| l.starts_with("G1 X")).count();
    assert_eq!(moves, 2 + 1 + 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_silkscreen_can_be_a_drawing_but_not_a_plot() {
    let d = dir("kinds");
    let gto = layer(&d);
    let dxf = d.join("silk.dxf");
    let (ok, _, err) = run(&["silk", &gto, dxf.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(
        std::fs::read_to_string(&dxf).unwrap().contains("POLYLINE")
            || std::fs::read_to_string(&dxf)
                .unwrap()
                .contains("LWPOLYLINE")
    );
    let plt = d.join("silk.plt");
    let (ok, _, err) = run(&["silk", &gto, plt.to_str().unwrap()]);
    assert!(!ok && err.contains("not a plot"), "{err}");
    assert!(!plt.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_tool_and_the_depth_of_the_silkscreen_are_settings_with_a_range() {
    let s = Settings::default();
    assert_eq!(
        (s.silk_tool_mm, s.silk_depth_mm, s.silk_width_tolerance_mm),
        (0.2, 0.05, 0.1)
    );
    let text = s.to_text();
    for line in [
        "silk_tool_mm = 0.2\n",
        "silk_depth_mm = 0.05\n",
        "silk_width_tolerance_mm = 0.1\n",
    ] {
        assert!(text.contains(line), "{line:?}");
    }
    for (k, v) in [
        ("silk_tool_mm", "0.01"),
        ("silk_depth_mm", "3"),
        ("silk_width_tolerance_mm", "-1"),
    ] {
        let mut t = Settings::default();
        t.set(k, v).unwrap();
        let e = t.validate().unwrap_err();
        assert!(e.contains(k), "{e}");
    }
}

#[test]
fn a_machine_that_cannot_reach_the_silkscreen_refuses_it() {
    let d = dir("table");
    let (gto, out) = (layer(&d), d.join("silk.nc"));
    let (ok, _, err) = run(&[
        "silk",
        &gto,
        out.to_str().unwrap(),
        "--machine-x-mm",
        "10",
        "--machine-y-mm",
        "10",
    ]);
    assert!(!ok && err.contains("--force"), "{err}");
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn strokes_in_any_order_and_direction_join_into_the_same_runs() {
    let mut seed = Seed(0xC0FF_EE00_1234_5678);
    for _ in 0..25 {
        // a few separate polylines of random points, far apart so that they never touch
        let mut strokes = Vec::new();
        let (runs, mut length) = (2 + (seed.next() * 4.0) as usize, 0.0);
        for k in 0..runs {
            let n = 2 + (seed.next() * 6.0) as usize;
            let (mut x, mut y) = (k as f64 * 1000.0, 0.0);
            for _ in 0..n {
                let (nx, ny) = (x + 1.0 + seed.next() * 5.0, y + seed.next() * 5.0 - 2.5);
                length += (nx - x).hypot(ny - y);
                // a random direction of each stroke
                strokes.push(if seed.next() < 0.5 {
                    (x, y, nx, ny)
                } else {
                    (nx, ny, x, y)
                });
                (x, y) = (nx, ny);
            }
        }
        // a random order
        for i in (1..strokes.len()).rev() {
            strokes.swap(i, (seed.next() * (i + 1) as f64) as usize);
        }
        let scene = Scene {
            shapes: strokes
                .iter()
                .map(|&(x0, y0, x1, y1)| Shape::Stroke {
                    x0,
                    y0,
                    x1,
                    y1,
                    w: 0.2,
                })
                .collect(),
            notes: Vec::new(),
        };
        let e = silk::engrave(&scene, &Settings::default()).unwrap();
        let p = &e.passes[0];
        assert_eq!(p.open.len() + p.paths.len(), runs);
        let cut: f64 = p.cuts().iter().map(|c| c.length_mm()).sum();
        assert!((cut - length).abs() < 1e-6, "{cut} against {length}");
    }
}
