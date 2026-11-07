//! The tools of the crib of EH3D-TOOL-MANAGER as the tools of a job (`--crib FILE --crib-fine N`): the table is read, the width a V-bit cuts with is worked out and the settings take it.

use eh3d_isocam::config::Settings;
use eh3d_isocam::crib::{apply, cutting_width_mm, parse_csv, CribTool, Kind};
use std::process::Command;

const HEAD: &str = "id,name,kind,diameter_mm,flutes,length_mm,tip_angle_deg,shank_mm,material,hours,meters,holes,max_hours,max_holes";
const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
const CRIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/crib.csv");

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

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn vbit(angle: f64, diameter: f64) -> CribTool {
    CribTool {
        id: 7,
        name: "V".into(),
        kind: Kind::VBit,
        diameter_mm: diameter,
        tip_angle_deg: Some(angle),
    }
}

fn table(rows: &[&str]) -> String {
    let mut text = String::from(HEAD);
    for r in rows {
        text.push('\n');
        text.push_str(r);
    }
    text.push('\n');
    text
}

// ---- reading the table -----------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_sample_table_of_the_repository_is_read() {
    let tools = parse_csv(&std::fs::read_to_string(CRIB).unwrap()).unwrap();
    assert_eq!(tools.len(), 4);
    assert_eq!((tools[0].id, tools[0].kind), (1, Kind::EndMill));
    assert!(near(tools[0].diameter_mm, 0.2) && tools[0].tip_angle_deg.is_none());
    assert_eq!(
        (tools[1].kind, tools[1].tip_angle_deg),
        (Kind::VBit, Some(30.0))
    );
    assert_eq!(tools[3].kind, Kind::Drill);
    assert_eq!(tools[3].name, "Drill 0.8 mm");
}

#[test]
fn the_columns_may_come_in_any_order_and_the_others_are_ignored() {
    let text = "tip_angle_deg,wear,kind,diameter_mm,name,id\n,9,end-mill,0.4,Mill,3\n45,1,engraver,0.6,\"Engraver, 45\",12\n";
    let tools = parse_csv(text).unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!((tools[0].id, tools[0].name.as_str()), (3, "Mill"));
    assert_eq!(tools[1].id, 12);
    assert_eq!(tools[1].name, "Engraver, 45");
    assert_eq!(tools[1].tip_angle_deg, Some(45.0));
}

#[test]
fn a_byte_order_mark_windows_line_ends_and_a_quoted_quote_are_read() {
    let text = format!(
        "\u{feff}{HEAD}\r\n1,\"Mill \"\"A\"\"\",end-mill,0.5,2,3,,3.175,carbide,0,0,0,0,0\r\n"
    );
    let tools = parse_csv(&text).unwrap();
    assert_eq!(tools[0].name, "Mill \"A\"");
}

#[test]
fn a_name_that_the_crib_program_protected_from_a_spreadsheet_is_given_back() {
    let text = table(&["1,'=Mill,end-mill,0.5,2,3,,3.175,carbide,0,0,0,0,0"]);
    assert_eq!(parse_csv(&text).unwrap()[0].name, "=Mill");
    let plain = table(&["1,'Mill,end-mill,0.5,2,3,,3.175,carbide,0,0,0,0,0"]);
    assert_eq!(parse_csv(&plain).unwrap()[0].name, "'Mill");
}

#[test]
fn a_bad_table_is_refused_with_the_row_and_the_reason() {
    let cases: [(String, &str); 12] = [
        (String::new(), "empty"),
        (
            "id,name,kind,diameter_mm\n".into(),
            "no column 'tip_angle_deg'",
        ),
        (
            table(&["0,A,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0"]),
            "row 2: the id '0'",
        ),
        (
            table(&["x,A,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0"]),
            "row 2: the id 'x'",
        ),
        (
            table(&[
                "1,A,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0",
                "1,B,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0",
            ]),
            "row 3: the id 1 is used twice",
        ),
        (
            table(&["1,A,saw,0.5,2,3,,3.175,c,0,0,0,0,0"]),
            "row 2: 'saw' is not a kind",
        ),
        (
            table(&["1,A,end-mill,0,2,3,,3.175,c,0,0,0,0,0"]),
            "diameter_mm '0'",
        ),
        (
            table(&["1,A,end-mill,51,2,3,,3.175,c,0,0,0,0,0"]),
            "diameter_mm '51'",
        ),
        (
            table(&["1,A,end-mill,NaN,2,3,,3.175,c,0,0,0,0,0"]),
            "diameter_mm 'NaN'",
        ),
        (
            table(&["1,A,v-bit,0.5,2,3,,3.175,c,0,0,0,0,0"]),
            "a v-bit needs its tip_angle_deg",
        ),
        (
            table(&["1,A,v-bit,0.5,2,3,9,3.175,c,0,0,0,0,0"]),
            "tip_angle_deg '9'",
        ),
        (
            table(&["1,A,end-mill,0.5"]),
            "row 2: it has 4 fields, not 14",
        ),
    ];
    for (text, part) in cases {
        let e = parse_csv(&text).unwrap_err();
        assert!(e.contains(part), "{part:?} not in {e:?}");
    }
}

