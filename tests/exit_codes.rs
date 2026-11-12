//! The exit codes of the convention of the suite on the command line: 0 for a run that went well, 1 for a failure of the work (a file that is not there), 64 for a wrong use.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn code(args: &[&str]) -> (i32, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the program runs");
    (
        out.status.code().expect("the program ends by itself"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_run_that_goes_well_ends_with_0() {
    assert_eq!(code(&["help"]).0, 0);
    assert_eq!(code(&["info", "examples/sample.gtl"]).0, 0);
}

#[test]
fn an_unknown_command_or_setting_ends_with_64() {
    let (c, e) = code(&["frobnicate"]);
    assert_eq!((c, e.contains("unknown command")), (64, true), "{e}");
    let (c, e) = code(&["config", "--nonsense", "1"]);
    assert_eq!(c, 64, "{e}");
}

#[test]
fn what_a_command_needs_and_is_missing_ends_with_64() {
    let (c, e) = code(&["info", "--feed"]);
    assert_eq!(c, 64, "{e}");
    let (c, e) = code(&["kind"]);
    assert_eq!(c, 64, "{e}");
}

#[test]
fn a_file_that_cannot_be_read_ends_with_1() {
    let (c, e) = code(&["info", "examples/there_is_no_such_file.gtl"]);
    assert_eq!(c, 1, "{e}");
}
