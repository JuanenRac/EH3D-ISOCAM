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

// Bug hunt (round 4): names and bytes that go to the server, the pen map and the range of a plot, the times and the depths of a job, and files that are damaged or huge.
mod hunt_round4 {
    use eh3d_isocam::config::Settings;
    use eh3d_isocam::drill::{read_drills, DrillOrder, Drills, ExcellonFormat};
    use eh3d_isocam::gcode::{duration_text, estimate_time, Job};
    use eh3d_isocam::hpgl::{
        check_range, check_range_at, parse_pen_map, pen_map_text, pens_for, set_pen_speed,
        MAX_PENS, MAX_UNITS,
    };
    use eh3d_isocam::server::{
        artifact_name, artifact_request, base64, check_key, is_server_key, media_type,
    };
    use eh3d_serverclient::json::Json;

    fn unbase64(text: &str) -> Vec<u8> {
        let value = |c: u8| -> u32 {
            match c {
                b'A'..=b'Z' => u32::from(c - b'A'),
                b'a'..=b'z' => u32::from(c - b'a') + 26,
                b'0'..=b'9' => u32::from(c - b'0') + 52,
                b'+' => 62,
                b'/' => 63,
                _ => panic!("not in the alphabet: {c}"),
            }
        };
        let mut out = Vec::new();
        for chunk in text.as_bytes().chunks(4) {
            let pad = chunk.iter().filter(|c| **c == b'=').count();
            let mut n = 0u32;
            for c in chunk.iter().filter(|c| **c != b'=') {
                n = (n << 6) | value(*c);
            }
            n <<= 6 * pad as u32;
            let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
            out.extend_from_slice(&bytes[..3 - pad]);
        }
        out
    }

    // ---- base64 and the names ----

