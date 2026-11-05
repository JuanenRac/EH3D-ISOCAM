//! The format 2.4 of the coordinates of a drill file and of a routing file (`--excellon-format 2.4`, `excellon_format = 2.4`): two digits before the point and four after, with the zeros as the header says.

use eh3d_isocam::config::Settings;
use eh3d_isocam::drill::{
    read_drills, read_drills_with_format, write_drills_in_format, write_route_excellon,
    write_route_excellon_in_format, DrillStyle, ExcellonFormat, Hole,
};
use eh3d_isocam::order::Cut;
use std::process::Command;

const INCH_MM: f64 = 25.4;

fn tmp(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"))
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

fn hole(tool: u8, x_mm: f64, y_mm: f64, diameter_mm: f64) -> Hole {
    Hole {
        tool,
        x_mm,
        y_mm,
        diameter_mm,
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.002
}

fn two_four() -> DrillStyle {
    DrillStyle::default()
}

#[test]
fn the_format_is_auto_or_2_4_and_anything_else_is_refused_with_the_same_words() {
    assert_eq!(ExcellonFormat::default(), ExcellonFormat::Auto);
    assert_eq!(ExcellonFormat::parse("auto"), Ok(ExcellonFormat::Auto));
    assert_eq!(ExcellonFormat::parse(" 2.4 "), Ok(ExcellonFormat::TwoFour));
    assert_eq!(ExcellonFormat::parse("AUTO"), Ok(ExcellonFormat::Auto));
    for bad in ["3.3", "2:4", "24", "", "2.5", "mils"] {
        assert_eq!(
            ExcellonFormat::parse(bad),
            Err(format!("excellon_format '{bad}' is not auto or 2.4"))
        );
    }
    assert_eq!(ExcellonFormat::Auto.name(), "auto");
    assert_eq!(ExcellonFormat::TwoFour.name(), "2.4");
}

#[test]
fn the_setting_is_in_the_file_on_the_command_line_and_in_the_default_text() {
    assert_eq!(Settings::default().excellon_format, ExcellonFormat::Auto);
    assert!(Settings::default()
        .to_text()
        .contains("excellon_format = auto\n"));
    let s = Settings::parse("# holes\nexcellon_format = 2.4\n").unwrap();
    assert_eq!(s.excellon_format, ExcellonFormat::TwoFour);
    assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    let mut t = Settings::default();
    t.set("--excellon-format", "2.4").unwrap();
    assert_eq!(t.excellon_format, ExcellonFormat::TwoFour);
    let e = Settings::parse("fine_tool_mm = 0.2\nexcellon_format = 3.3\n").unwrap_err();
    assert_eq!(e, "line 2: excellon_format '3.3' is not auto or 2.4");
}

#[test]
fn the_plain_style_of_the_bungard_programs_cannot_be_in_2_4() {
    let e = Settings::parse("ncd_isocam_style = yes\nexcellon_format = 2.4\n").unwrap_err();
    assert!(
        e.contains("excellon_format 2.4") && e.contains("ncd_isocam_style"),
        "{e}"
    );
    let plain = DrillStyle {
        plain: true,
        digits: 0,
    };
    let e = write_drills_in_format(&[hole(1, 10.0, 10.0, 0.8)], &plain, ExcellonFormat::TwoFour)
        .unwrap_err();
    assert!(e.contains("cannot be written in the 2.4 format"), "{e}");
    assert!(
        write_drills_in_format(&[hole(1, 10.0, 10.0, 0.8)], &plain, ExcellonFormat::Auto).is_ok()
    );
}

#[test]
fn a_file_without_leading_zeros_is_read_as_two_digits_and_four() {
    // INCH,TZ: the leading zeros are left out, so X12340 is 1.2340 in and Y5000 is 0.5 in
    let text = "M48\nINCH,TZ\nT1C0.0315\n%\nT1\nX12340Y5000\nX012340Y005000\nM30\n";
    let d = read_drills_with_format(text, None, ExcellonFormat::TwoFour).unwrap();
    assert_eq!(d.holes.len(), 2);
    for h in &d.holes {
        assert!(
            near(h.x_mm, 1.234 * INCH_MM) && near(h.y_mm, 0.5 * INCH_MM),
            "{h:?}"
        );
        assert!((h.diameter_mm - 0.8).abs() < 0.01, "{h:?}");
    }
}

#[test]
fn a_file_without_trailing_zeros_is_read_with_the_zeros_put_back() {
    // INCH,LZ: the trailing zeros are left out, so X0123 is 012300 = 1.2300 in and Y05 is 050000 = 5 in
    let text = "M48\nINCH,LZ\nT1C0.0315\n%\nT1\nX0123Y05\nM30\n";
    let d = read_drills_with_format(text, None, ExcellonFormat::TwoFour).unwrap();
    assert_eq!(d.holes.len(), 1);
    assert!(near(d.holes[0].x_mm, 1.23 * INCH_MM), "{:?}", d.holes[0]);
    assert!(near(d.holes[0].y_mm, 5.0 * INCH_MM), "{:?}", d.holes[0]);
}

#[test]
fn a_negative_coordinate_is_read_and_a_metric_file_has_the_same_digits() {
    let text = "M48\nINCH,TZ\nT1C0.0315\n%\nT1\nX-005000Y010000\nM30\n";
    let d = read_drills_with_format(text, None, ExcellonFormat::TwoFour).unwrap();
    assert!(near(d.holes[0].x_mm, -0.5 * INCH_MM) && near(d.holes[0].y_mm, INCH_MM));
    // in millimetres the digits are the same and the unit is the file's: X012340 is 1.2340 mm
    let metric = "M48\nMETRIC,TZ\nT1C0.8\n%\nT1\nX012340Y005000\nM30\n";
    let d = read_drills_with_format(metric, None, ExcellonFormat::TwoFour).unwrap();
    assert!(near(d.holes[0].x_mm, 1.234) && near(d.holes[0].y_mm, 0.5));
}

#[test]
fn a_file_that_states_its_own_format_keeps_it() {
    // the digits after INCH,TZ say 3.3: they win over the option, which is only the format of a file that says nothing
    let own = "M48\nINCH,TZ,000.000\nT1C0.0315\n%\nT1\nX1234Y5000\nM30\n";
    for format in [ExcellonFormat::Auto, ExcellonFormat::TwoFour] {
        let d = read_drills_with_format(own, None, format).unwrap();
        assert!(
            near(d.holes[0].x_mm, 1.234 * INCH_MM) && near(d.holes[0].y_mm, 5.0 * INCH_MM),
            "{format:?}: {:?}",
            d.holes[0]
        );
    }
    // a FILE_FORMAT comment is read as 2.4 even with the automatic format, and never as whole mils
    let comment = "M48\nINCH,TZ\n; FILE_FORMAT=2:4\nT1C0.0315\n%\nT1\nX012340Y005000\nM30\n";
    let d = read_drills(comment).unwrap();
    assert!(near(d.holes[0].x_mm, 1.234 * INCH_MM) && near(d.holes[0].y_mm, 0.5 * INCH_MM));
}

#[test]
fn the_holes_are_written_in_six_digits_and_read_back() {
    let holes = vec![
        hole(1, 10.16, 25.4, 0.8),
        hole(1, 50.0, 2.54, 0.8),
        hole(2, -12.7, 100.0, 1.0),
        hole(2, 0.0, 0.0, 1.0),
    ];
    let text = write_drills_in_format(&holes, &two_four(), ExcellonFormat::TwoFour).unwrap();
    assert!(
        text.starts_with("M48\nINCH,TZ\n; FILE_FORMAT=2:4\n"),
        "{text}"
    );
    assert!(
        text.contains("T1C0.0315\n") && text.contains("T2C0.0394\n"),
        "{text}"
    );
    for line in [
        "X004000Y010000\n",
        "X019685Y001000\n",
        "X-005000Y039370\n",
        "X000000Y000000\n",
    ] {
        assert!(text.contains(line), "{line:?} in {text}");
    }
    assert!(text.ends_with("M30\n"), "{text}");
    // the file states its format, so it reads back the same with either setting
    for format in [ExcellonFormat::Auto, ExcellonFormat::TwoFour] {
        let back = read_drills_with_format(&text, None, format).unwrap();
        assert_eq!(back.holes.len(), holes.len());
        for (a, b) in holes.iter().zip(&back.holes) {
            assert_eq!(a.tool, b.tool);
            assert!(near(a.x_mm, b.x_mm) && near(a.y_mm, b.y_mm), "{a:?} {b:?}");
            assert!((a.diameter_mm - b.diameter_mm).abs() < 0.01, "{a:?} {b:?}");
        }
    }
}

#[test]
fn the_automatic_format_still_writes_whole_mils() {
    let holes = vec![hole(1, 25.4, 25.4, 0.8)];
    let text = write_drills_in_format(&holes, &two_four(), ExcellonFormat::Auto).unwrap();
    assert!(
        text.contains("X1000Y1000\n") && text.contains("whole mils"),
        "{text}"
    );
    assert!(!text.contains("FILE_FORMAT"), "{text}");
}

#[test]
fn a_position_that_does_not_fit_two_digits_is_refused_when_writing() {
    // 99.9999 in is the largest: 2539.99746 mm
    let ok = write_drills_in_format(
        &[hole(1, 2539.99, 0.0, 0.8)],
        &two_four(),
        ExcellonFormat::TwoFour,
    );
    assert!(ok.is_ok());
    for mm in [2540.0, -2540.0, 4000.0] {
        let e = write_drills_in_format(
            &[hole(1, mm, 0.0, 0.8)],
            &two_four(),
            ExcellonFormat::TwoFour,
        )
        .unwrap_err();
        assert!(
            e.contains("does not fit the 2.4 format") && e.contains("99.9999 in"),
            "{mm}: {e}"
        );
    }
    let e = write_drills_in_format(
        &[hole(1, f64::NAN, 0.0, 0.8)],
        &two_four(),
        ExcellonFormat::TwoFour,
    )
    .unwrap_err();
    assert!(e.contains("out of range"), "{e}");
}

#[test]
fn a_coordinate_with_too_many_digits_is_an_error_at_the_line_of_the_file() {
    // seven digits do not fit 2.4; the line is the one of the file, not of the text the reader is given
    let text = "M48\nINCH,TZ\nT1C0.0315\n%\nT1\nX1234567Y1\nM30\n";
    let e = read_drills_with_format(text, None, ExcellonFormat::TwoFour).unwrap_err();
    assert!(e.starts_with("line 6:"), "{e}");
}

#[test]
fn the_command_line_reads_and_writes_a_drill_file_in_2_4() {
    let input = tmp("excellon_24_in.ncd");
    let out = tmp("excellon_24_out.ncd");
    // INCH,TZ with six digits and no format of its own: the whole-mil reading would make it ten times too large
    std::fs::write(
        &input,
        "M48\nINCH,TZ\nT1C0.0315\n%\nT1\nX012340Y005000\nX040000Y010000\nM30\n",
    )
    .unwrap();
    let (ok, text, err) = run(&["drills", &input, &out, "--excellon-format", "2.4"]);
    assert!(ok, "{err}");
    assert!(text.contains("2 holes"), "{text}");
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(written.contains("; FILE_FORMAT=2:4\n"), "{written}");
    assert!(
        written.contains("X012340Y005000\n") && written.contains("X040000Y010000\n"),
        "{written}"
    );
    let back = read_drills(&written).unwrap();
    assert!(near(back.holes[0].x_mm, 1.234 * INCH_MM) && near(back.holes[1].x_mm, 4.0 * INCH_MM));
    // the same through a configuration file
    let cfg = tmp("excellon_24.cfg");
    std::fs::write(&cfg, "excellon_format = 2.4\n").unwrap();
    let out2 = tmp("excellon_24_out2.ncd");
    let (ok, _, err) = run(&["drills", &input, &out2, "--config", &cfg]);
    assert!(ok, "{err}");
    assert_eq!(std::fs::read_to_string(&out2).unwrap(), written);
}

#[test]
fn the_command_line_refuses_a_format_it_does_not_know_and_the_plain_style_with_2_4() {
    let input = tmp("excellon_24_bad_in.ncd");
    let out = tmp("excellon_24_bad_out.ncd");
    std::fs::write(
        &input,
        "M48\nINCH,TZ\nT1C0.0315\n%\nT1\nX012340Y005000\nM30\n",
    )
    .unwrap();
    let (ok, _, err) = run(&["drills", &input, &out, "--excellon-format", "3.3"]);
    assert!(
        !ok && err.contains("excellon_format '3.3' is not auto or 2.4"),
        "{err}"
    );
    let (ok, _, err) = run(&[
        "drills",
        &input,
        &out,
        "--excellon-format",
        "2.4",
        "--ncd-isocam-style",
        "yes",
    ]);
    assert!(!ok && err.contains("ncd_isocam_style"), "{err}");
    let (ok, text, err) = run(&["config"]);
    assert!(ok && text.contains("excellon_format = auto"), "{err}");
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64, closed: bool) -> Cut {
    Cut {
        pts: vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
        closed,
    }
}

/// The points of the routed paths of a file, in millimetres, read with the reader of the suite.
fn route_points_mm(text: &str) -> Vec<Vec<(f64, f64)>> {
    let f = eh3d_parser_excellon::parse(text);
    let units = f.units.expect("the routing file says its units");
    f.routes
        .iter()
        .map(|r| {
            r.points
                .iter()
                .map(|p| (units.to_mm(p.x), units.to_mm(p.y)))
                .collect()
        })
        .collect()
}

const OUTLINE_50_BY_30: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";

#[test]
fn the_routing_file_is_written_in_six_digits_with_the_same_head_as_the_drills() {
    let cuts = [
        square(0.0, 0.0, 25.4, 12.7, true),
        square(-12.7, 5.08, 2.54, 10.16, false),
    ];
    let text = write_route_excellon_in_format(&cuts, 1.0, 2, ExcellonFormat::TwoFour).unwrap();
    assert!(
        text.starts_with("M48\nINCH,TZ\n; FILE_FORMAT=2:4\n"),
        "{text}"
    );
    assert!(text.contains("T2C0.0394\n%\nT2\n"), "{text}");
    for line in [
        "G00X000000Y000000\nM15\n",
        "G01X010000Y000000\n",
        "G01X010000Y005000\n",
        "G00X-005000Y002000\nM15\n",
        "G01X001000Y004000\n",
    ] {
        assert!(text.contains(line), "{line:?} in {text}");
    }
    assert_eq!(text.matches("M15").count(), 2);
    assert_eq!(text.matches("M16").count(), 2);
    assert!(text.ends_with("M16\nM30\n"), "{text}");
    // the drill files and the routing file share the lines that announce the format
    let drills = write_drills_in_format(
        &[hole(1, 10.0, 10.0, 0.8)],
        &two_four(),
        ExcellonFormat::TwoFour,
    )
    .unwrap();
    let head = |t: &str| t.lines().take(4).collect::<Vec<_>>().join("\n");
    assert_eq!(head(&text), head(&drills));
}

#[test]
fn the_routing_file_in_2_4_is_read_back_with_negative_positions() {
    let cuts = [
        square(-12.7, -5.08, 30.48, 20.32, true),
        square(1.27, 2.54, 3.81, 5.08, false),
    ];
    let text = write_route_excellon_in_format(&cuts, 1.0, 1, ExcellonFormat::TwoFour).unwrap();
    let paths = route_points_mm(&text);
    assert_eq!(paths.len(), 2, "{text}");
    for (path, cut) in paths.iter().zip(&cuts) {
        let mut want = cut.pts.clone();
        if cut.closed {
            want.push(cut.pts[0]);
        }
        assert_eq!(path.len(), want.len(), "{text}");
        for (a, b) in path.iter().zip(&want) {
            assert!(near(a.0, b.0) && near(a.1, b.1), "{a:?} {b:?}");
        }
    }
}

#[test]
fn the_routing_file_in_whole_mils_is_what_it_was() {
    let cuts = [square(0.0, 0.0, 25.4, 12.7, true)];
    let auto = write_route_excellon_in_format(&cuts, 1.0, 1, ExcellonFormat::Auto).unwrap();
    assert_eq!(auto, write_route_excellon(&cuts, 1.0, 1).unwrap());
    assert!(
        auto.starts_with("M48\nINCH\nT1C0.0394\n%\nT1\nG00X0Y0\nM15\n"),
        "{auto}"
    );
    assert!(
        auto.contains("G01X1000Y0\n") && !auto.contains("FILE_FORMAT"),
        "{auto}"
    );
}

#[test]
fn a_cut_that_does_not_fit_two_digits_is_refused_in_the_routing_file_too() {
    let fits = [square(0.0, 0.0, 2539.99, 1.0, false)];
    assert!(write_route_excellon_in_format(&fits, 1.0, 1, ExcellonFormat::TwoFour).is_ok());
    for mm in [2540.0, -2540.0] {
        let cuts = [square(0.0, 0.0, mm, 1.0, false)];
        let e = write_route_excellon_in_format(&cuts, 1.0, 1, ExcellonFormat::TwoFour).unwrap_err();
        assert!(
            e.contains("does not fit the 2.4 format") && e.contains("99.9999 in"),
            "{mm}: {e}"
        );
        // whole mils hold it
        assert!(write_route_excellon_in_format(&cuts, 1.0, 1, ExcellonFormat::Auto).is_ok());
    }
    let nan = [square(0.0, 0.0, f64::NAN, 1.0, false)];
    let e = write_route_excellon_in_format(&nan, 1.0, 1, ExcellonFormat::TwoFour).unwrap_err();
    assert!(e.contains("out of range"), "{e}");
    // the checks of the tool and of the path are the same in both formats
    let one = [square(0.0, 0.0, 1.0, 1.0, true)];
    assert!(write_route_excellon_in_format(&[], 1.0, 1, ExcellonFormat::TwoFour).is_err());
    assert!(write_route_excellon_in_format(&one, 0.0, 1, ExcellonFormat::TwoFour).is_err());
    assert!(write_route_excellon_in_format(&one, 1.0, 0, ExcellonFormat::TwoFour).is_err());
}

#[test]
fn the_command_line_routes_in_2_4_with_the_option_and_with_a_configuration_file() {
    let outline = tmp("excellon_24_route.gko");
    std::fs::write(&outline, OUTLINE_50_BY_30).unwrap();
    let out = tmp("excellon_24_route.ncd");
    let (ok, text, err) = run(&[
        "route",
        &outline,
        &out,
        "--route-tool-mm",
        "2",
        "--excellon-format",
        "2.4",
    ]);
    assert!(ok, "{err}");
    assert!(text.contains("1 paths"), "{text}");
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(
        written.starts_with("M48\nINCH,TZ\n; FILE_FORMAT=2:4\n"),
        "{written}"
    );
    // every position is six digits, with a sign when it is negative (the cut goes round the board, outside it)
    let moves: Vec<&str> = written
        .lines()
        .filter(|l| l.starts_with("G00") || l.starts_with("G01"))
        .collect();
    assert!(moves.len() >= 5, "{written}");
    for l in &moves {
        let (x, y) = l[4..].split_once('Y').expect("an X and a Y");
        for v in [x, y] {
            let digits = v.strip_prefix('-').unwrap_or(v);
            assert!(
                digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_digit()),
                "{l}"
            );
        }
    }
    let paths = route_points_mm(&written);
    assert_eq!(paths.len(), 1, "{written}");
    let lo = paths[0].iter().fold(f64::MAX, |m, p| m.min(p.0));
    let hi = paths[0].iter().fold(f64::MIN, |m, p| m.max(p.0));
    assert!(
        (lo + 1.0).abs() < 0.5 && (hi - 51.0).abs() < 0.5,
        "{lo} {hi}"
    );
    let cfg = tmp("excellon_24_route.cfg");
    std::fs::write(&cfg, "excellon_format = 2.4\nroute_tool_mm = 2\n").unwrap();
    let out2 = tmp("excellon_24_route2.ncd");
    let (ok, _, err) = run(&["route", &outline, &out2, "--config", &cfg]);
    assert!(ok, "{err}");
    assert_eq!(std::fs::read_to_string(&out2).unwrap(), written);
    // without the option the file is in whole mils as before
    let out3 = tmp("excellon_24_route3.ncd");
    let (ok, _, err) = run(&["route", &outline, &out3, "--route-tool-mm", "2"]);
    assert!(ok, "{err}");
    let plain = std::fs::read_to_string(&out3).unwrap();
    assert!(
        plain.starts_with("M48\nINCH\nT1C0.0787\n") && !plain.contains("FILE_FORMAT"),
        "{plain}"
    );
}

#[test]
fn the_command_line_refuses_the_plain_style_with_2_4_and_an_unknown_format_when_routing() {
    let outline = tmp("excellon_24_route_bad.gko");
    std::fs::write(&outline, OUTLINE_50_BY_30).unwrap();
    let out = tmp("excellon_24_route_bad.ncd");
    let (ok, _, err) = run(&[
        "route",
        &outline,
        &out,
        "--excellon-format",
        "2.4",
        "--ncd-isocam-style",
        "yes",
    ]);
    assert!(!ok && err.contains("ncd_isocam_style"), "{err}");
    let (ok, _, err) = run(&["route", &outline, &out, "--excellon-format", "3.3"]);
    assert!(
        !ok && err.contains("excellon_format '3.3' is not auto or 2.4"),
        "{err}"
    );
}
