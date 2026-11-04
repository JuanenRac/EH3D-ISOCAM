//! More tests: how the kind of a file is told from its text, at the limits (nothing, mixed, nearly one thing and another), and the messages that say what to do.

use eh3d_isocam::filetype::{detect, expect_drills, expect_gerber, FileKind};

const GERBER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
const OLD_GERBER: &str = "G04 old style*\nD10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
const EXCELLON: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n";
const HPGL: &str = "IN;SP1;PU0,0;PD100,0,100,100;PU;SP0;\n";
const DXF: &str =
    "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n10\n21\n10\n0\nENDSEC\n0\nEOF\n";

#[test]
fn each_kind_of_file_is_told_from_a_normal_example() {
    assert_eq!(detect(GERBER), FileKind::Gerber);
    assert_eq!(detect(OLD_GERBER), FileKind::GerberOldStyle);
    assert_eq!(detect(EXCELLON), FileKind::Excellon);
    assert_eq!(detect(HPGL), FileKind::Hpgl);
    assert_eq!(detect(DXF), FileKind::Dxf);
}

#[test]
fn nothing_and_text_that_is_not_a_cam_file_are_unknown() {
    for text in [
        "",
        "\n\n  \n",
        "hello world",
        "G21 G90\nG0 X0 Y0\n",
        "a,b,c\n1,2,3\n",
        "{\"json\": true}",
        "\u{feff}",
    ] {
        assert_eq!(detect(text), FileKind::Unknown, "{text:?}");
    }
}

#[test]
fn the_line_ends_and_the_blanks_around_the_lines_do_not_change_the_kind() {
    for (text, kind) in [
        (GERBER, FileKind::Gerber),
        (EXCELLON, FileKind::Excellon),
        (DXF, FileKind::Dxf),
        (HPGL, FileKind::Hpgl),
    ] {
        let crlf = text.replace('\n', "\r\n");
        assert_eq!(detect(&crlf), kind, "crlf {kind:?}");
        let padded: String = text.lines().map(|l| format!("  {l}  \n")).collect();
        assert_eq!(detect(&padded), kind, "padded {kind:?}");
    }
}

#[test]
fn a_dxf_needs_its_section_and_its_end_and_a_comment_start_is_allowed() {
    // without the end of file it is not a drawing that can be read
    assert_ne!(
        detect("0\nSECTION\n2\nENTITIES\n0\nENDSEC\n"),
        FileKind::Dxf
    );
    // the section is in any case
    assert_eq!(
        detect("0\nsection\n2\nENTITIES\n0\nendsec\n0\neof\n"),
        FileKind::Dxf
    );
    // a comment first (group 999)
    assert_eq!(
        detect("999\nmade by a program\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n"),
        FileKind::Dxf
    );
    // blank lines before the first pair
    assert_eq!(detect(&format!("\n\n{DXF}")), FileKind::Dxf);
}

#[test]
fn a_gerber_with_a_format_statement_is_a_gerber_even_when_it_looks_like_other_things() {
    assert_eq!(detect("%FSLAX24Y24*%\nM48\n"), FileKind::Gerber);
    assert_eq!(detect("%FSLAX24Y24*%\n"), FileKind::Gerber);
    assert_eq!(detect("%FSTAX24Y24*%\nIN;PU0,0;PD1,1;\n"), FileKind::Gerber);
}

#[test]
fn an_old_style_gerber_needs_blocks_that_end_in_a_star_and_draw_with_a_position() {
    // a star and a D code but no position: not enough
    assert_eq!(detect("D10*\nM02*\n"), FileKind::Unknown);
    // one block that draws is enough when there are two stars
    assert_eq!(detect("X10Y10D03*\nM02*\n"), FileKind::GerberOldStyle);
    // a single star is not a Gerber
    assert_eq!(detect("X10Y10D03*\n"), FileKind::Unknown);
}

#[test]
fn an_hpgl_needs_two_drawing_instructions_and_an_init_or_a_pen_or_four_of_them() {
    assert_eq!(detect("IN;PU0,0;PD1,1;"), FileKind::Hpgl);
    assert_eq!(detect("SP1;PU0,0;PD1,1;"), FileKind::Hpgl);
    // a single drawing instruction is not a plot
    assert_eq!(detect("IN;PU0,0;"), FileKind::Unknown);
    // without IN or SP, four moves make a plot and three do not
    assert_eq!(detect("PU0,0;PD1,1;PD2,2;PD3,3;"), FileKind::Hpgl);
    assert_eq!(detect("PU0,0;PD1,1;PD2,2;"), FileKind::Unknown);
    // one `*` in it and it is not read as a plot
    assert_ne!(detect("IN;PU0,0;PD1,1;*"), FileKind::Hpgl);
    // instructions that are not of the set of a plot are not counted
    assert_eq!(detect("XX;YY;PU0,0;"), FileKind::Unknown);
}