    #[test]
    fn base64_gives_the_vectors_of_the_standard() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(plain.as_bytes()), coded, "{plain:?}");
        }
    }

    #[test]
    fn base64_of_every_length_has_the_right_size_and_reads_back() {
        let data: Vec<u8> = (0..=255u8).cycle().take(300).collect();
        for n in 0..=data.len() {
            let coded = base64(&data[..n]);
            assert_eq!(coded.len(), n.div_ceil(3) * 4, "{n}");
            assert_eq!(coded.len() % 4, 0);
            assert_eq!(unbase64(&coded), data[..n].to_vec(), "{n}");
        }
    }

    #[test]
    fn base64_of_bytes_that_are_all_ones_and_all_zeros_keeps_the_alphabet() {
        assert_eq!(base64(&[0xff, 0xff, 0xff]), "////");
        assert_eq!(base64(&[0, 0, 0]), "AAAA");
        assert_eq!(base64(&[0xfb, 0xef, 0xbe]), "++++");
    }

    #[test]
    fn an_artifact_name_is_the_last_part_of_a_path_of_either_slash() {
        assert_eq!(artifact_name("a/b/c.nc").unwrap(), "c.nc");
        assert_eq!(artifact_name("C:\\x\\y.nc").unwrap(), "y.nc");
        assert_eq!(artifact_name("mixed/dir\\file.plt").unwrap(), "file.plt");
        assert_eq!(artifact_name("  spaced.nc  ").unwrap(), "spaced.nc");
    }

    #[test]
    fn a_name_that_is_a_dot_or_climbs_a_folder_is_refused() {
        for bad in [".", "..", "a/..", "a\\..", "../..", " .. ", "/."] {
            assert!(artifact_name(bad).is_err(), "{bad:?}");
        }
        assert!(artifact_name("...").is_ok());
        assert!(artifact_name(".hidden").is_ok());
        assert!(artifact_name("a..b").is_ok());
    }

    #[test]
    fn a_name_that_hides_or_reverses_its_text_is_refused() {
        for bad in [
            "gcode\u{202e}txt.exe",
            "a\u{200b}b",
            "a\u{200f}b",
            "a\u{2066}b",
            "\u{feff}a",
            "a\u{0}b",
            "a\u{7f}b",
            "a\nb",
        ] {
            assert!(artifact_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_name_has_one_to_one_hundred_and_twenty_characters_and_the_count_is_of_characters() {
        assert!(artifact_name("").is_err());
        assert!(artifact_name("dir/").is_err());
        assert!(artifact_name("   ").is_err());
        assert!(artifact_name(&"x".repeat(120)).is_ok());
        assert!(artifact_name(&"x".repeat(121)).is_err());
        assert!(artifact_name(&"\u{e9}".repeat(120)).is_ok());
        assert!(artifact_name(&"\u{65e5}".repeat(121)).is_err());
    }

    #[test]
    fn the_media_type_comes_from_the_last_extension_in_any_case() {
        for (file, kind) in [
            ("a.NC", "text/x-gcode"),
            ("dir.d/a.tap", "text/x-gcode"),
            ("a.tar.nc", "text/x-gcode"),
            ("a.PLT", "application/vnd.hp-hpgl"),
            ("a.drl", "application/x-excellon"),
            ("C:\\x\\a.GTL", "application/x-gerber"),
            ("a.svg", "image/svg+xml"),
            ("a.csv", "text/csv"),
        ] {
            assert_eq!(media_type(file), kind, "{file}");
        }
    }

    #[test]
    fn a_file_with_no_known_extension_is_plain_bytes() {
        for file in [
            "",
            "noext",
            "dir.nc/noext",
            "a.",
            ".",
            ".nc.exe",
            "a.nc ",
            "a.n c",
        ] {
            assert_eq!(media_type(file), "application/octet-stream", "{file:?}");
        }
    }

    #[test]
    fn the_request_of_an_artifact_reads_back_whatever_the_name_and_the_values() {
        let name = "a\"b\\c\nd\u{1f600}";
        let params = [("k\"1", "v\\1"), ("tab\t", "line\r\nbreak\u{0}")];
        let body = artifact_request(name, "text/x-gcode", &[0, 1, 2, 255, 254], &params);
        let v = Json::parse(&body).unwrap();
        assert_eq!(v.get("name").and_then(Json::as_str), Some(name));
        assert_eq!(v.get("format").and_then(Json::as_str), Some("text/x-gcode"));
        let data = v.get("data_base64").and_then(Json::as_str).unwrap();
        assert_eq!(unbase64(data), vec![0, 1, 2, 255, 254]);
        let p = v.get("params").unwrap();
        assert_eq!(p.get("k\"1").and_then(Json::as_str), Some("v\\1"));
        assert_eq!(
            p.get("tab\t").and_then(Json::as_str),
            Some("line\r\nbreak\u{0}")
        );
        assert!(v.get("producer").is_some());
    }

    #[test]
    fn the_request_of_an_empty_artifact_is_still_json() {
        let v = Json::parse(&artifact_request("a.txt", "text/plain", &[], &[])).unwrap();
        assert_eq!(v.get("data_base64").and_then(Json::as_str), Some(""));
        assert!(v.get("params").is_some());
    }

    #[test]
    fn the_keys_of_the_link_are_told_in_any_case_and_a_value_out_of_range_is_refused() {
        assert!(is_server_key("SERVER_ADDRESS "));
        assert!(!is_server_key("server_address_x"));
        assert!(!is_server_key("feed"));
        assert!(check_key("server_timeout_s", "1").is_ok());
        assert!(check_key("server_timeout_s", "120").is_ok());
        for (key, value) in [
            ("server_timeout_s", "0"),
            ("server_timeout_s", "121"),
            ("server_timeout_s", "-1"),
            ("server_timeout_s", "1.5"),
            ("server_timeout_s", ""),
            ("server_retries", "6"),
            ("server_max_answer_mib", "257"),
            ("server_max_answer_mib", "0"),
            ("server_mdns", "maybe"),
            ("server_address", "a b"),
            ("server_token", "a\tb"),
        ] {
            assert!(check_key(key, value).is_err(), "{key} = {value:?}");
        }
        assert!(check_key("server_retries", "0").is_ok());
        assert!(check_key("server_retries", "5").is_ok());
    }

    // ---- the pen map and the range of a plot ----

    #[test]
    fn a_pen_map_is_read_and_written_back_the_same() {
        let map = parse_pen_map("1:2, 3:1").unwrap();
        assert_eq!(map, vec![(1, 2), (3, 1)]);
        assert_eq!(parse_pen_map(&pen_map_text(&map)).unwrap(), map);
        assert_eq!(parse_pen_map("").unwrap(), Vec::<(u8, u8)>::new());
        assert_eq!(parse_pen_map(" , ,").unwrap(), Vec::<(u8, u8)>::new());
        assert_eq!(parse_pen_map("1 : 2 ,, 4:3").unwrap(), vec![(1, 2), (4, 3)]);
    }

    #[test]
    fn a_pen_map_with_a_tool_or_a_pen_out_of_range_is_refused() {
        let last = MAX_PENS;
        assert!(parse_pen_map(&format!("255:{last}")).is_ok());
        let one_past = format!("1:{}", last + 1);
        for bad in [
            "0:1",
            "1:0",
            one_past.as_str(),
            "256:1",
            "-1:1",
            "1:-1",
            "1",
            ":",
            "1:",
            ":1",
            "a:b",
            "1:2:3",
            "1:2,1:3",
            "1.5:1",
        ] {
            assert!(parse_pen_map(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_pens_of_the_tools_are_free_and_never_two_tools_share_one() {
        let pens = pens_for(&[3, 1, 2, 3], &[(2, 1)]).unwrap();
        let mut tools: Vec<u8> = pens.iter().map(|p| p.0).collect();
        tools.sort_unstable();
        assert_eq!(tools, vec![1, 2, 3]);
        let mut used: Vec<u8> = pens.iter().map(|p| p.1).collect();
        used.sort_unstable();
        used.dedup();
        assert_eq!(used.len(), 3, "{pens:?}");
        assert!(pens.contains(&(2, 1)), "the mapped pen is kept: {pens:?}");
    }

    #[test]
    fn as_many_tools_as_pens_fit_and_one_more_is_refused_without_a_panic() {
        let tools: Vec<u8> = (1..=MAX_PENS as u8).collect();
        assert_eq!(pens_for(&tools, &[]).unwrap().len(), MAX_PENS);
        let more: Vec<u8> = (1..=MAX_PENS as u8 + 1).collect();
        assert!(pens_for(&more, &[]).is_err());
        let all_mapped: Vec<(u8, u8)> = (1..=MAX_PENS as u8).map(|p| (p, p)).collect();
        assert!(pens_for(&[200], &all_mapped).is_err());
        assert!(pens_for(&[], &[]).unwrap().is_empty());
    }

    #[test]
    fn the_pen_speed_is_off_at_zero_and_refused_outside_its_range() {
        assert_eq!(set_pen_speed("IN;\nSP1;\n", 0.0).unwrap(), "IN;\nSP1;\n");
        assert_eq!(set_pen_speed("IN;\nSP1;\n", -0.0).unwrap(), "IN;\nSP1;\n");
        for bad in [
            0.099,
            100.01,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert!(set_pen_speed("IN;\n", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_pen_speed_is_written_without_useless_zeros() {
        for (speed, word) in [
            (0.1, "VS0.1;"),
            (12.5, "VS12.5;"),
            (2.0, "VS2;"),
            (10.0, "VS10;"),
            (100.0, "VS100;"),
        ] {
            let plot = set_pen_speed("IN;\nSP1;\n", speed).unwrap();
            assert_eq!(plot, format!("IN;\n{word}\nSP1;\n"), "{speed}");
        }
    }

    #[test]
    fn the_range_of_a_plot_is_exact_at_the_largest_unit_and_for_both_signs() {
        assert!(check_range(&format!("PU{MAX_UNITS},0;")).is_ok());
        assert!(check_range(&format!("PD0,{MAX_UNITS};")).is_ok());
        assert!(check_range(&format!("PU-{MAX_UNITS},0;")).is_ok());
        assert!(check_range(&format!("PU{},0;", MAX_UNITS + 1)).is_err());
        assert!(check_range(&format!("PD0,-{};", MAX_UNITS + 1)).is_err());
    }

    #[test]
    fn a_coordinate_too_long_for_a_number_or_a_line_that_is_not_a_move_is_handled() {
        assert!(check_range("PU99999999999999999999999,0;").is_err());
        assert!(check_range("SP99999999;\nLB99999999;\n").is_ok());
        assert!(check_range("").is_ok());
        assert!(check_range("PU;\nPD;\n").is_ok());
    }

    #[test]
    fn the_range_of_a_plot_with_units_that_are_zero_or_not_numbers_does_not_panic() {
        for units in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let _ = check_range_at("PU40000,0;", units);
            let _ = check_range_at("PU1,0;", units);
        }
    }

    // ---- the times and the depths of a job ----

    #[test]
    fn a_time_is_told_in_seconds_minutes_and_hours_and_never_panics() {
        for (seconds, text) in [
            (0.0, "0 s"),
            (44.6, "45 s"),
            (59.4, "59 s"),
            (59.5, "1 min 0 s"),
            (147.0, "2 min 27 s"),
            (3599.0, "59 min 59 s"),
            (3600.0, "1 h 0 min"),
            (3719.0, "1 h 1 min"),
            (-5.0, "0 s"),
            (f64::NAN, "0 s"),
            (f64::NEG_INFINITY, "0 s"),
        ] {
            assert_eq!(duration_text(seconds), text, "{seconds}");
        }
        let _ = duration_text(f64::INFINITY);
        let _ = duration_text(1e300);
        let _ = duration_text(f64::MAX);
    }

    #[test]
    fn the_time_of_a_job_is_the_length_over_the_feed_and_the_total_is_the_sum() {
        let job = Job::default();
        let t = estimate_time(&job, 1000.0, job.feed_mm_min, 0.0, 0).unwrap();
        assert_eq!(t.cutting_s, 60.0);
        assert_eq!(t.plunging_s, 0.0);
        assert_eq!(t.rapid_s, 0.0);
        assert_eq!(t.total_s(), 60.0);
        let none = estimate_time(&job, 1000.0, 0.0, 0.0, 0).unwrap();
        assert_eq!(none.total_s(), 0.0);
    }

    #[test]
    fn the_time_of_a_job_is_refused_for_speeds_and_lengths_that_make_no_sense() {
        let job = Job::default();
        for rapid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(estimate_time(&job, rapid, 1.0, 1.0, 1).is_err(), "{rapid}");
        }
        for bad in [-0.1, f64::NAN, f64::INFINITY] {
            assert!(estimate_time(&job, 1000.0, bad, 1.0, 1).is_err(), "{bad}");
            assert!(estimate_time(&job, 1000.0, 1.0, bad, 1).is_err(), "{bad}");
        }
        let still = Job {
            feed_mm_min: 0.0,
            ..Job::default()
        };
        assert!(estimate_time(&still, 1000.0, 1.0, 1.0, 1).is_err());
        let no_plunge = Job {
            plunge_mm_min: f64::NAN,
            ..Job::default()
        };
        assert!(estimate_time(&no_plunge, 1000.0, 1.0, 1.0, 1).is_err());
    }

    #[test]
    fn a_job_with_no_step_or_a_step_that_is_not_a_number_goes_down_in_one_go() {
        for step in [0.0, -0.01, f64::NAN, 0.06, 5.0] {
            let job = Job {
                depth_mm: 0.06,
                depth_step_mm: step,
                ..Job::default()
            };
            assert_eq!(job.levels(), vec![0.06], "{step}");
        }
    }

    #[test]
    fn the_depths_of_a_job_in_steps_rise_and_the_last_is_the_whole_depth() {
        for (depth, step, count) in [
            (0.06, 0.02, 3),
            (0.06, 0.025, 3),
            (0.06, 0.03, 2),
            (2.0, 0.005, 400),
            (0.06, 0.059, 2),
        ] {
            let job = Job {
                depth_mm: depth,
                depth_step_mm: step,
                ..Job::default()
            };
            let levels = job.levels();
            assert_eq!(levels.len(), count, "{depth} {step}");
            assert_eq!(*levels.last().unwrap(), depth);
            assert!(levels.windows(2).all(|w| w[0] < w[1]), "{depth} {step}");
            assert!(levels[0] > 0.0);
        }
    }

    #[test]
    fn a_job_is_valid_at_the_edges_of_every_range_and_not_one_step_past() {
        let ok = |f: &dyn Fn(&mut Job)| {
            let mut j = Job::default();
            f(&mut j);
            j.validate().is_ok()
        };
        assert!(ok(&|j| j.depth_mm = 0.005));
        assert!(ok(&|j| j.depth_mm = 2.0));
        assert!(!ok(&|j| j.depth_mm = 0.004_999));
        assert!(!ok(&|j| j.depth_mm = 2.000_001));
        assert!(ok(&|j| j.depth_step_mm = 0.0));
        assert!(ok(&|j| j.depth_step_mm = 0.005));
        assert!(!ok(&|j| j.depth_step_mm = 0.004));
        assert!(!ok(&|j| j.depth_step_mm = f64::NAN));
        assert!(!ok(&|j| j.safe_z_mm = 0.4));
        assert!(!ok(&|j| j.tool_change_z_mm = 1.0));
        assert!(!ok(&|j| j.feed_mm_min = 0.5));
        assert!(!ok(&|j| j.spindle_rpm = 999.0));
        assert!(!ok(&|j| j.spindle_wait_s = -0.1));
    }

    #[test]
    fn a_job_with_a_number_that_is_not_a_number_in_any_field_is_not_valid() {
        let fields: [fn(&mut Job, f64); 8] = [
            |j, v| j.safe_z_mm = v,
            |j, v| j.tool_change_z_mm = v,
            |j, v| j.depth_mm = v,
            |j, v| j.feed_mm_min = v,
            |j, v| j.plunge_mm_min = v,
            |j, v| j.spindle_rpm = v,
            |j, v| j.spindle_wait_s = v,
            |j, v| j.depth_step_mm = v,
        ];
        for set in fields {
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut j = Job::default();
                set(&mut j, bad);
                assert!(j.validate().is_err(), "{bad}");
            }
        }
    }

    // ---- settings and files ----

    #[test]
    fn the_words_of_the_format_and_of_the_drill_order_read_back_their_own_names() {
        for f in [ExcellonFormat::Auto, ExcellonFormat::TwoFour] {
            assert_eq!(ExcellonFormat::parse(f.name()), Ok(f));
        }
        for o in [DrillOrder::File, DrillOrder::Nearest, DrillOrder::Optimized] {
            assert_eq!(DrillOrder::parse(o.name()), Ok(o));
        }
        assert_eq!(DrillOrder::parse(" Original "), Ok(DrillOrder::File));
        assert_eq!(DrillOrder::parse("OPTIMISED"), Ok(DrillOrder::Optimized));
        assert_eq!(ExcellonFormat::parse(" AUTO "), Ok(ExcellonFormat::Auto));
        for bad in ["", "2,4", "2.5", "auto auto", "nearest!", "fil"] {
            assert!(ExcellonFormat::parse(bad).is_err(), "{bad:?}");
            assert!(DrillOrder::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_settings_text_of_the_defaults_reads_back_and_writes_the_same_text() {
        let text = Settings::default().to_text();
        let back = Settings::parse(&text).unwrap();
        assert_eq!(back.to_text(), text);
        assert_eq!(back, Settings::default());
    }

    #[test]
    fn an_empty_settings_file_and_a_file_of_comments_are_the_defaults() {
        assert_eq!(Settings::parse("").unwrap(), Settings::default());
        assert_eq!(
            Settings::parse("# nothing\n\n   # here\n").unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn a_settings_line_that_is_wrong_names_its_line_and_numbers_that_are_not_numbers_are_refused() {
        let e = Settings::parse("# c\n\nnot a setting line").unwrap_err();
        assert!(e.contains("line 3"), "{e}");
        assert!(Settings::parse("no_such_key = 1").is_err());
        for bad in ["nan", "inf", "-inf", "1e999", "abc", ""] {
            assert!(
                Settings::parse(&format!("scale_x = {bad}")).is_err(),
                "{bad:?}"
            );
        }
    }

    fn sane(r: Result<Drills, String>) {
        if let Ok(d) = r {
            for h in &d.holes {
                assert!(h.x_mm.is_finite() && h.y_mm.is_finite(), "{h:?}");
                assert!(h.x_mm.abs() <= 5000.0 && h.y_mm.abs() <= 5000.0, "{h:?}");
                assert!(
                    h.diameter_mm.is_finite() && (0.0..=50.0).contains(&h.diameter_mm),
                    "{h:?}"
                );
            }
        }
    }

    #[test]
    fn a_drill_file_that_is_empty_or_cut_never_gives_holes_and_never_panics() {
        assert!(read_drills("").is_err());
        assert!(read_drills("M48\n").is_err());
        assert!(read_drills("M48\nMETRIC\nT1C0.8\n%\n").is_err());
        assert!(read_drills("\u{0}\u{0}\u{0}").is_err());
    }

    #[test]
    fn a_drill_file_with_huge_or_odd_numbers_gives_holes_inside_the_limits_or_an_error() {
        for text in [
            "M48\nMETRIC\nT1C0.8\n%\nT1\nX99999999999999999999Y1\nM30\n",
            "M48\nMETRIC\nT1C999\n%\nT1\nX1Y1\nM30\n",
            "M48\nMETRIC\nT1C-5\n%\nT1\nX1Y1\nM30\n",
            "M48\nMETRIC\nT1C0.8\n%\nT1\nX-99999Y-99999\nM30\n",
            "M48\nINCH\nT1C1e999\n%\nT1\nX1Y1\nM30\n",
            "M48\nMETRIC\nT1C0.8\n%\nT1\nXnanYinf\nM30\n",
            "M48\nMETRIC\nT1C0.8\n%\nT99999999999\nX1Y1\nM30\n",
            "M48\nMETRIC\nT1C0.8\n%\nT1\nX1.5.5Y--2\nM30\n",
        ] {
            sane(read_drills(text));
        }
    }
}
