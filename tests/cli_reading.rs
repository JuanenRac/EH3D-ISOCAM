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

fn run_in(lang: &str, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(args)
        .env("LC_ALL", lang)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_size_limit_comes_from_the_configuration_file_and_the_refusal_is_in_the_language_of_the_person(
) {
    let dir = folder("limit_cfg");
    let file = dir.join("big.gtl");
    // a layer a little over 1 MiB: comments, then the board
    let mut text = String::new();
    while text.len() < 1_100_000 {
        text.push_str("G04 padding padding padding padding padding padding*\n");
    }
    text.push_str(BOARD);
    fs::write(&file, text).unwrap();
    let cfg = dir.join("isocam.cfg");
    fs::write(&cfg, "max_input_mb = 1\n").unwrap();
    let (f, c) = (file.to_str().unwrap(), cfg.to_str().unwrap());
    let (ok, _, err) = run_in("en", &["info", f, "--config", c]);
    assert!(
        !ok && err.contains("above the limit") && err.contains("max_input_mb"),
        "{err}"
    );
    for (lang, word) in [
        ("es", "supera el límite"),
        ("de", "überschreitet die Grenze"),
        ("fr", "dépasse la limite"),
        ("it", "supera il limite"),
        ("ja", "上限を超えています"),
        ("zh", "文件超过上限"),
        ("pt", "excede o limite"),
    ] {
        let (ok, _, err) = run_in(lang, &["info", f, "--config", c]);
        assert!(!ok && err.contains(word), "{lang}: {err}");
    }
    // the same file passes with the default limit, and --max-bytes wins over the configuration file
    let (ok, _, err) = run_in("en", &["info", f]);
    assert!(ok, "{err}");
    let (ok, _, err) = run_in("en", &["info", f, "--config", c, "--max-bytes", "5000000"]);
    assert!(ok, "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_size_limit_and_the_arc_tolerance_of_the_settings_are_checked() {
    use eh3d_isocam::config::Settings;
    let s = Settings::parse("max_input_mb = 8\narc_tolerance_mm = 0.002\n").unwrap();
    assert_eq!((s.max_input_mb, s.arc_tolerance_mm), (8.0, 0.002));
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    let d = Settings::default();
    assert_eq!((d.max_input_mb, d.arc_tolerance_mm), (64.0, 0.005));
    for bad in [
        "max_input_mb = 0\n",
        "max_input_mb = 100000\n",
        "arc_tolerance_mm = 0\n",
        "arc_tolerance_mm = 5\n",
    ] {
        assert!(Settings::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn an_output_that_is_an_input_of_the_run_is_refused_and_the_input_stays_as_it_was() {
    let dir = folder("same_file");
    let file = dir.join("board.gtl");
    fs::write(&file, BOARD).unwrap();
    let f = file.to_str().unwrap();
    for args in [vec!["isolate", f, f]] {
        let (ok, _, err) = run(&args);
        assert!(!ok && err.contains("also an input"), "{args:?}: {err}");
        assert_eq!(fs::read_to_string(&file).unwrap(), BOARD, "{args:?}");
    }
    // the same file written through another spelling of its path
    let other = dir.join(".").join("board.gtl");
    let (ok, _, err) = run(&["isolate", f, other.to_str().unwrap()]);
    assert!(!ok && err.contains("also an input"), "{err}");
    assert_eq!(fs::read_to_string(&file).unwrap(), BOARD);
    let _ = fs::remove_dir_all(&dir);
}
