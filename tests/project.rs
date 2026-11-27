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
    assert_eq!(eh3dfile::result_name(&layers, top, "nc"), "iso-top.nc");
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

/// A job whose copper layers have these paths inside `manufacturing/`.
fn write_layers(path: &Path, files: &[(&str, &str)]) {
    let layers: Vec<String> = files
        .iter()
        .map(|(rel, _)| format!(r#"{{"path":"{rel}","kind":"copper","name":"{rel}"}}"#))
        .collect();
    let record = format!(
        r#"{{"format_version":"1.0.0","layers":[{}]}}"#,
        layers.join(",")
    );
    let mut entries: Vec<(&str, &[u8])> = vec![("project.json", record.as_bytes())];
    entries.extend(files.iter().map(|(rel, text)| (*rel, text.as_bytes())));
    write_branch(path, SOURCE_BRANCH, &entries);
}

#[test]
fn two_layers_with_the_same_stem_each_keep_their_own_program() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    write_layers(
        &path,
        &[("layers/board.gtl", COPPER), ("layers/board.gbl", OTHER)],
    );
    let out = run(&path, &["isolate"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["iso+board.gbl.nc", "iso+board.gtl.nc"]);
    for (name, source) in [
        ("iso+board.gtl.nc", "board.gtl"),
        ("iso+board.gbl.nc", "board.gbl"),
    ] {
        let code = String::from_utf8(suite::read_result(&KIT, &path, name).unwrap()).unwrap();
        assert!(code.contains(source), "{name}: {code}");
    }
    // one layer at a time keeps the other, and the window names them the same way
    assert!(run(&path, &["isolate", "--layer", "board.gtl"])
        .status
        .success());
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.choose_job_layer(1));
    assert!(w.save_project(), "{:?}", w.status);
    assert_eq!(
        suite::state(&KIT, &path).unwrap().results,
        ["iso+board.gbl.nc", "iso+board.gtl.nc"]
    );
}

#[test]
fn a_file_name_that_two_layers_share_is_not_chosen_by_guess() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    write_layers(
        &path,
        &[("layers/a/top.gtl", COPPER), ("layers/b/top.gtl", OTHER)],
    );
    let layers = eh3dfile::copper_layers(&path).unwrap();
    let why = eh3dfile::choose(&layers, "top.gtl").unwrap_err();
    assert!(
        why.contains("layers/a/top.gtl") && why.contains("layers/b/top.gtl"),
        "{why}"
    );
    let second = eh3dfile::choose(&layers, "layers/b/top.gtl").unwrap();
    assert_eq!(second.bytes, OTHER.as_bytes());
    assert_eq!(
        eh3dfile::result_name(&layers, second, "nc"),
        "iso+b+top.gtl.nc"
    );
    let before = std::fs::read(&path).unwrap();
    assert!(!run(&path, &["isolate", "--layer", "top.gtl"])
        .status
        .success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn no_program_is_written_over_a_project_or_the_settings_it_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let before = std::fs::read(&path).unwrap();
    let layer = dir.path().join("one.gtl");
    std::fs::write(&layer, COPPER).unwrap();
    let isolate = |out: &Path, extra: &[&std::ffi::OsStr]| {
        Command::new(BIN)
            .arg("isolate")
            .arg(&layer)
            .arg(out)
            .args(extra)
            .output()
            .expect("the program runs")
    };
    let out = isolate(&path, &[]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("project"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let settings = dir.path().join("my.cfg");
    std::fs::write(&settings, "depth_mm = 0.05\n").unwrap();
    let out = isolate(&settings, &["--config".as_ref(), settings.as_os_str()]);
    assert!(!out.status.success());
    assert_eq!(
        std::fs::read_to_string(&settings).unwrap(),
        "depth_mm = 0.05\n"
    );

    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(!w.export(&path));
    assert_eq!(w.status, Status::SameFile);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn the_header_names_the_hash_of_the_layer_that_was_read_not_of_what_is_on_disk_later() {
    let hash_of = |text: &str| eh3d_parser_gcode::emit::sha256_hex(text.as_bytes());
    let dir = tempfile::tempdir().unwrap();
    // a layer that changes on disk after it was opened
    let layer = dir.path().join("one.gtl");
    std::fs::write(&layer, COPPER).unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&layer), "{:?}", w.status);
    std::fs::write(&layer, OTHER).unwrap();
    let out = dir.path().join("one.nc");
    assert!(w.export(&out), "{:?}", w.status);
    let code = std::fs::read_to_string(&out).unwrap();
    assert!(code.contains(&hash_of(COPPER)), "{code}");
    // a file with the name of the layer of the job lies next to the project and is not the layer
    let path = project(dir.path());
    std::fs::write(dir.path().join("top.gtl"), OTHER).unwrap();
    assert!(w.open(&path), "{:?}", w.status);
    let beside = dir.path().join("beside.nc");
    assert!(w.export(&beside), "{:?}", w.status);
    let code = std::fs::read_to_string(&beside).unwrap();
    assert!(code.contains(&hash_of(COPPER)), "{code}");
    assert!(!code.contains(&hash_of(OTHER)), "{code}");
}

#[test]
fn the_window_says_when_keeping_a_program_removed_the_signature_of_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let signer = eh3d_project::Ed25519Signer::from_seed(&[7; 32]);
    Project::open(&path, &OpenOptions::default())
        .unwrap()
        .sign_project(&signer, &SaveOptions::default())
        .unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    // the window asks first, and nothing is written until the person agrees
    let before = std::fs::read(&path).unwrap();
    assert!(!w.save_project(), "{:?}", w.status);
    let asked: Vec<String> = w
        .pending
        .as_ref()
        .expect("the window asks")
        .iter()
        .map(|s| s.text("en"))
        .collect();
    assert!(asked.iter().any(|n| n.contains("signature")), "{asked:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // not agreeing leaves the project as it was, and agreeing afterwards does nothing
    w.cancel_save();
    assert!(w.pending.is_none() && !w.confirm_save());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // opening again forgets the question
    assert!(!w.save_project());
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.pending.is_none() && !w.confirm_save());
    // agreeing writes and tells what was removed
    assert!(!w.save_project());
    assert!(w.confirm_save(), "{:?}", w.status);
    assert!(w.pending.is_none());
    assert!(
        w.notes.iter().any(|n| n.contains("signature")),
        "{:?}",
        w.notes
    );
    // with no signature left there is nothing to ask
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.save_project(), "{:?}", w.status);
    assert!(w.pending.is_none());
}

