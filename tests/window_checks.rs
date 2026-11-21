//! The checks and the report of the job in the window, without a screen: the same findings as the command line, in the language of the window. The window has not been looked at on a
//! screen, and the buttons that call these are not written yet.

use eh3d_isocam::crib;
use eh3d_isocam::lang::Lang;
use eh3d_isocam::window::{Status, Window};
use std::path::PathBuf;

fn sample() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample.gtl")
}

fn table() -> Vec<crib::CribTool> {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/crib.csv"),
    )
    .unwrap();
    crib::parse_csv(&text).unwrap()
}

#[test]
fn the_window_runs_check_drc_and_preview() {
    let mut w = Window::new(Lang::En);
    // nothing is open: nothing to check, and the window says so
    assert!(w.checks_text().is_none());
    assert_eq!(w.status, Status::NoLayer);
    assert!(w.open(&sample()));
    // the check and the design rules of the command line
    assert_eq!(w.checks_text().as_deref(), Some("Checks: nothing found"));
    // the picture of the paths over the copper is what `preview` draws
    assert!(w.compute());
    assert!(!w.passes.is_empty() && !w.view.drawing.shapes.is_empty());
    // the report of the job, with its tools set against the table of the crib
    let text = w.report_text(Some(&table())).unwrap();
    assert!(text.contains("crib tool 1 (End mill 0.2 mm)"), "{text}");
    assert!(text.contains("Nothing found before the job"), "{text}");
}

#[test]
fn the_checks_and_the_report_follow_the_language_and_the_parameters_of_the_window() {
    let mut w = Window::new(Lang::De);
    assert!(w.open(&sample()));
    assert_eq!(
        w.checks_text().as_deref(),
        Some("Prüfungen: nichts gefunden")
    );
    assert!(w.report_text(None).unwrap().starts_with("Auftrag: "));
    // a tool the crib does not have is told after the parameters change
    w.lang = Lang::Es;
    let i = w
        .fields
        .iter()
        .position(|f| f.key == "fine_tool_mm")
        .unwrap();
    w.set_field(i, "0.15".to_string());
    let text = w.report_text(Some(&table())).unwrap();
    assert!(
        text.contains("el almacén no tiene ninguna que corte ese ancho"),
        "{text}"
    );
    assert!(text.contains("1 avisos antes del trabajo"), "{text}");
    // a value that cannot be used is told by the status and nothing is made of it
    w.set_field(i, "wide".to_string());
    assert!(w.report_text(None).is_none());
    assert!(matches!(w.status, Status::BadValue { .. }));
}

#[test]
fn the_arcs_are_a_parameter_of_the_window_and_the_report_tells_the_saving() {
    let mut w = Window::new(Lang::En);
    assert!(w.open(&sample()));
    let i = w
        .fields
        .iter()
        .position(|f| f.key == "arc_fit_tolerance_mm")
        .expect("the parameter is in the window");
    assert_eq!(w.fields[i].text, "0");
    w.set_field(i, "0.01".to_string());
    let text = w.report_text(None).unwrap();
    assert!(text.contains(" are arcs"), "{text}");
    assert!(
        text.contains("moves instead of") && text.contains("arcs fitted within 0.01 mm"),
        "{text}"
    );
}
