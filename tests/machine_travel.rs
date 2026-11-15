//! A twin of a Bungard CCD that runs the G-code of the program against the travel of its table: the machine halts with an alarm at the first move that leaves the table, and what it
//! had done by then is told. The sizes of the tables are the ones written in the documents of EH3D-DRILLPRO (`docs/CONNECTION.md` and `docs/PROTOCOL.md`): 495 x 325 mm for the CCD MTC
//! (the one the programs were tried on) and 325 x 270 mm for the CCD/2 MTC (not tried). The origin of the table is taken at a corner, as the programs of the suite place the work; the
//! documents do not give the position of the home switches, so nothing is said here about the margin the real machine keeps around them. Boards come from a seeded generator, so every
//! run is the same. Nothing here has run on a machine.

use eh3d_isocam::gcode::{write_in, Job};
use eh3d_isocam::isolate::Pass;
use eh3d_isocam::order::Order;
use eh3d_isocam::units::Unit;
use eh3d_parser_gcode::{interpret, parse, Dialect};

/// The table of a machine, in mm: X from 0 to `width` and Y from 0 to `depth`.
#[derive(Debug, Clone, Copy)]
struct Table {
    name: &'static str,
    width: f64,
    depth: f64,
}

const CCD_MTC: Table = Table {
    name: "CCD MTC",
    width: 495.0,
    depth: 325.0,
};
const CCD2_MTC: Table = Table {
    name: "CCD/2 MTC",
    width: 325.0,
    depth: 270.0,
};

/// What the twin did with a program.
#[derive(Debug, Clone, PartialEq)]
enum Run {
    /// Every move was made and the program ended.
    Done { moves: usize },
    /// The machine halted: the line of the move that leaves the table, where it was and how many moves it had made.
    Alarm {
        line: usize,
        from: (f64, f64),
        made: usize,
        of: usize,
    },
}

fn inside(t: &Table, x: f64, y: f64) -> bool {
    x >= -1e-6 && y >= -1e-6 && x <= t.width + 1e-6 && y <= t.depth + 1e-6
}

fn run(t: &Table, program: &str) -> Run {
    let trace = interpret(&parse(program, Dialect::Grbl));
    let of = trace.segments.len();
    for (made, s) in trace.segments.iter().enumerate() {
        if !inside(t, s.to.x, s.to.y) {
            return Run::Alarm {
                line: s.line,
                from: (s.from.x, s.from.y),
                made,
                of,
            };
        }
    }
    Run::Done { moves: of }
}

/// Loops of a rectangle `w` x `h` at `(x, y)` and two more inside it, as the passes of an isolation cut around a board.
fn board(x: f64, y: f64, w: f64, h: f64) -> Vec<Pass> {
    let ring = |g: f64| {
        vec![
            (x + g, y + g),
            (x + w - g, y + g),
            (x + w - g, y + h - g),
            (x + g, y + h - g),
            (x + g, y + g),
        ]
    };
    vec![Pass {
        paths: vec![ring(0.0), ring(1.0), ring(2.0)],
        diameter_mm: 0.2,
        ..Pass::default()
    }]
}

fn program(passes: &[Pass]) -> String {
    write_in(passes, &Job::default(), &Order::default(), Unit::Mm).unwrap()
}

/// A small deterministic generator (SplitMix64): the same seed always gives the same numbers.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number in `[lo, hi)` on a quarter of a millimetre that is never a whole number, so nothing lies on the edge of a table.
    fn quarter(&mut self, lo: u32, hi: u32) -> f64 {
        f64::from(lo + (self.next() % u64::from(hi - lo)) as u32) + 0.25
    }
}

#[test]
fn a_board_that_fits_the_table_runs_to_the_end() {
    let text = program(&board(20.25, 20.25, 100.5, 80.5));
    match run(&CCD_MTC, &text) {
        Run::Done { moves } => assert!(moves > 12, "{moves} moves"),
        other => panic!("{other:?}\n{text}"),
    }
    assert!(matches!(run(&CCD2_MTC, &text), Run::Done { .. }));
}

