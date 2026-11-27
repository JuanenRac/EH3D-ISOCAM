//! The limits of the machine a program is for: the settings that describe it, and the command line refusing (or, with --force, warning about) a program that would leave the table, cut
//! too deep or run the spindle too fast, and the dry run of a program (`simulate`) with its time, its box and its findings in the eight languages. Nothing here has run on a machine.

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

// ---- the dry run ------------------------------------------------------------------------------------------------------------------------------------------

/// Runs the program with a language of the system.
fn run_in(lang: &str, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .env("LC_ALL", lang)
        .current_dir(root())
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_dry_run_of_a_program_of_the_command_line_has_a_time_a_box_and_no_findings() {
    let d = dir("sim_ok");
    let nc = d.join("a.nc");
    let (ok, _, err) = isolate(&nc, &[]);
    assert!(ok, "{err}");
    let (ok, out, err) = run(&["simulate", nc.to_str().unwrap(), "--rapid-mm-min", "3000"]);
    assert!(ok, "{out}{err}");
    assert!(out.contains("time: ") && out.contains("cutting"), "{out}");
    assert!(
        out.contains("travel: X ") && out.contains("deepest cut 0.060 mm"),
        "{out}"
    );
    assert!(out.contains("no warnings"), "{out}");
    // without the speed of the rapid moves there is no time, and it says why
    let (ok, out, _) = run(&["simulate", nc.to_str().unwrap()]);
    assert!(ok && out.contains("rapid_mm_min"), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn isolate_prints_the_estimated_time_only_when_the_rapid_speed_is_known() {
    let d = dir("iso_time");
    let nc = d.join("a.nc");
    let (ok, out, err) = isolate(&nc, &[]);
    assert!(ok && !out.contains("estimated time"), "{out}{err}");
    let _ = std::fs::remove_file(&nc);
    let (ok, out, err) = isolate(&nc, &["--rapid-mm-min", "3000"]);
    assert!(ok && out.contains("estimated time: "), "{out}{err}");
    // the same time as the dry run of the program that was written
    let (ok, sim, err) = run(&["simulate", nc.to_str().unwrap(), "--rapid-mm-min", "3000"]);
    assert!(ok && sim.contains("time: "), "{sim}{err}");
    // a plot is not a program for a machine: no time
    let plt = d.join("a.plt");
    let (ok, out, err) = isolate(&plt, &["--rapid-mm-min", "3000"]);
    assert!(ok && !out.contains("estimated time"), "{out}{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_dry_run_speaks_the_language_of_the_system() {
    let d = dir("sim_lang");
    let nc = d.join("a.nc");
    let (ok, _, err) = isolate(&nc, &[]);
    assert!(ok, "{err}");
    for (lang, word) in [
        ("es_ES.UTF-8", "sin avisos"),
        ("de_DE.UTF-8", "keine Warnungen"),
        ("fr_FR.UTF-8", "aucun avertissement"),
        ("it_IT.UTF-8", "nessun avviso"),
        ("ja_JP.UTF-8", "警告なし"),
        ("zh_CN.UTF-8", "没有警告"),
        ("pt_PT.UTF-8", "sem avisos"),
        ("en_US.UTF-8", "no warnings"),
    ] {
        let (ok, out, err) = run_in(lang, &["simulate", nc.to_str().unwrap()]);
        assert!(ok && out.contains(word), "{lang}: {out}{err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_dry_run_fails_with_the_findings_of_a_bad_program() {
    let d = dir("sim_bad");
    let nc = d.join("bad.nc");
    std::fs::write(&nc, "G21\nG90\nG0 X5 Y5\nG0 Z-1\nG1 X20 F100\n").unwrap();
    let (ok, out, err) = run_in("en_US.UTF-8", &["simulate", nc.to_str().unwrap()]);
    assert!(!ok, "{out}");
    assert!(
        out.contains("warning: line 4: a rapid move goes into the material"),
        "{out}"
    );
    assert!(
        out.contains("line 5: it cuts with the spindle off"),
        "{out}"
    );
    assert!(out.contains("the program does not end"), "{out}");
    assert!(err.contains("3 findings"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_dry_run_applies_the_limits_of_the_machine() {
    let d = dir("sim_limits");
    let nc = d.join("a.nc");
    let (ok, _, err) = isolate(&nc, &[]);
    assert!(ok, "{err}");
    let (ok, out, _) = run(&[
        "simulate",
        nc.to_str().unwrap(),
        "--machine-profile",
        "ccd-mtc",
    ]);
    assert!(ok, "{out}");
    let (ok, out, err) = run(&[
        "simulate",
        nc.to_str().unwrap(),
        "--machine-x-mm",
        "15",
        "--machine-y-mm",
        "10",
    ]);
    assert!(!ok && out.contains("outside the table"), "{out}{err}");
    // a program that is not there, and no program at all
    let (ok, _, err) = run(&["simulate", d.join("none.nc").to_str().unwrap()]);
    assert!(!ok && err.contains("none.nc"), "{err}");
    let out = Command::new(BIN)
        .arg("simulate")
        .current_dir(root())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_hole_outside_the_table_stops_the_drill_file_and_the_plot_of_the_holes() {
    let d = dir("holes");
    let holes = d.join("board.drl");
    std::fs::write(
        &holes,
        "M48\nMETRIC,TZ\nT01C0.8\n%\nT01\nX1.0Y2.0\nX20.0Y2.0\nM30\n",
    )
    .unwrap();
    for name in ["out.ncd", "out.plt"] {
        let out = d.join(name);
        let line = |extra: &[&str]| {
            let mut args = vec!["drills", holes.to_str().unwrap(), out.to_str().unwrap()];
            args.extend_from_slice(extra);
            run(&args)
        };
        let table = ["--machine-x-mm", "10", "--machine-y-mm", "10"];
        let (ok, _, err) = line(&table);
        assert!(!ok, "{name}");
        assert!(
            err.contains("hole 2 of 2") && err.contains("outside the table"),
            "{name}: {err}"
        );
        assert!(!out.exists(), "{name}");
        let (ok, _, err) = line(&[&table[..], &["--force"]].concat());
        assert!(ok && err.contains("outside the table"), "{name}: {err}");
        assert!(out.exists(), "{name}");
        // with no table nothing is checked, as before
        let (ok, _, err) = line(&[]);
        assert!(ok, "{name}: {err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_plot_of_the_isolation_outside_the_table_is_refused() {
    let d = dir("plot");
    let out = d.join("out.plt");
    let table = ["--machine-x-mm", "10", "--machine-y-mm", "10"];
    let (ok, _, err) = isolate(&out, &table);
    assert!(!ok);
    assert!(err.contains("outside the table"), "{err}");
    assert!(!out.exists());
    let (ok, _, err) = isolate(&out, &[&table[..], &["--force"]].concat());
    assert!(ok && err.contains("outside the table"), "{err}");
    assert!(out.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_window_says_what_a_forced_program_breaks_and_refuses_holes_outside_the_table() {
    use eh3d_isocam::lang::Lang;
    use eh3d_isocam::window::{Status, Window};
    let d = dir("window");
    let mut w = Window::new(Lang::En);
    assert!(
        w.open(&root().join("examples/sample.gtl")),
        "{:?}",
        w.status
    );
    let set = |w: &mut Window, key: &str, text: &str| {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    };
    set(&mut w, "machine_x_mm", "10");
    set(&mut w, "machine_y_mm", "10");
    for name in ["no.nc", "no.plt"] {
        assert!(!w.export(&d.join(name)), "{name}");
        assert!(!d.join(name).exists(), "{name}");
    }
    w.set_force(true);
    for name in ["forced.nc", "forced.plt"] {
        w.notes.clear();
        assert!(w.export(&d.join(name)), "{name}: {:?}", w.status);
        assert!(
            w.notes.iter().any(|n| n.contains("outside the table")),
            "{name}: {:?}",
            w.notes
        );
    }
    let holes = d.join("board.drl");
    std::fs::write(&holes, "M48\nMETRIC,TZ\nT01C0.8\n%\nT01\nX20.0Y2.0\nM30\n").unwrap();
    assert!(w.open_drills(&holes), "{:?}", w.status);
    assert!(!w.export_drills(&d.join("holes.ncd")));
    assert!(!d.join("holes.ncd").exists());
    assert!(
        matches!(&w.status, Status::Failed(why) if why.contains("outside the table")),
        "{:?}",
        w.status
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn small_table() -> eh3d_isocam::machine::Limits {
    eh3d_isocam::machine::Limits {
        table_mm: Some((100.0, 60.0)),
        max_rpm: 30000.0,
        ..Default::default()
    }
}

#[test]
fn a_program_that_cannot_be_judged_does_not_keep_the_limits() {
    use eh3d_isocam::machine::check_program;
    for line in [
        "$H",
        "$130=999",
        "G92 X0",
        "G53 G0 X0",
        "G28",
        "G10 L20 P1 X0",
        "G1 X5 #3 F100",
        "G1 X[900] F100",
        "G91.5",
        "M2\nG0 X900",
    ] {
        let program = format!("G21\nG90\nG0 X1 Y1\n{line}\n");
        let said = check_program(&program, &small_table());
        // an unknown code with no axis words only warns: the others stop the check
        if line == "G91.5" {
            continue;
        }
        assert!(said.is_err(), "{line:?}: {said:?}");
        assert!(
            said.unwrap_err().contains("line 4") || line.contains('\n'),
            "{line:?}"
        );
    }
    assert!(check_program("G21\nG90\nM3 S24000\nG0 X1 Y1\nM5\nM2\n", &small_table()).is_ok());
}

#[test]
fn a_spindle_started_too_fast_is_found_even_when_nothing_moves_at_that_speed() {
    use eh3d_isocam::machine::check_program;
    let said = check_program("G21\nM3 S99999\nS100\nG0 X1 Y1\nM5\n", &small_table());
    assert!(said.unwrap_err().contains("99999"));
    assert!(check_program("G21\nS99999\nM3 S100\nG0 X1 Y1\nM5\n", &small_table()).is_ok());
}

#[test]
fn a_pause_with_the_spindle_on_is_a_finding_of_the_dry_run() {
    let text =
        |stop: &str| format!("G21\nG90\nG0 Z2\nM3 S24000\n{stop}\nG1 Z-0.1 F60\nG0 Z2\nM5\nM2\n");
    for stop in ["M0", "M1"] {
        let r = eh3d_isocam::simulate::run(&text(stop), 3000.0);
        assert!(
            r.warnings
                .iter()
                .any(|w| w.line == 5 && format!("{:?}", w.kind).contains("spindle on")),
            "{stop}: {:?}",
            r.warnings
        );
    }
    assert!(eh3d_isocam::simulate::run(&text("M5\nM0\nM3"), 3000.0)
        .warnings
        .is_empty());
}

#[test]
fn the_name_of_the_height_map_cannot_stop_or_resume_the_controller() {
    use eh3d_isocam::isolate::Pass;
    let d = dir("realtime_name");
    let map = d.join("bench!~.map");
    std::fs::write(&map, "levelling 1\ngrid 0 0 10 10 2 2\n0 0\n0 0.1\n").unwrap();
    let s = Settings {
        heightmap_file: map.to_string_lossy().into_owned(),
        ..Settings::default()
    };
    let pass = Pass {
        paths: vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]],
        diameter_mm: 0.2,
        ..Pass::default()
    };
    let (program, _) = eh3d_isocam::board::gcode_program(&[pass], &s, false).unwrap();
    assert!(
        !program.contains(['!', '~', '?']),
        "{}",
        program.lines().next().unwrap()
    );
}
