//! The report before a job and the tools set against the crib table: the passes, the tools, the arcs, the dry run and the checks of the layer, told in the eight languages, and the
//! exit code 2 of a verification that is not passed. Nothing here has run on a machine.

use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_isocam::config::Settings;
use eh3d_isocam::crib;
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::lang::Lang;
use eh3d_isocam::report::{checks, job_report};
use eh3d_isocam::transform::Board;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(root())
        .env("LC_ALL", "en_GB.UTF-8")
        .output()
        .expect("the program runs");
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn table() -> Vec<crib::CribTool> {
    let text = std::fs::read_to_string(root().join("examples/crib.csv")).unwrap();
    crib::parse_csv(&text).unwrap()
}

fn passes_of(s: &Settings) -> Vec<Pass> {
    let layer = eh3d_isocam::window::read_layer(&root().join("examples/sample.gtl"), s).unwrap();
    eh3d_isocam::isolate::plan_with_pads(&layer.scene, &s.plan().unwrap(), s.around_pads()).unwrap()
}

fn mm(v: f64) -> String {
    format!("{v:.3} mm")
}

#[test]
fn the_report_lists_the_passes_the_tools_and_what_is_not_checked() {
    let s = Settings::default();
    let passes = passes_of(&s);
    let r = job_report(&passes, &s, 0, None).unwrap();
    assert_eq!(r.passes, passes.len());
    assert_eq!(
        r.paths,
        passes.iter().map(|p| p.cuts().len()).sum::<usize>()
    );
    assert_eq!(r.tools.len(), 1);
    assert!((r.tools[0].width_mm - s.fine_tool_mm).abs() < 1e-9);
    assert!(!r.crib_read && r.arcs == 0 && r.findings() == 0);
    let text = r.text(Lang::En, &mm);
    assert!(text.starts_with("Job: "), "{text}");
    assert!(text.contains("The crib table was not read"), "{text}");
    assert!(text.contains("Nothing found before the job"), "{text}");
}

#[test]
fn every_tool_is_set_against_the_crib_by_the_width_it_cuts() {
    let s = Settings::default();
    let passes = passes_of(&s);
    let r = job_report(&passes, &s, 0, Some(&table())).unwrap();
    assert_eq!(r.tools[0].found, Some((1, "End mill 0.2 mm".to_string())));
    assert_eq!(r.findings(), 0);
    // a tool that no tool of the crib cuts is a finding
    let mut thin = s.clone();
    thin.set("fine_tool_mm", "0.15").unwrap();
    let r = job_report(&passes_of(&thin), &thin, 0, Some(&table())).unwrap();
    assert_eq!(r.tools[0].found, None);
    assert_eq!(r.findings(), 1);
    assert!(r
        .text(Lang::En, &mm)
        .contains("the crib has no tool that cuts that width"));
    // a V-bit is taken at the depth of the job: 30 degrees at 0.06 mm plus the flat of 0.1 mm cuts 0.1321 mm
    let v = Pass {
        paths: vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]],
        diameter_mm: 0.1321,
        ..Pass::default()
    };
    let r = job_report(&[v], &s, 0, Some(&table())).unwrap();
    assert_eq!(r.tools[0].found.as_ref().map(|t| t.0), Some(2));
}

#[test]
fn the_report_is_told_in_each_of_the_eight_languages() {
    let s = Settings::default();
    let r = job_report(&passes_of(&s), &s, 2, Some(&table())).unwrap();
    let mut firsts = Vec::new();
    for l in Lang::ALL {
        let text = r.text(l, &mm);
        assert!(text.contains("81.023"), "{} {text}", l.code());
        assert!(text.lines().count() >= 6, "{}", l.code());
        firsts.push(text.lines().next().unwrap().to_string());
    }
    firsts.sort();
    firsts.dedup();
    assert_eq!(firsts.len(), 8, "{firsts:?}");
    // two places where the tool does not fit are a finding
    assert!(r
        .text(Lang::Pt, &mm)
        .contains("sítios onde a ferramenta não cabe"));
    assert_eq!(r.findings(), 1);
}

#[test]
fn the_arcs_of_the_program_are_counted_in_the_report() {
    let mut s = Settings::default();
    s.set("arc_fit_tolerance_mm", "0.01").unwrap();
    let r = job_report(&passes_of(&s), &s, 0, None).unwrap();
    assert!(r.arcs >= 1 && r.arcs < r.moves, "{} of {}", r.arcs, r.moves);
    assert!(r.text(Lang::En, &mm).contains(" are arcs"));
}

#[test]
fn the_checks_of_a_layer_count_the_places_where_the_tool_does_not_fit() {
    let s = Settings::default();
    // two tracks 0.1 mm apart, closer than the 0.2 mm tool
    let track = |y: f64| Shape::Stroke {
        x0: 0.0,
        y0: y,
        x1: 10.0,
        y1: y,
        w: 0.3,
    };
    let board = Board {
        scene: Scene {
            shapes: vec![track(0.0), track(0.4)],
            notes: Vec::new(),
        },
        ..Board::default()
    };
    let c = checks(&board, &s).unwrap();
    assert!(c.gaps >= 1, "{c:?}");
    assert!(c.total() >= 1 && !c.details.is_empty());
    assert!(c.text(Lang::En).starts_with("Checks: "));
    for l in Lang::ALL {
        assert!(!c.text(l).is_empty());
    }
    let wide = Board {
        scene: Scene {
            shapes: vec![track(0.0), track(3.0)],
            notes: Vec::new(),
        },
        ..Board::default()
    };
    let c = checks(&wide, &s).unwrap();
    assert_eq!(c.gaps, 0);
    assert_eq!(c.text(Lang::En), "Checks: nothing found");
}

#[test]
fn the_command_tells_the_report_and_ends_with_2_when_there_is_something_to_look_at() {
    let (code, out, err) = run(&[
        "report",
        "examples/sample.gtl",
        "--crib-file",
        "examples/crib.csv",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("crib tool 1 (End mill 0.2 mm)"), "{out}");
    assert!(out.contains("Checks: nothing found"), "{out}");
    let (code, out, err) = run(&[
        "report",
        "examples/sample.gtl",
        "--crib-file",
        "examples/crib.csv",
        "--fine-tool-mm",
        "0.15",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(
        out.contains("the crib has no tool that cuts that width"),
        "{out}"
    );
    assert!(err.contains("verification not passed: 1 findings"), "{err}");
    // a missing layer is a failure of the work and a missing argument is a wrong use
    assert_eq!(run(&["report", "examples/no_such.gtl"]).0, 1);
    assert_eq!(run(&["report"]).0, 64);
    // the language of the system tells the report
    let es = Command::new(BIN)
        .args(["report", "examples/sample.gtl"])
        .current_dir(root())
        .env("LC_ALL", "es_ES.UTF-8")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&es.stdout).starts_with("Trabajo: "));
}