#[test]
fn an_excellon_is_told_by_its_header_or_its_tools_and_holes_and_not_by_a_lone_word() {
    assert_eq!(detect("M48\n"), FileKind::Excellon);
    assert_eq!(detect("T01C0.8\nT01\nX1Y1\nM30\n"), FileKind::Excellon);
    assert_eq!(detect("T1C0.800F200S65\nX1Y1\n"), FileKind::Excellon);
    // a header with no `M48` but a `%` and tools by number alone
    assert_eq!(
        detect("%\nT1\nX1.0Y1.0\nT2\nX2.0Y2.0\n"),
        FileKind::Excellon
    );
    assert_eq!(detect("M30\n"), FileKind::Unknown);
    assert_eq!(detect("T1\n"), FileKind::Unknown);
    assert_eq!(detect("T123C0.8\nX1Y1\n"), FileKind::Unknown);
}

#[test]
fn the_names_of_the_kinds_are_all_different_and_say_what_they_are() {
    let all = [
        FileKind::Gerber,
        FileKind::GerberOldStyle,
        FileKind::Excellon,
        FileKind::Hpgl,
        FileKind::Dxf,
        FileKind::Unknown,
    ];
    let mut names: Vec<&str> = all.iter().map(|k| k.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), all.len());
    assert!(FileKind::Excellon.name().contains("drill"));
    assert!(FileKind::Hpgl.name().contains("HP-GL"));
    assert!(FileKind::Dxf.name().contains("DXF"));
    assert!(all.iter().filter(|k| k.is_gerber()).count() == 2);
}

#[test]
fn a_file_that_is_not_a_gerber_layer_is_refused_with_what_to_do_and_the_rest_is_let_through() {
    for kind in [FileKind::Excellon, FileKind::Hpgl, FileKind::Dxf] {
        let e = expect_gerber("board.x", kind).unwrap_err();
        assert!(e.starts_with("board.x is "), "{e}");
        assert!(e.contains("not a Gerber layer"), "{e}");
    }
    assert!(expect_gerber("a", FileKind::Excellon)
        .unwrap_err()
        .contains("--drills"));
    assert!(expect_gerber("a", FileKind::Hpgl)
        .unwrap_err()
        .contains("--outline"));
    for kind in [
        FileKind::Gerber,
        FileKind::GerberOldStyle,
        FileKind::Unknown,
    ] {
        assert!(expect_gerber("a", kind).is_ok(), "{kind:?}");
    }
}

#[test]
fn a_file_that_is_not_a_drill_file_is_refused_and_the_rest_is_let_through() {
    for kind in [
        FileKind::Gerber,
        FileKind::GerberOldStyle,
        FileKind::Hpgl,
        FileKind::Dxf,
    ] {
        let e = expect_drills("holes.x", kind).unwrap_err();
        assert!(
            e.starts_with("holes.x is ") && e.contains("not a drill file"),
            "{e}"
        );
    }
    for kind in [FileKind::Excellon, FileKind::Unknown] {
        assert!(expect_drills("a", kind).is_ok(), "{kind:?}");
    }
}

#[test]
fn the_name_in_the_message_is_the_one_that_was_given_even_with_odd_characters() {
    let e = expect_gerber("my \"board\" é.drl", FileKind::Excellon).unwrap_err();
    assert!(e.contains("my \"board\" é.drl"));
}

#[test]
fn a_path_with_a_coordinate_that_is_not_a_number_writes_nothing() {
    use eh3d_isocam::gcode::{write_in, Job};
    use eh3d_isocam::isolate::Pass;
    use eh3d_isocam::order::Order;
    use eh3d_isocam::units::Unit;
    let square = |bad: f64| Pass {
        paths: vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, bad), (0.0, 0.0)]],
        ..Pass::default()
    };
    let ok = write_in(&[square(1.0)], &Job::default(), &Order::default(), Unit::Mm).unwrap();
    assert!(ok.contains("G1 X1 Y1"), "{ok}");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err =
            write_in(&[square(bad)], &Job::default(), &Order::default(), Unit::Mm).unwrap_err();
        assert!(err.contains("not a number"), "{err}");
    }
}

#[test]
fn a_plot_with_a_coordinate_beyond_the_range_of_the_format_is_refused() {
    use eh3d_isocam::hpgl::{check_range, write};
    use eh3d_isocam::isolate::Pass;
    let pass = |far: f64| Pass {
        paths: vec![vec![(0.0, 0.0), (far, 0.0), (far, 10.0), (0.0, 0.0)]],
        ..Pass::default()
    };
    // 800 mm are 32000 units: inside; 820 mm are 32800 units: outside
    assert!(write(&[pass(800.0)]).is_ok());
    let err = write(&[pass(820.0)]).unwrap_err();
    assert!(err.contains("32767") && err.contains("too large"), "{err}");
    assert!(check_range("IN;SP1;PU0,0;PD100,-200;PU0,0;SP0;").is_ok());
    assert!(check_range("PU0,0;PD40000,0;").is_err());
    assert!(check_range("PU0,0;PD-40000,0;").is_err());
}