#[test]
fn a_board_that_does_not_fit_the_smaller_table_halts_the_machine_at_the_first_move_out() {
    // 340 mm wide: it fits the CCD MTC (495 wide) and not the CCD/2 MTC (325 wide)
    let text = program(&board(5.25, 5.25, 340.5, 150.5));
    assert!(
        matches!(run(&CCD_MTC, &text), Run::Done { .. }),
        "{}",
        CCD_MTC.name
    );
    match run(&CCD2_MTC, &text) {
        Run::Alarm { line, made, of, .. } => {
            assert!(line > 0 && made < of, "line {line}, {made} of {of}");
            // the machine stopped there: nothing after that move was made, so the cut is left half done
            assert!(made > 0, "the first moves are inside the table");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_board_left_of_the_origin_or_beyond_the_far_edge_halts_the_machine() {
    let left = program(&board(-10.25, 20.25, 50.5, 50.5));
    let far = program(&board(450.25, 20.25, 60.5, 50.5));
    let high = program(&board(20.25, 300.25, 50.5, 40.5));
    for (name, text) in [("left", left), ("far", far), ("high", high)] {
        assert!(matches!(run(&CCD_MTC, &text), Run::Alarm { .. }), "{name}");
    }
}

#[test]
fn putting_the_board_back_inside_the_table_is_the_way_back_and_a_board_larger_than_the_table_has_none(
) {
    // the board was placed too far to the right: shift it by what sticks out, and the program runs
    let (w, h) = (80.5, 60.5);
    let bad = program(&board(450.25, 20.25, w, h));
    assert!(matches!(run(&CCD_MTC, &bad), Run::Alarm { .. }));
    let moved = program(&board(450.25 - 60.0, 20.25, w, h));
    assert!(matches!(run(&CCD_MTC, &moved), Run::Done { .. }));
    // a board of 520 mm does not fit the 495 mm of the table whatever its place: moving it does not help
    for x in [0.25, 10.25, -20.25] {
        assert!(matches!(
            run(&CCD_MTC, &program(&board(x, 20.25, 520.5, 40.5))),
            Run::Alarm { .. }
        ));
    }
}

#[test]
fn the_twin_agrees_with_the_geometry_for_boards_drawn_by_a_seeded_generator() {
    let mut rng = Seeded(20_261_115);
    let (mut fits, mut halts) = (0, 0);
    for n in 0..120 {
        let table = if n % 2 == 0 { CCD_MTC } else { CCD2_MTC };
        let (w, h) = (rng.quarter(10, 400), rng.quarter(10, 300));
        let (x, y) = (rng.quarter(0, 300), rng.quarter(0, 250));
        let text = program(&board(x, y, w, h));
        let truth = inside(&table, x, y) && inside(&table, x + w, y + h);
        match run(&table, &text) {
            Run::Done { .. } => {
                assert!(
                    truth,
                    "board {n} ({x}, {y}, {w}, {h}) on the {}",
                    table.name
                );
                fits += 1;
            }
            Run::Alarm { .. } => {
                assert!(
                    !truth,
                    "board {n} ({x}, {y}, {w}, {h}) on the {}",
                    table.name
                );
                halts += 1;
            }
        }
    }
    assert!(
        fits > 5 && halts > 5,
        "{fits} fit and {halts} halt: the generator does not try both"
    );
}

#[test]
fn the_same_program_gives_the_same_run_every_time() {
    let text = program(&board(30.25, 40.25, 120.5, 90.5));
    assert_eq!(run(&CCD_MTC, &text), run(&CCD_MTC, &text));
    let bad = program(&board(480.25, 40.25, 120.5, 90.5));
    assert_eq!(run(&CCD_MTC, &bad), run(&CCD_MTC, &bad));
}