#[test]
fn quotes_that_do_not_close_are_errors() {
    let open = table(&["1,\"A,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0"]);
    assert!(parse_csv(&open).unwrap_err().contains("never closed"));
    let after = table(&["1,\"A\"x,end-mill,0.5,2,3,,3.175,c,0,0,0,0,0"]);
    assert!(parse_csv(&after)
        .unwrap_err()
        .contains("after the closing quote"));
}

// ---- the width of the cut ----------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_width_of_a_v_bit_is_worked_out_and_cut_down_to_a_ten_thousandth() {
    // values worked out by hand: 2 * depth * tan(angle / 2) + flat, then floored to 0.0001
    // 60 deg, 0.06 deep, 0.1 flat: 0.16928203... -> 0.1692
    assert!(near(
        cutting_width_mm(&vbit(60.0, 0.5), 0.06, 0.1).unwrap(),
        0.1692
    ));
    // 30 deg, 0.06 deep, 0.1 flat: 0.13215... -> 0.1321
    assert!(near(
        cutting_width_mm(&vbit(30.0, 0.5), 0.06, 0.1).unwrap(),
        0.1321
    ));
    // 30 deg, 0.1 deep, 0.05 flat: 0.10358... -> 0.1035
    assert!(near(
        cutting_width_mm(&vbit(30.0, 0.5), 0.1, 0.05).unwrap(),
        0.1035
    ));
    // 45 deg, 0.1 deep, no flat: 0.08284... -> 0.0828
    assert!(near(
        cutting_width_mm(&vbit(45.0, 1.0), 0.1, 0.0).unwrap(),
        0.0828
    ));
    // 90 deg, 0.2 deep, 0.1 flat: exactly 0.5
    assert!(near(
        cutting_width_mm(&vbit(90.0, 2.0), 0.2, 0.1).unwrap(),
        0.5
    ));
}

#[test]
fn the_width_is_never_more_than_the_diameter_or_less_than_a_hundredth() {
    // 90 deg, 0.5 deep, 0.1 flat would be 1.1 mm: the bit is only 0.3 wide
    assert!(near(
        cutting_width_mm(&vbit(90.0, 0.3), 0.5, 0.1).unwrap(),
        0.3
    ));
    // no depth and no flat is a point: the least width is 0.01
    assert!(near(
        cutting_width_mm(&vbit(10.0, 0.5), 0.0, 0.0).unwrap(),
        0.01
    ));
}

#[test]
fn an_end_mill_cuts_with_its_diameter_and_a_drill_or_a_laser_does_not_mill() {
    let mill = CribTool {
        id: 1,
        name: "M".into(),
        kind: Kind::EndMill,
        diameter_mm: 0.8,
        tip_angle_deg: None,
    };
    assert!(near(cutting_width_mm(&mill, 0.06, 0.1).unwrap(), 0.8));
    for (kind, word) in [
        (Kind::Drill, "drill"),
        (Kind::Laser, "laser"),
        (Kind::Nozzle, "nozzle"),
    ] {
        let tool = CribTool {
            kind,
            ..mill.clone()
        };
        let e = cutting_width_mm(&tool, 0.06, 0.1).unwrap_err();
        assert!(e.contains(&format!("is a {word}")), "{e}");
    }
}

#[test]
fn a_depth_or_flat_out_of_range_is_refused() {
    let v = vbit(30.0, 0.5);
    for depth in [-0.1, 100.5, f64::NAN, f64::INFINITY] {
        assert!(cutting_width_mm(&v, depth, 0.1)
            .unwrap_err()
            .contains("the depth is"));
    }
    for flat in [-0.1, 5.1, f64::NAN] {
        assert!(cutting_width_mm(&v, 0.06, flat)
            .unwrap_err()
            .contains("the flat is"));
    }
}

// ---- the settings --------------------------------------------------------------------------------------------------------------------------------------------

