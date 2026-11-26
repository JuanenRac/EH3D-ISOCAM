//! The `.eh3d` project: the copper layers are read from the job, each program of isolation is kept in the branch of this program only, a job that changes makes them out of date, and
//! the window chooses the copper layer of the job and saves its program. Every file is made here from invented data.

use eh3d_gerberviewer::eh3dfile::{self as suite, SOURCE_BRANCH};
use eh3d_isocam::eh3dfile::{self, KIT};
use eh3d_isocam::lang::Lang;
use eh3d_isocam::window::{Status, Window};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");
/// Two square pads of 2 mm, 8 mm apart.
const COPPER: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,2.0X2.0*%\nD10*\nX00000Y00000D03*\nX100000Y00000D03*\nM02*\n";
/// One round pad of 2 mm.
const OTHER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,2.0*%\nD10*\nX50000Y50000D03*\nM02*\n";

fn write_branch(path: &Path, branch: &str, files: &[(&str, &[u8])]) {
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (name, bytes) in files {
        update.entries.insert((*name).to_string(), bytes.to_vec());
    }
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .save_branch(&Writer::for_app(branch), update, &SaveOptions::default())
        .unwrap();
}

fn write_job(path: &Path, top: &str) {
    let record = br#"{"format_version":"1.0.0","layers":[
        {"path":"layers/top.gtl","kind":"copper","name":"Top copper"},
        {"path":"layers/bottom.gbl","kind":"copper","name":"Bottom copper"}]}"#;
    write_branch(
        path,
        SOURCE_BRANCH,
        &[
            ("project.json", record),
            ("layers/top.gtl", top.as_bytes()),
            ("layers/bottom.gbl", COPPER.as_bytes()),
        ],
    );
}

fn project(dir: &Path) -> PathBuf {
    let path = dir.join("board.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Invented board".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    write_branch(&path, "pcb", &[("board.json", b"{}")]);
    write_job(&path, COPPER);
    path
}

fn hash(path: &Path, branch: &str) -> String {
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .branch(branch)
        .unwrap()
        .hash
        .clone()
}

fn run(path: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .arg("project")
        .arg(args[0])
        .arg(path)
        .args(&args[1..])
        .output()
        .expect("the program runs")
}

#[test]
fn the_copper_layers_are_chosen_by_path_or_name_and_name_their_programs() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let layers = eh3dfile::copper_layers(&path).unwrap();
    assert_eq!(layers.len(), 2);
    let top = eh3dfile::choose(&layers, "top.gtl").unwrap();
    assert_eq!(top, eh3dfile::choose(&layers, "layers/top.gtl").unwrap());
    assert!(eh3dfile::choose(&layers, "inner.g2").is_err());
    assert_eq!(eh3dfile::result_name(top, "nc"), "iso-top.nc");
    write_branch(
        &path,
        SOURCE_BRANCH,
        &[("project.json", br#"{"layers":[]}"#)],
    );
    let why = eh3dfile::copper_layers(&path).unwrap_err();
    assert!(why.contains("no copper layer"), "{why}");
}

#[test]
fn isolate_keeps_a_program_for_each_copper_layer_in_its_own_branch_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let (pcb, job) = (hash(&path, "pcb"), hash(&path, SOURCE_BRANCH));
    let out = run(&path, &["isolate"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(hash(&path, "pcb"), pcb);
    assert_eq!(hash(&path, SOURCE_BRANCH), job);
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["iso-bottom.nc", "iso-top.nc"]);
    assert_eq!(state.inputs, ["layers/bottom.gbl", "layers/top.gtl"]);
    assert!(!state.out_of_date);
    let code = suite::read_result(&KIT, &path, "iso-top.nc").unwrap();
    let code = String::from_utf8(code).unwrap();
    assert!(
        code.contains("top.gtl"),
        "the header names the layer: {code}"
    );
    assert_eq!(run(&path, &["info"]).status.code(), Some(0));
}

#[test]
fn a_job_that_changes_makes_the_programs_out_of_date_until_they_are_made_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    assert!(run(&path, &["isolate", "--layer", "top.gtl"])
        .status
        .success());
    assert!(run(&path, &["isolate", "--layer", "bottom.gbl"])
        .status
        .success());
    assert_eq!(
        suite::state(&KIT, &path).unwrap().results,
        ["iso-bottom.nc", "iso-top.nc"]
    );
    write_job(&path, OTHER);
    assert_eq!(run(&path, &["info"]).status.code(), Some(2));
    assert!(run(&path, &["isolate", "--layer", "top.gtl"])
        .status
        .success());
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["iso-top.nc"]);
    assert!(!state.out_of_date);
}

#[test]
fn a_refused_isolation_leaves_the_project_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let before = std::fs::read(&path).unwrap();
    for args in [
        &["isolate", "--layer", "missing.gtl"][..],
        &["isolate", "--layer"],
        &["isolate", "--nothing-like-this"],
        &["isolate", "--fine-tool-mm", "20"],
        &["mill"],
    ] {
        assert_ne!(run(&path, args).status.code(), Some(0), "{args:?}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "{args:?}");
    }
}

#[test]
fn the_window_opens_the_copper_of_a_project_and_saves_its_programs() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let mut w = Window::new(Lang::En);
    assert!(!w.save_project());
    assert_eq!(w.status, Status::NoProject);
    assert!(w.open_dropped(&path), "{:?}", w.status);
    assert_eq!(w.job_layers.len(), 2);
    assert_eq!(w.job_index, Some(0));
    assert!(matches!(w.status, Status::ProjectOpened { layers: 2, .. }));
    assert_eq!(w.layer.as_ref().unwrap().path, dir.path().join("top.gtl"));
    assert!(w.save_project(), "{:?}", w.status);
    // the layer keeps the path it had before the program was made from a copy
    assert_eq!(w.layer.as_ref().unwrap().path, dir.path().join("top.gtl"));
    assert!(w.choose_job_layer(1));
    assert!(w.save_project(), "{:?}", w.status);
    assert!(!w.choose_job_layer(9));
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["iso-bottom.nc", "iso-top.nc"]);
    write_job(&path, OTHER);
    assert!(w.open(&path));
    assert!(matches!(w.status, Status::ProjectStale { .. }));
}
