//! The command line reads files as bytes (a size limit, text that is not UTF-8), shows the notices of the Gerber reader by their stable code, and says nothing when nothing changed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BOARD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1.0*%\nD10*\nX50000Y50000D03*\nM02*\n";
const NO_UNITS: &str = "%FSLAX24Y24*%\n%ADD10C,1.0*%\nD10*\nX50000Y50000D03*\nM02*\n";

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_isocam_{name}_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn latin1_board() -> Vec<u8> {
    // a comment with an e with an acute accent written as one byte (Latin-1), which is not UTF-8
    let mut b = b"G04 Capa de cobre, fabricaci".to_vec();
    b.push(0xF3);
    b.extend_from_slice(b"n*\n");
    b.extend_from_slice(BOARD.as_bytes());
    b
}

#[test]
fn text_that_is_not_utf8_is_read_and_the_warning_has_the_code_of_the_reader() {
    let dir = folder("latin1");
    let file = dir.join("l.gtl");
    fs::write(&file, latin1_board()).unwrap();
    let f = file.to_str().unwrap();
    let (ok, _, err) = run(&["info", f]);
    assert!(ok, "{err}");
    assert!(err.contains("[input-not-utf8]"), "{err}");
    let (ok, _, err) = run(&["info", f, "--hide", "input-not-utf8"]);
    assert!(ok, "{err}");
    assert!(!err.contains("input-not-utf8"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_file_over_the_limit_is_refused() {
    let dir = folder("limit");
    let file = dir.join("b.gtl");
    fs::write(&file, BOARD).unwrap();
    let f = file.to_str().unwrap();
    let (ok, _, err) = run(&["info", f, "--max-bytes", "10"]);
    assert!(!ok && err.contains("above the limit"), "{err}");
    let (ok, _, err) = run(&["info", f, "--max-bytes", "many"]);
    assert!(!ok && err.contains("not a whole number"), "{err}");
    let (ok, _, err) = run(&["info", f]);
    assert!(ok, "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_notice_is_hidden_by_its_code_and_a_misspelt_code_is_an_error() {
    let dir = folder("hide");
    let file = dir.join("n.gtl");
    fs::write(&file, NO_UNITS).unwrap();
    let f = file.to_str().unwrap();
    let (ok, _, err) = run(&["info", f]);
    assert!(ok, "{err}");
    assert!(err.contains("[units-missing]"), "{err}");
    let (_, _, err) = run(&["info", f, "--hide", "units-missing"]);
    assert!(!err.contains("units-missing"), "{err}");
    let (ok, _, err) = run(&["info", f, "--hide", "units-mising"]);
    assert!(!ok && err.contains("not a code"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_note_about_the_lighter_paths_is_printed_only_when_they_changed() {
    let dir = folder("quiet");
    let file = dir.join("b.gtl");
    fs::write(&file, BOARD).unwrap();
    let f = file.to_str().unwrap();
    // a tolerance far below the size of a segment of the circle takes no point out
    let (ok, _, err) = run(&["info", f, "--optimize-tolerance-mm", "0.000001"]);
    assert!(ok, "{err}");
    assert!(!err.contains("points instead of"), "{err}");
    // a coarse one does
    let (ok, _, err) = run(&["info", f, "--optimize-tolerance-mm", "0.2"]);
    assert!(ok, "{err}");
    assert!(err.contains("points instead of"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}