fn sample_tools() -> Vec<CribTool> {
    parse_csv(&std::fs::read_to_string(CRIB).unwrap()).unwrap()
}

#[test]
fn the_chosen_tools_replace_the_tools_of_the_settings_and_each_is_told() {
    let mut s = Settings {
        crib_file: "crib.csv".into(),
        crib_fine: 2,
        crib_coarse: 3,
        crib_route: 3,
        ..Settings::default()
    };
    let notes = apply(&mut s, &sample_tools()).unwrap();
    // the V-bit of 30 degrees at the depth of 0.06 with the flat of 0.1: 0.1321
    assert!(near(s.fine_tool_mm, 0.1321), "{}", s.fine_tool_mm);
    assert!(near(s.coarse_tool_mm, 1.0) && near(s.route_tool_mm, 1.0));
    assert_eq!(notes.len(), 3);
    assert!(
        notes[0].starts_with("fine_tool_mm = 0.1321 from the crib: tool 2 (V-bit 30 deg, v-bit)")
    );
    assert!(notes[1].starts_with("coarse_tool_mm = 1 from the crib: tool 3"));
}

#[test]
fn the_depth_and_the_flat_of_the_settings_are_used() {
    let mut s = Settings {
        crib_file: "crib.csv".into(),
        crib_fine: 2,
        depth_mm: 0.1,
        crib_vbit_flat_mm: 0.05,
        ..Settings::default()
    };
    apply(&mut s, &sample_tools()).unwrap();
    assert!(near(s.fine_tool_mm, 0.1035), "{}", s.fine_tool_mm);
}

#[test]
fn a_number_of_0_leaves_the_setting_alone() {
    let mut s = Settings {
        crib_file: "crib.csv".into(),
        ..Settings::default()
    };
    let before = s.clone();
    assert!(apply(&mut s, &sample_tools()).unwrap().is_empty());
    assert_eq!(s, before);
}

#[test]
fn a_missing_tool_a_drill_and_a_v_bit_for_the_outline_are_refused_with_the_setting() {
    let tools = sample_tools();
    let mut s = Settings {
        crib_file: "crib.csv".into(),
        crib_fine: 9,
        ..Settings::default()
    };
    assert!(apply(&mut s, &tools)
        .unwrap_err()
        .contains("crib_fine 9: the crib has no tool with that number"));
    s.crib_fine = 4;
    assert!(apply(&mut s, &tools)
        .unwrap_err()
        .contains("crib_fine 4: the tool 4 (Drill 0.8 mm) is a drill"));
    s.crib_fine = 0;
    s.crib_route = 2;
    assert!(apply(&mut s, &tools)
        .unwrap_err()
        .contains("crib_route 2: the cut round the board needs an end mill"));
}

#[test]
fn the_crib_keys_are_settings_with_limits() {
    let mut s = Settings::default();
    s.set("crib_file", " tools/crib.csv ").unwrap();
    s.set("crib-fine", "2").unwrap();
    s.set("crib_coarse", "3").unwrap();
    s.set("crib_route", "1").unwrap();
    s.set("crib_vbit_flat_mm", "4mil").unwrap();
    assert_eq!(s.crib_file, "tools/crib.csv");
    assert_eq!((s.crib_fine, s.crib_coarse, s.crib_route), (2, 3, 1));
    assert!(near(s.crib_vbit_flat_mm, 0.1016), "{}", s.crib_vbit_flat_mm);
    // the short name of the option
    s.set("--crib", "other.csv").unwrap();
    assert_eq!(s.crib_file, "other.csv");
    for (key, v) in [
        ("crib_fine", "-1"),
        ("crib_fine", "x"),
        ("crib_coarse", "4294967296"),
        ("crib_file", "a#b.csv"),
    ] {
        assert!(s.set(key, v).is_err(), "{key} {v}");
    }
    s.crib_vbit_flat_mm = 5.5;
    assert!(s.validate().unwrap_err().contains("crib_vbit_flat_mm"));
}

#[test]
fn a_tool_of_the_crib_without_a_table_is_refused() {
    let e = Settings::parse("crib_fine = 2\n").unwrap_err();
    assert!(
        e.contains("crib_fine is 2: a tool of the crib needs crib_file"),
        "{e}"
    );
    let ok = Settings::parse("crib_file = c.csv\ncrib_fine = 2\n").unwrap();
    assert_eq!((ok.crib_file.as_str(), ok.crib_fine), ("c.csv", 2));
}

