//! The limits of the machine a program is for: the settings that describe it, and the command line refusing (or, with --force, warning about) a program that would leave the table, cut
//! too deep or run the spindle too fast. Nothing here has run on a machine.

use eh3d_isocam::config::Settings;
use eh3d_isocam::machine::Profile;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-isocam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A folder of its own for one test, empty.
fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eh3d_isocam_safety_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Runs the program in the folder of the repository: whether it succeeded, what it printed and what it said on the error output.
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

#[test]
fn the_machine_settings_have_a_default_that_checks_nothing() {
    let s = Settings::default();
    assert_eq!(s.machine_profile, Profile::Off);
    assert_eq!(s.machine_table(), None);
    assert!(s.machine_limits().is_off());
    let text = s.to_text();
    for line in [
        "machine_profile = none\n",
        "machine_x_mm = 0\n",
        "machine_y_mm = 0\n",
        "machine_margin_mm = 0\n",
        "machine_max_depth_mm = 0\n",
        "machine_max_rpm = 0\n",
    ] {
        assert!(text.contains(line), "{line:?} is not in\n{text}");
    }
}

#[test]
fn the_machine_settings_are_written_and_read_back() {
    let s = Settings::parse(
        "machine_profile = ccd2-mtc\nmachine_margin_mm = 3\nmachine_max_depth_mm = 0.3\nmachine_max_rpm = 30000\n",
    )
    .unwrap();
    assert_eq!(s.machine_table(), Some((325.0, 270.0)));
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    // the size of the settings replaces the one of the profile, one side at a time
    let mut t = s.clone();
    t.set("machine_x_mm", "300").unwrap();
    assert_eq!(t.machine_table(), Some((300.0, 270.0)));
    // with no profile the two sides make a table of their own
    let u = Settings::parse("machine_x_mm = 200\nmachine_y_mm = 100\n").unwrap();
    assert_eq!(u.machine_table(), Some((200.0, 100.0)));
    assert_eq!(u.machine_limits().table_mm, Some((200.0, 100.0)));
    // the short name of the profile and a length with its unit
    let mut v = Settings::default();
    v.set("--machine", "ccd-mtc").unwrap();
    v.set("machine_margin_mm", "0.1in").unwrap();
    assert_eq!(v.machine_profile, Profile::CcdMtc);
    assert!((v.machine_margin_mm - 2.54).abs() < 1e-9);
}

#[test]
fn wrong_machine_settings_are_refused_with_their_name() {
    for (text, part) in [
        ("machine_profile = ccd9\n", "machine_profile 'ccd9'"),
        ("machine_x_mm = 200\n", "must both be set"),
        (
            "machine_x_mm = 5\nmachine_y_mm = 100\n",
            "machine_x_mm is 5",
        ),
        (
            "machine_x_mm = -2\nmachine_y_mm = 100\n",
            "machine_x_mm is -2",
        ),
        ("machine_margin_mm = -1\n", "machine_margin_mm is -1"),
        (
            "machine_profile = ccd2-mtc\nmachine_margin_mm = 140\n",
            "machine_margin_mm is 140",
        ),
        (
            "machine_x_mm = 200\nmachine_y_mm = 100\nmachine_margin_mm = 60\n",
            "leaves no room",
        ),
        (
            "machine_max_depth_mm = 0.001\n",
            "machine_max_depth_mm is 0.001",
        ),
        (
            "machine_max_depth_mm = 0.03\n",
            "deeper than machine_max_depth_mm",
        ),
        ("machine_max_rpm = 500\n", "machine_max_rpm is 500"),
        ("machine_max_rpm = 10000\n", "faster than machine_max_rpm"),
    ] {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?} gave {e:?}");
    }
}

#[test]
fn a_board_inside_the_table_is_written_as_before() {
    let d = dir("inside");
    let out = d.join("a.nc");
    let (ok, _, err) = isolate(&out, &["--machine-profile", "ccd-mtc"]);
    assert!(ok, "{err}");
    let with = std::fs::read_to_string(&out).unwrap();
    let plain = d.join("b.nc");
    let (ok, _, err) = isolate(&plain, &[]);
    assert!(ok, "{err}");
    assert_eq!(with, std::fs::read_to_string(&plain).unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_board_larger_than_the_table_is_refused_with_the_line_and_nothing_is_written() {
    let d = dir("outside");
    let out = d.join("a.nc");
    let (ok, _, err) = isolate(&out, &["--machine-x-mm", "15", "--machine-y-mm", "10"]);
    assert!(!ok);
    assert!(err.contains("outside the table (15 x 10 mm"), "{err}");
    assert!(err.contains("line ") && err.contains("--force"), "{err}");
    assert!(!out.exists(), "a program that leaves the table was saved");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn with_force_the_program_is_written_and_the_warning_is_shown() {
    let d = dir("force");
    let out = d.join("a.nc");
    let (ok, _, err) = isolate(
        &out,
        &["--machine-x-mm", "15", "--machine-y-mm", "10", "--force"],
    );
    assert!(ok, "{err}");
    assert!(
        err.contains("warning: line ") && err.contains("outside the table"),
        "{err}"
    );
    assert!(out.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_margin_from_the_edges_counts() {
    let d = dir("margin");
    let out = d.join("a.nc");
    // the board starts about two millimetres from the origin: a margin of ten does not leave room
    let (ok, _, err) = isolate(
        &out,
        &["--machine-profile", "ccd2-mtc", "--machine-margin-mm", "10"],
    );
    assert!(!ok && err.contains("10 mm kept from its edges"), "{err}");
    let (ok, _, err) = isolate(
        &out,
        &["--machine-profile", "ccd2-mtc", "--machine-margin-mm", "1"],
    );
    assert!(ok, "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_depth_and_the_spindle_of_the_job_may_not_pass_the_limits_of_the_machine() {
    let d = dir("limits");
    let out = d.join("a.nc");
    let (ok, _, err) = isolate(&out, &["--machine-max-depth-mm", "0.05"]);
    assert!(
        !ok && err.contains("deeper than machine_max_depth_mm"),
        "{err}"
    );
    let (ok, _, err) = isolate(&out, &["--machine-max-rpm", "20000"]);
    assert!(!ok && err.contains("faster than machine_max_rpm"), "{err}");
    let (ok, _, err) = isolate(
        &out,
        &[
            "--machine-max-depth-mm",
            "0.06",
            "--machine-max-rpm",
            "24000",
        ],
    );
    assert!(ok, "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_settings_file_of_the_example_has_the_machine_keys() {
    let text = std::fs::read_to_string(root().join("examples").join("isocam.cfg")).unwrap();
    for key in [
        "machine_profile",
        "machine_x_mm",
        "machine_y_mm",
        "machine_margin_mm",
        "machine_max_depth_mm",
        "machine_max_rpm",
    ] {
        assert!(text.contains(key), "{key} is not in examples/isocam.cfg");
    }
}
