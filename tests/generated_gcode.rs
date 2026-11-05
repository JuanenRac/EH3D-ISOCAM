//! The G-code that the program writes is read back with the reader of the suite and walked the way the G-code debugger walks it in its strict mode: nothing in it may be a warning of
//! the reader, a rapid move into the material or a cut with the spindle off.

use eh3d_isocam::gcode::{write_in, Job};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::Order;
use eh3d_isocam::units::Unit;
use eh3d_parser_gcode::emit::{header_with_units, sha256_hex, Header};
use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};

/// What the strict mode of the debugger would report for `text`.
fn alerts(text: &str) -> Vec<String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let mut found: Vec<String> = trace
        .diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .map(|d| format!("line {}: {}", d.line, d.message))
        .collect();
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        let lowest = s.from.z.min(s.to.z);
        if rapid && lowest < 0.0 && (s.from.x != s.to.x || s.from.y != s.to.y || s.to.z < s.from.z)
        {
            found.push(format!(
                "line {}: a rapid move goes into the material",
                s.line
            ));
        }
        if !rapid && !s.spindle_on && lowest < 0.0 {
            found.push(format!("line {}: it cuts with the spindle off", s.line));
        }
        if !rapid && s.feed <= 0.0 {
            found.push(format!("line {}: a cutting move has no feed", s.line));
        }
    }
    if !trace.ended {
        found.push("the program does not end".into());
    }
    found
}

fn square(x: f64, y: f64, side: f64) -> Vec<(f64, f64)> {
    vec![
        (x, y),
        (x + side, y),
        (x + side, y + side),
        (x, y + side),
        (x, y),
    ]
}

fn pass(diameter_mm: f64, x: f64) -> Pass {
    Pass {
        paths: vec![square(x, 0.0, 5.0), square(x, 10.0, 3.0)],
        diameter_mm,
        ..Pass::default()
    }
}

#[test]
fn the_checker_used_here_does_find_a_cut_with_the_spindle_off() {
    assert!(!alerts("G21\nG1 Z-1 F100\nM2\n").is_empty());
    assert!(!alerts("G21\nM3 S1000\nG0 Z-1\nM2\n").is_empty());
    assert!(!alerts("G21\nM3 S1000\nG1 X5\nM2\n").is_empty());
    assert!(alerts("G21\nM3 S1000\nG0 Z2\nG1 Z-0.1 F60\nG1 X5 F300\nG0 Z2\nM5\nM2\n").is_empty());
}

#[test]
fn the_gcode_of_the_passes_in_mm_has_no_alert_with_a_tool_change_and_a_spindle_wait() {
    let passes = [pass(0.2, 0.0), pass(0.4, 20.0)];
    let job = Job {
        spindle_wait_s: 1.5,
        ..Job::default()
    };
    let body = write_in(&passes, &job, &Order::default(), Unit::Mm).unwrap();
    assert!(
        body.contains("M0\n") && body.contains("G4 P1.5\n"),
        "{body}"
    );
    let head = header_with_units(
        &Header {
            generator: "eh3d-isocam",
            version: "0.0.0",
            source: Some(("board.gtl", sha256_hex(b"board"))),
            tools: "0.2 mm, 0.4 mm",
        },
        "mm (G21)",
    );
    let all = format!("{head}{body}");
    let found = alerts(&all);
    assert!(found.is_empty(), "{found:?}\n{all}");
    // the reader walked the cuts: four closed paths with four sides each are at least eight cutting moves
    let trace = interpret(&parse(&all, Dialect::Grbl));
    let cuts = trace
        .segments
        .iter()
        .filter(|s| matches!(s.motion, Motion::Linear) && s.from.z < 0.0 && s.to.z < 0.0)
        .count();
    assert!(cuts >= 8, "{cuts} cutting moves\n{all}");
}

#[test]
fn the_gcode_of_the_passes_in_inches_has_no_alert_either() {
    let passes = [pass(0.2, 0.0)];
    let body = write_in(&passes, &Job::default(), &Order::default(), Unit::Inch).unwrap();
    assert!(body.starts_with("G20\n"), "{body}");
    let found = alerts(&body);
    assert!(found.is_empty(), "{found:?}\n{body}");
}

#[test]
fn the_command_line_starts_the_gcode_with_the_standard_header_and_the_program_has_no_alert() {
    let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
    let out = std::env::temp_dir().join(format!("eh3d_isocam_header_{}.nc", std::process::id()));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args([
            "isolate",
            sample,
            out.to_str().unwrap(),
            "--coarse-tool-mm",
            "0.4",
        ])
        .output()
        .expect("the program starts");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let hash = sha256_hex(&std::fs::read(sample).unwrap());
    let want = format!(
        "; generator: eh3d-isocam {}\n; units: mm (G21)\n; source: sample.gtl sha256:{hash}\n; tools: one tool per pass, see the comment of each pass\nG21\nG90\n",
        env!("CARGO_PKG_VERSION")
    );
    assert!(text.starts_with(&want), "{text}");
    let found = alerts(&text);
    assert!(found.is_empty(), "{found:?}");
}