#[test]
fn the_crib_settings_are_written_and_read_back() {
    let s = Settings {
        crib_file: "C:\\eh3d tools\\crib.csv".into(),
        crib_fine: 2,
        crib_route: 3,
        crib_vbit_flat_mm: 0.25,
        ..Settings::default()
    };
    let text = s.to_text();
    assert!(text.contains("crib_file = C:\\eh3d tools\\crib.csv\n"));
    assert!(text.contains("crib_vbit_flat_mm = 0.25\n"));
    assert_eq!(Settings::parse(&text).unwrap(), s);
    assert!(Settings::default().to_text().contains("crib_file = \n"));
}

// ---- the command line ----------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn the_command_line_takes_the_tool_from_the_crib_and_tells_it() {
    let (ok, with, err) = run(&["info", SAMPLE, "--crib", CRIB, "--crib-fine", "2"]);
    assert!(ok, "{err}");
    assert!(
        err.contains("note: fine_tool_mm = 0.1321 from the crib: tool 2 (V-bit 30 deg, v-bit)"),
        "{err}"
    );
    // the same as writing the width by hand
    let (ok, by_hand, err) = run(&["info", SAMPLE, "--fine-tool-mm", "0.1321"]);
    assert!(ok, "{err}");
    assert_eq!(with, by_hand);
}

#[test]
fn a_configuration_file_names_the_crib_and_the_options_win_over_it() {
    let cfg = tmp("crib_job.cfg");
    std::fs::write(&cfg, format!("crib_file = {CRIB}\ncrib_fine = 1\n")).unwrap();
    let (ok, _, err) = run(&["info", SAMPLE, "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(
        err.contains("fine_tool_mm = 0.2 from the crib: tool 1"),
        "{err}"
    );
    let (ok, _, err) = run(&["info", SAMPLE, "--config", &cfg, "--crib-fine", "3"]);
    assert!(ok, "{err}");
    assert!(
        err.contains("fine_tool_mm = 1 from the crib: tool 3"),
        "{err}"
    );
}

#[test]
fn the_table_is_not_read_when_no_tool_is_chosen_and_config_does_not_apply_it() {
    let (ok, _, err) = run(&["info", SAMPLE, "--crib", "no/such/crib.csv"]);
    assert!(ok, "{err}");
    assert!(!err.contains("crib"), "{err}");
    let (ok, text, err) = run(&["config", "--crib", "no/such/crib.csv", "--crib-fine", "2"]);
    assert!(ok, "{err}");
    assert!(text.contains("crib_file = no/such/crib.csv\n") && text.contains("crib_fine = 2\n"));
    assert!(text.contains("fine_tool_mm = 0.2\n"), "{text}");
}

#[test]
fn the_command_line_refuses_what_the_crib_cannot_give() {
    let (ok, _, err) = run(&["info", SAMPLE, "--crib-fine", "2"]);
    assert!(
        !ok && err.contains("crib_fine is 2: a tool of the crib needs crib_file"),
        "{err}"
    );
    let (ok, _, err) = run(&[
        "info",
        SAMPLE,
        "--crib",
        "no/such/crib.csv",
        "--crib-fine",
        "2",
    ]);
    assert!(!ok && err.contains("no/such/crib.csv"), "{err}");
    let (ok, _, err) = run(&["info", SAMPLE, "--crib", CRIB, "--crib-fine", "9"]);
    assert!(
        !ok && err.contains("crib_fine 9: the crib has no tool"),
        "{err}"
    );
    let (ok, _, err) = run(&["info", SAMPLE, "--crib", CRIB, "--crib-fine", "4"]);
    assert!(!ok && err.contains("is a drill"), "{err}");
    let bad = tmp("crib_bad.csv");
    std::fs::write(&bad, "id,name\n1,A\n").unwrap();
    let (ok, _, err) = run(&["info", SAMPLE, "--crib", &bad, "--crib-fine", "1"]);
    assert!(!ok && err.contains("the table has no column"), "{err}");
}

#[test]
fn a_tool_from_the_crib_that_is_too_thin_for_the_limit_of_the_file_is_refused() {
    // the V-bit of 30 degrees makes 0.1321; a limit of 0.2 mm refuses it by the name of the setting
    let (ok, _, err) = run(&[
        "info",
        SAMPLE,
        "--crib",
        CRIB,
        "--crib-fine",
        "2",
        "--min-tool-mm",
        "0.2",
    ]);
    assert!(
        !ok && err.contains("0.1321") && err.contains("min_tool_mm"),
        "{err}"
    );
}
