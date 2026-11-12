//! Damaged files with a fixed seed: a drill file, the settings and the tool table are cut at every place and copied with bytes changed, inserted or cut. Each is read without a panic,
//! and the holes that come out are finite. The seed is fixed, so a failure is the same failure every time.

use eh3d_isocam::config::Settings;
use eh3d_isocam::crib::parse_csv;
use eh3d_isocam::drill::{read_drills, read_outline};

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const METRIC: &str = "M48\nMETRIC,TZ\nT01C0.80\nT02C1.00\n%\nT01\nX10.0Y10.0\nX20.5Y10.0\nT02\nX30.0Y25.25\nX5.0Y5.0\nM30\n";
const INCHES: &str = "M48\nINCH,LZ\nT01C0.0315\n%\nT01\nX0.5Y0.5\nX1.0Y0.75\nM30\n";
const OUTLINE: &str = "G04 outline*\n%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";

const PIECES: &[u8] = b"0123456789.-eE\n ,;%XYTMGCD*";

fn file(name: &str) -> String {
    std::fs::read_to_string(format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn damaged(rng: &mut Rng, text: &str, round: usize) -> Vec<u8> {
    let mut copy = text.as_bytes().to_vec();
    for _ in 0..1 + round % 5 {
        let at = rng.below(copy.len());
        match rng.below(4) {
            0 => copy[at] = PIECES[rng.below(PIECES.len())],
            1 => copy.insert(at, rng.next() as u8),
            2 => {
                copy.remove(at);
            }
            _ => copy.truncate(at.max(1)),
        }
        if copy.is_empty() {
            copy.push(b'0');
        }
    }
    copy
}

fn check_drills(text: &str) {
    if let Ok(d) = read_drills(text) {
        for h in &d.holes {
            assert!(
                h.x_mm.is_finite() && h.y_mm.is_finite() && h.diameter_mm.is_finite(),
                "{h:?}"
            );
        }
    }
}

#[test]
fn every_cut_of_a_drill_file_is_read_without_a_panic() {
    for text in [METRIC, INCHES] {
        assert!(!read_drills(text).unwrap().holes.is_empty());
        for end in 0..=text.len() {
            if text.is_char_boundary(end) {
                check_drills(&text[..end]);
            }
        }
        let lines: Vec<&str> = text.lines().collect();
        for end in 0..=lines.len() {
            check_drills(&lines[..end].join("\n"));
        }
    }
}

#[test]
fn copies_of_a_drill_file_with_bytes_changed_are_read_without_a_panic() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for text in [METRIC, INCHES] {
        for round in 0..400 {
            let copy = damaged(&mut rng, text, round);
            check_drills(&String::from_utf8_lossy(&copy));
        }
    }
}

#[test]
fn damaged_outlines_are_read_without_a_panic() {
    let mut rng = Rng(11);
    assert!(read_outline("board.gko", OUTLINE).is_ok());
    for round in 0..400 {
        let copy = damaged(&mut rng, OUTLINE, round);
        let _ = read_outline("board.gko", &String::from_utf8_lossy(&copy));
    }
    for end in 0..=OUTLINE.len() {
        if OUTLINE.is_char_boundary(end) {
            let _ = read_outline("board.gko", &OUTLINE[..end]);
        }
    }
}

#[test]
fn damaged_settings_are_read_or_refused_without_a_panic() {
    let text = file("isocam.cfg");
    assert!(Settings::parse(&text).is_ok());
    let mut rng = Rng(77);
    for round in 0..500 {
        let copy = damaged(&mut rng, &text, round);
        if let Ok(s) = Settings::parse(&String::from_utf8_lossy(&copy)) {
            let _ = s.validate();
        }
    }
    for end in (0..=text.len()).step_by(3) {
        if text.is_char_boundary(end) {
            let _ = Settings::parse(&text[..end]);
        }
    }
}

#[test]
fn a_damaged_tool_table_is_read_or_refused_without_a_panic() {
    let text = file("crib.csv");
    assert!(parse_csv(&text).is_ok());
    let mut rng = Rng(5);
    for round in 0..500 {
        let copy = damaged(&mut rng, &text, round);
        let _ = parse_csv(&String::from_utf8_lossy(&copy));
    }
    for end in (0..=text.len()).step_by(2) {
        if text.is_char_boundary(end) {
            let _ = parse_csv(&text[..end]);
        }
    }
}
