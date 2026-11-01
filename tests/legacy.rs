//! Old-style Gerber layers (no format statement, a separate list of apertures): the same board written both ways must give the same copper and the same paths.

use eh3d_gerberviewer::scene::from_gerber;
use eh3d_isocam::config::Settings;
use eh3d_isocam::isolate::plan;
use eh3d_isocam::legacy::{guess_format, has_format_statement, parse_apertures, Zeros};
use eh3d_isocam::units::Unit;
use std::process::Command;

// A small board written by hand: inches, 2.3 digits, leading zeros left out (a 100 mil pad is X100 in thousandths of an inch), two pads and a track.
const OLD: &str = "G90*\nG70*\nG54D10*\nX500Y500D02*\nX1500Y500D01*\nG54D11*\nX500Y500D03*\nX1500Y500D03*\nM02*\n";
const LIST: &str = "heading line\n   D10   10   0.25\n   D11   60   1.52\n";
const NEW: &str = "%FSLAX23Y23*%\n%MOIN*%\n%ADD10C,0.01*%\n%ADD11C,0.06*%\nD10*\nX500Y500D02*\nX1500Y500D01*\nD11*\nX500Y500D03*\nX1500Y500D03*\nM02*\n";

fn passes_of(text: &str) -> Vec<usize> {
    let scene = from_gerber(&eh3d_parser_gerber::parse(text));
    let mut s = Settings::default();
    s.set("res_mm", "0.02").unwrap();
    s.set("fine_tool_mm", "0.2").unwrap();
    let passes = plan(&scene, &s.plan().unwrap()).unwrap();
    passes.iter().map(|p| p.cuts().len()).collect()
}

#[test]
fn an_old_style_layer_reads_like_the_same_layer_in_extended_form() {
    let s = Settings::default();
    assert!(!has_format_statement(OLD));
    let text = s.gerber_text(OLD, Some(LIST)).unwrap();
    assert!(has_format_statement(&text));
    let (a, b) = (passes_of(&text), passes_of(NEW));
    assert!(!a.is_empty() && a.iter().all(|n| *n > 0));
    assert_eq!(a, b);
}

#[test]
fn the_sizes_of_the_apertures_follow_the_unit_and_the_column_of_the_list() {
    let mil = parse_apertures(LIST, Unit::Mil, 1).unwrap();
    assert!((mil[1].x_mm - 1.524).abs() < 1e-9);
    let mm = parse_apertures(LIST, Unit::Mm, 2).unwrap();
    assert!((mm[1].x_mm - 1.52).abs() < 1e-9);
}

#[test]
fn an_old_style_layer_without_its_list_is_refused_with_what_to_do() {
    let e = Settings::default().gerber_text(OLD, None).unwrap_err();
    assert!(e.contains("--apertures"), "{e}");
    // an extended layer needs no list
    assert_eq!(Settings::default().gerber_text(NEW, None).unwrap(), NEW);
}

#[test]
fn the_format_comes_from_the_settings() {
    let mut s = Settings::default();
    s.set("gerber_format", "2.4").unwrap();
    s.set("gerber_zeros", "trailing").unwrap();
    s.set("gerber_units", "mm").unwrap();
    let t = s
        .gerber_text("G54D10*\nX1Y1D02*\n", Some("D10 3\n"))
        .unwrap();
    assert!(t.starts_with("%FSTAX24Y24*%\n%MOMM*%\n"), "{t}");
    assert!(s.set("gerber_units", "mil").is_err());
    assert!(s.set("gerber_format", "9.9").is_err());
    assert!(s.set("gerber_zeros", "middle").is_err());
    assert!(s.set("aperture_size_column", "x").is_err());
}

#[test]
fn the_new_settings_are_written_and_read_back() {
    let mut s = Settings::default();
    s.set("gerber_format", "3.4").unwrap();
    s.set("gerber_zeros", "trailing").unwrap();
    s.set("gerber_units", "mm").unwrap();
    s.set("aperture_units", "inch").unwrap();
    s.set("aperture_size_column", "2").unwrap();
    let back = Settings::parse(&s.to_text()).unwrap();
    assert_eq!(back, s);
    assert_eq!(back.gerber_zeros, Zeros::Trailing);
}

#[test]
fn a_guess_of_the_format_is_a_hint_and_garbage_does_not_break_it() {
    assert!(guess_format(OLD).is_some());
    assert!(guess_format("").is_none());
    for t in ["\u{0}", "X", "XY", "X-", "D10*X99999999999999999999Y1D01*"] {
        let _ = guess_format(t);
    }
}

#[test]
fn the_command_line_reads_an_old_style_layer_with_its_list() {
    let dir = std::env::temp_dir().join(format!("isocam_legacy_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (g, l, o) = (
        dir.join("old.g11"),
        dir.join("old.ger"),
        dir.join("out.plt"),
    );
    std::fs::write(&g, OLD).unwrap();
    std::fs::write(&l, LIST).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args([
            "isolate",
            g.to_str().unwrap(),
            o.to_str().unwrap(),
            "--apertures",
            l.to_str().unwrap(),
            "--res-mm",
            "0.02",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(std::fs::read_to_string(&o).unwrap().contains("PD"));
    // without the list it is an error that says what to give
    let bad = Command::new(env!("CARGO_BIN_EXE_eh3d-isocam"))
        .args(["info", g.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("--apertures"));
    std::fs::remove_dir_all(&dir).ok();
}