/// Saves the project the way another program does: the change is made on a copy and the file is replaced with it, so this
/// process has no memory of that write.
fn saved_by_another(path: &Path, text: &str) {
    let copy = path.with_file_name("another.eh3d");
    std::fs::copy(path, &copy).unwrap();
    write_branch(&copy, "pcb", &[("board.json", text.as_bytes())]);
    std::fs::copy(&copy, path).unwrap();
    std::fs::remove_file(&copy).unwrap();
}

#[test]
fn a_project_another_program_saved_is_not_written_over_and_can_be_read_again_or_saved_apart() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    saved_by_another(&path, "{\"made\":1}");
    let bytes = std::fs::read(&path).unwrap();
    assert!(!w.save_project());
    assert!(
        matches!(w.status, Status::ProjectChanged { .. }),
        "{:?}",
        w.status
    );
    assert!(w.status.is_problem());
    assert!(w.status.text(Lang::En).contains("board.eh3d"));
    assert_eq!(w.changed.as_deref(), Some(path.as_path()));
    assert!(w.pending.is_none() && !w.confirm_save());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // a file that is there is never written over, under another name the work goes beside what the other program saved
    assert!(!w.save_project_as(&path));
    let apart = dir.path().join("mine.eh3d");
    assert!(w.save_project_as(&apart), "{:?}", w.status);
    assert_eq!(w.project.as_deref(), Some(apart.as_path()));
    assert_eq!(hash(&apart, "pcb"), hash(&path, "pcb"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // what the window wrote itself does not count
    assert!(w.save_project(), "{:?}", w.status);
    // reading it again takes the offer away and lets it be saved
    saved_by_another(&apart, "{\"made\":2}");
    assert!(!w.save_project());
    assert!(w.reopen_project(), "{:?}", w.status);
    assert!(w.changed.is_none() && !w.reopen_project());
    assert!(w.save_project(), "{:?}", w.status);
}

#[test]
fn a_save_by_another_program_while_a_question_waits_refuses_the_agreed_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let signer = eh3d_project::Ed25519Signer::from_seed(&[7; 32]);
    Project::open(&path, &OpenOptions::default())
        .unwrap()
        .sign_project(&signer, &SaveOptions::default())
        .unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(!w.save_project(), "{:?}", w.status);
    assert!(w.pending.is_some());
    saved_by_another(&path, "{\"made\":3}");
    let bytes = std::fs::read(&path).unwrap();
    assert!(!w.confirm_save());
    assert!(
        matches!(w.status, Status::ProjectChanged { .. }),
        "{:?}",
        w.status
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    w.dismiss_changed();
    assert!(w.changed.is_none());
}
