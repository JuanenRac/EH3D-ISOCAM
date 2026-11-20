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
    let (ok, _, err) = run(&["isolate", f, f]);
    assert!(!ok && err.contains("also an input"), "{err}");
    assert_eq!(fs::read_to_string(&file).unwrap(), BOARD);
    // the same file written through another spelling of its path
    let other = dir.join(".").join("board.gtl");
    let (ok, _, err) = run(&["isolate", f, other.to_str().unwrap()]);
    assert!(!ok && err.contains("also an input"), "{err}");
    assert_eq!(fs::read_to_string(&file).unwrap(), BOARD);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_marks_of_a_csv_file_become_fiducials_and_a_header_or_a_comment_is_left_out() {
    let dir = folder("fiducials_csv");
    let file = dir.join("marks.csv");
    fs::write(
        &file,
        "design_x,design_y,measured_x,measured_y\n# the first two marks\n0,0,0.1,0.2\n50;0;50.1;0.15\n  0 40 0.05 40.2\n",
    )
    .unwrap();
    let (ok, out, err) = run(&["config", "--fiducials-csv", file.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert_eq!(out.matches("fiducial = ").count(), 3, "{out}");
    let (ok, _, err) = run(&[
        "config",
        "--fiducials-csv",
        dir.join("none.csv").to_str().unwrap(),
    ]);
    assert!(!ok && err.contains("none.csv"), "{err}");
    fs::write(&file, "0,0,1\n").unwrap();
    let (ok, _, err) = run(&["config", "--fiducials-csv", file.to_str().unwrap()]);
    assert!(!ok && err.contains("four numbers"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn info_estimates_the_time_only_when_the_settings_give_the_rapid_speed() {
    let dir = folder("time");
    let file = dir.join("t.gtl");
    fs::write(&file, BOARD).unwrap();
    let cfg = dir.join("isocam.cfg");
    fs::write(&cfg, "rapid_mm_min = 3000\n").unwrap();
    let (f, c) = (file.to_str().unwrap(), cfg.to_str().unwrap());
    let (ok, out, err) = run(&["info", f]);
    assert!(ok && !out.contains("estimated time"), "{out}{err}");
    let (ok, out, err) = run(&["info", f, "--config", c]);
    assert!(ok && out.contains("estimated time: "), "{out}{err}");
    fs::write(&cfg, "rapid_mm_min = 50\n").unwrap();
    let (ok, _, err) = run(&["info", f, "--config", c]);
    assert!(!ok && err.contains("rapid_mm_min is 50"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_settings_file_over_a_mebibyte_is_refused_and_a_normal_one_is_read() {
    let dir = folder("cfg_limit");
    let board = dir.join("b.gtl");
    fs::write(&board, BOARD).unwrap();
    let big = dir.join("big.cfg");
    let padding = "# padding padding padding\n".repeat(50_000);
    fs::write(&big, format!("{padding}max_input_mb = 1\n")).unwrap();
    let b = board.to_str().unwrap();
    let (ok, _, err) = run(&["info", b, "--config", big.to_str().unwrap()]);
    assert!(!ok && err.contains("is refused"), "{err}");
    let small = dir.join("small.cfg");
    fs::write(&small, "max_input_mb = 1\n").unwrap();
    let (ok, _, err) = run(&["info", b, "--config", small.to_str().unwrap()]);
    assert!(ok, "{err}");
    let _ = fs::remove_dir_all(&dir);
}

// ---- the server mode ----

/// A server of the computer that answers `n` requests with `status` and `body` and gives back the requests it got.
fn local_server(
    n: usize,
    status: &str,
    body: &str,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let reply = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = std::thread::spawn(move || {
        let mut heads = Vec::new();
        for _ in 0..n {
            let Ok((mut s, _)) = listener.accept() else {
                break;
            };
            let mut buf = vec![0u8; 262_144];
            let mut got = 0;
            loop {
                let k = s.read(&mut buf[got..]).unwrap_or(0);
                if k == 0 {
                    break;
                }
                got += k;
                let text = String::from_utf8_lossy(&buf[..got]).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let want = text[..split]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if got >= split + 4 + want {
                        break;
                    }
                }
            }
            heads.push(String::from_utf8_lossy(&buf[..got]).to_string());
            let _ = s.write_all(reply.as_bytes());
        }
        heads
    });
    (address, handle)
}

const CREATED: &str = "{\"artifact_id\":\"a-7\",\"blob_sha256\":\"ab\",\"size\":9}";

fn link_file(dir: &std::path::Path, address: &str) -> String {
    let cfg = dir.join("link.cfg");
    fs::write(
        &cfg,
        format!("server_address = {address}\nserver_token = program-token\nserver_retries = 0\nserver_timeout_s = 5\n"),
    )
    .unwrap();
    cfg.to_str().unwrap().to_string()
}

#[test]
fn the_output_of_isolate_is_kept_on_the_server_with_the_values_it_was_made_with() {
    let dir = folder("to_server");
    let layer = dir.join("b.gtl");
    fs::write(&layer, BOARD).unwrap();
    let out = dir.join("b.nc");
    let (address, handle) = local_server(1, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (ok, stdout, err) = run(&[
        "isolate",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--config",
        &cfg,
        "--to-server",
    ]);
    assert!(ok, "{err}");
    assert!(
        stdout.contains("kept on the server: b.nc as a-7"),
        "{stdout}"
    );
    assert!(out.exists());
    let heads = handle.join().unwrap();
    assert!(
        heads[0].starts_with("POST /api/v1/artifacts HTTP/1.1"),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("Bearer program-token"));
    assert!(heads[0].contains("\"name\":\"b.nc\""), "{}", heads[0]);
    assert!(
        heads[0].contains("\"format\":\"text/x-gcode\""),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("\"command\":\"isolate\""), "{}", heads[0]);
    assert!(
        heads[0].contains("\"fine_tool_mm\":\"0.2\""),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("eh3d-isocam"), "{}", heads[0]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn to_server_without_a_server_is_refused_before_any_work_and_the_local_mode_is_untouched() {
    let dir = folder("no_server");
    let layer = dir.join("b.gtl");
    fs::write(&layer, BOARD).unwrap();
    let out = dir.join("b.nc");
    let (ok, _, err) = run(&[
        "isolate",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--to-server",
    ]);
    assert!(!ok && err.contains("server mode is off"), "{err}");
    assert!(!out.exists(), "nothing was written");
    // without --to-server the same run needs no server and writes the file
    let (ok, stdout, err) = run(&["isolate", layer.to_str().unwrap(), out.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(!stdout.contains("kept on the server"));
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_server_that_refuses_is_told_after_the_file_was_written() {
    let dir = folder("refused");
    let layer = dir.join("b.gtl");
    fs::write(&layer, BOARD).unwrap();
    let out = dir.join("b.nc");
    let (address, handle) = local_server(
        1,
        "422 Unprocessable Entity",
        "{\"error\":\"bad_artifact\",\"detail\":\"name too long\"}",
    );
    let cfg = link_file(&dir, &address);
    let (ok, _, err) = run(&[
        "isolate",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--config",
        &cfg,
        "--to-server",
    ]);
    handle.join().unwrap();
    assert!(!ok, "{err}");
    assert!(
        err.contains("not kept on the server") && err.contains("name too long"),
        "{err}"
    );
    assert!(out.exists(), "the local file stays");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_report_of_a_dry_run_is_kept_on_the_server() {
    let dir = folder("simulate_server");
    let program = dir.join("p.nc");
    fs::write(
        &program,
        "G21\nG90\nG0 Z5\nG0 X1 Y1\nG1 Z-0.1 F100\nG1 X5 Y5 F300\nG0 Z5\nM2\n",
    )
    .unwrap();
    let (address, handle) = local_server(1, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (_, stdout, err) = run(&[
        "simulate",
        program.to_str().unwrap(),
        "--config",
        &cfg,
        "--to-server",
    ]);
    let heads = handle.join().unwrap();
    assert!(
        stdout.contains("kept on the server: p.nc.report.txt as a-7"),
        "{stdout} {err}"
    );
    assert!(
        heads[0].contains("\"format\":\"text/plain\""),
        "{}",
        heads[0]
    );
    assert!(
        heads[0].contains("\"command\":\"simulate\""),
        "{}",
        heads[0]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_server_command_shows_the_link_without_the_token_and_what_the_server_answers() {
    let dir = folder("server_status");
    let (address, handle) = local_server(
        1,
        "200 OK",
        "{\"name\":\"eh3d-server\",\"version\":\"0.1.8\",\"api_revision\":1,\"status\":\"ok\"}",
    );
    let cfg = link_file(&dir, &address);
    let (ok, stdout, err) = run(&["server", "status", "--config", &cfg]);
    handle.join().unwrap();
    assert!(ok, "{err}");
    assert!(
        stdout.contains("server_token = ***") && !stdout.contains("program-token"),
        "{stdout}"
    );
    assert!(stdout.contains("eh3d-server 0.1.8 (ok, api 1)"), "{stdout}");
    let (ok, stdout, _) = run(&["server", "status"]);
    assert!(ok && stdout.contains("server mode is off"), "{stdout}");
    let (ok, _, err) = run(&["server", "restart"]);
    assert!(!ok && err.contains("unknown"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_keys_of_the_link_are_settings_of_the_file_and_of_the_command_line_with_their_ranges() {
    let dir = folder("server_keys");
    let layer = dir.join("b.gtl");
    fs::write(&layer, BOARD).unwrap();
    let f = layer.to_str().unwrap();
    let cfg = dir.join("bad.cfg");
    fs::write(&cfg, "fine_tool_mm = 0.2\nserver_timeout_s = 0\n").unwrap();
    let (ok, _, err) = run(&["info", f, "--config", cfg.to_str().unwrap()]);
    assert!(
        !ok && err.contains("server_timeout_s") && err.contains("line 2"),
        "{err}"
    );
    let (ok, _, err) = run(&["info", f, "--server-retries", "9"]);
    assert!(!ok && err.contains("server_retries"), "{err}");
    let cfg = dir.join("good.cfg");
    fs::write(&cfg, "server_address = 10.0.0.5:18200\nserver_mdns = off\nserver_timeout_s = 120\nserver_retries = 5\nserver_machine = bench\n").unwrap();
    let (ok, _, err) = run(&["info", f, "--config", cfg.to_str().unwrap()]);
    assert!(ok, "{err}");
    // a plain run needs no server key and never reads the network
    let (ok, _, err) = run(&["info", f]);
    assert!(ok, "{err}");
    let _ = fs::remove_dir_all(&dir);
}
