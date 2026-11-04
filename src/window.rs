//! The state of the window, without any screen: the layer that is open, the parameters being edited, the paths worked out from them, what was last said to the person and the zoom of the
//! picture. The screen (`gui`, built with the `gui` feature) only draws this and passes on what the person does, so everything here is tested as plain code.

use crate::config::Settings;
use crate::draw::Drawing;
use crate::forced::{forced_passes, insert};
use crate::gcode::write_in;
use crate::hpgl::{isocam_style, set_pen_speed, write_opts, PenBy, PlotOptions};
use crate::isolate::{plan_with_pads, Pass};
use crate::lang::{fill, Lang, Msg};
use crate::optimize::optimize_passes;
use crate::tolerance::close_pieces_scoped;
use crate::units::Unit;
use crate::viewport::Viewport;
use crate::{dxf, filetype};
use eh3d_gerberviewer::scene::{from_gerber_with, Scene, SceneOptions};
use eh3d_parser_gcode::emit::{header_with_units, sha256_hex, Header};
use eh3d_parser_gerber::{parse_bytes_with, read_file_limited, InputLimits};
use std::fs;
use std::path::{Path, PathBuf};

/// The extensions of the files the window offers to open as a Gerber layer.
pub const LAYER_EXTENSIONS: [&str; 14] = [
    "gtl", "gbl", "gts", "gbs", "gto", "gbo", "gtp", "gbp", "gko", "gm1", "gbr", "ger", "pho",
    "art",
];

/// A parameter of the window: the setting it changes, the sentence that names it and the text being typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: &'static str,
    pub label: Msg,
    pub text: String,
}

/// The parameters the window offers, in the order they are shown, with the values of the settings.
pub fn default_fields(s: &Settings) -> Vec<Field> {
    let f = |key: &'static str, label: Msg, text: String| Field { key, label, text };
    vec![
        f(
            "fine_tool_mm",
            Msg::ToolDiameter,
            s.fine_tool_mm.to_string(),
        ),
        f("fine_passes", Msg::Passes, s.fine_passes.to_string()),
        f(
            "fine_stepover_mm",
            Msg::Stepover,
            s.fine_stepover_mm.to_string(),
        ),
        f("depth_mm", Msg::Depth, s.depth_mm.to_string()),
        f("feed_mm_min", Msg::Feed, s.feed_mm_min.to_string()),
        f("plunge_mm_min", Msg::Plunge, s.plunge_mm_min.to_string()),
        f("safe_z_mm", Msg::SafeZ, s.safe_z_mm.to_string()),
        f("spindle_rpm", Msg::Spindle, s.spindle_rpm.to_string()),
        f("res_mm", Msg::Resolution, s.res_mm.to_string()),
    ]
}

/// What the window last said to the person.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    Opened {
        name: String,
        shapes: usize,
        width_mm: f64,
        height_mm: f64,
    },
    Computed {
        passes: usize,
        paths: usize,
        cut_mm: f64,
    },
    Exported {
        file: String,
    },
    NoLayer,
    Failed(String),
    Gaps(usize),
    NotDrawn(String),
    Stale,
    BadValue {
        label: Msg,
        why: String,
    },
    SameFile,
}

impl Status {
    /// The sentence, in the language of the person.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::Opened {
                name,
                shapes,
                width_mm,
                height_mm,
            } => {
                let (shapes, width, height) = (
                    shapes.to_string(),
                    format!("{width_mm:.2}"),
                    format!("{height_mm:.2}"),
                );
                fill(
                    Msg::Opened.tr(lang),
                    &[
                        name.as_str(),
                        shapes.as_str(),
                        width.as_str(),
                        height.as_str(),
                    ],
                )
            }
            Status::Computed {
                passes,
                paths,
                cut_mm,
            } => {
                let (passes, paths, cut) = (
                    passes.to_string(),
                    paths.to_string(),
                    format!("{cut_mm:.1}"),
                );
                fill(
                    Msg::Computed.tr(lang),
                    &[passes.as_str(), paths.as_str(), cut.as_str()],
                )
            }
            Status::Exported { file } => fill(Msg::Exported.tr(lang), &[file.as_str()]),
            Status::NoLayer => Msg::NoLayer.tr(lang).to_string(),
            Status::Failed(why) => fill(Msg::Failed.tr(lang), &[why.as_str()]),
            Status::Gaps(n) => fill(Msg::Gaps.tr(lang), &[n.to_string().as_str()]),
            Status::NotDrawn(why) => fill(Msg::NotDrawn.tr(lang), &[why.as_str()]),
            Status::Stale => Msg::Stale.tr(lang).to_string(),
            Status::BadValue { label, why } => {
                fill(Msg::BadValue.tr(lang), &[label.tr(lang), why.as_str()])
            }
            Status::SameFile => Msg::SameFile.tr(lang).to_string(),
        }
    }

    /// Whether the sentence is a problem or a warning (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        !matches!(
            self,
            Status::Ready
                | Status::Opened { .. }
                | Status::Computed { .. }
                | Status::Exported { .. }
        )
    }
}

/// The layer that is open.
#[derive(Debug, Clone)]
pub struct Layer {
    pub path: PathBuf,
    pub name: String,
    pub scene: Scene,
    pub drawing: Drawing,
}

/// The window.
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    /// Write although the tool does not fit between pieces of copper or the layer has copper that is not drawn.
    pub force: bool,
    pub layer: Option<Layer>,
    pub passes: Vec<Pass>,
    /// The places where the tool does not fit and no path was laid.
    pub unfixed: usize,
    pub status: Status,
    /// The parameters changed after the paths were worked out.
    pub stale: bool,
    pub viewport: Viewport,
    base: Settings,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        let base = Settings::default();
        Window {
            lang,
            fields: default_fields(&base),
            force: false,
            layer: None,
            passes: Vec::new(),
            unfixed: 0,
            status: Status::Ready,
            stale: false,
            viewport: Viewport::default(),
            base,
        }
    }

    /// The settings with the values typed in the parameters. A value that is not valid says which parameter it is.
    pub fn settings(&self) -> Result<Settings, Status> {
        let mut s = self.base.clone();
        for f in &self.fields {
            s.set(f.key, &f.text).map_err(|why| Status::BadValue {
                label: f.label,
                why,
            })?;
        }
        s.validate().map_err(Status::Failed)?;
        Ok(s)
    }

    /// Changes the text of a parameter; the paths shown are then out of date.
    pub fn set_field(&mut self, index: usize, text: String) {
        if let Some(f) = self.fields.get_mut(index) {
            if f.text != text {
                f.text = text;
                self.stale = true;
            }
        }
    }

    /// The box of the picture: the copper and the paths.
    fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        let (mut lo, mut hi) = self
            .layer
            .as_ref()
            .and_then(|l| l.drawing.extent())
            .unwrap_or((
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            ));
        for p in self
            .passes
            .iter()
            .flat_map(|p| p.paths.iter().chain(p.open.iter()))
        {
            for q in p {
                lo = (lo.0.min(q.0), lo.1.min(q.1));
                hi = (hi.0.max(q.0), hi.1.max(q.1));
            }
        }
        (lo.0 <= hi.0 && lo.1 <= hi.1).then_some((lo, hi))
    }

    /// Shows the whole drawing in a picture of this size.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((lo, hi)) = self.extent() {
            self.viewport = Viewport::fit(lo, hi, width_px, height_px, 24.0);
        }
    }

    /// Opens a Gerber layer and works out the paths with the parameters. The layer stays closed (and the old one open) when it cannot be read.
    pub fn open(&mut self, path: &Path) -> bool {
        match read_layer(path, &self.base) {
            Ok(layer) => {
                self.layer = Some(layer);
                self.passes.clear();
                self.unfixed = 0;
                self.stale = false;
                self.compute();
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// What stops the paths being written: copper that is not drawn, or places where the tool does not fit. The box "write anyway" lifts it.
    fn blocker(&self) -> Option<Status> {
        if self.force {
            return None;
        }
        let notes = self.layer.as_ref().map(|l| l.scene.notes.clone())?;
        if !notes.is_empty() {
            return Some(Status::NotDrawn(notes.join("; ")));
        }
        (self.unfixed > 0).then_some(Status::Gaps(self.unfixed))
    }

    /// Works out the paths of the layer with the parameters. Returns whether it could.
    pub fn compute(&mut self) -> bool {
        let Some(scene) = self.layer.as_ref().map(|l| l.scene.clone()) else {
            self.status = Status::NoLayer;
            return false;
        };
        let settings = match self.settings() {
            Ok(s) => s,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        match build(&scene, &settings) {
            Ok((passes, unfixed)) => {
                self.passes = passes;
                self.unfixed = unfixed;
                self.stale = false;
                self.status = self.blocker().unwrap_or_else(|| self.computed_status());
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// The sentence for paths that are worked out and nothing stops: how many passes and paths and how much cut.
    fn computed_status(&self) -> Status {
        Status::Computed {
            passes: self.passes.len(),
            paths: self.passes.iter().map(|p| p.cuts().len()).sum(),
            cut_mm: self
                .passes
                .iter()
                .flat_map(Pass::cuts)
                .map(|c| c.length_mm())
                .sum(),
        }
    }

    /// Switches "write anyway" on or off; the sentence of the status follows when paths are shown.
    pub fn set_force(&mut self, force: bool) {
        self.force = force;
        if self.layer.is_some() && !self.passes.is_empty() && !self.stale {
            self.status = self.blocker().unwrap_or_else(|| self.computed_status());
        }
    }

    /// The name suggested for the file of the paths: the name of the layer with `.nc`, next to it.
    pub fn suggested_output(&self) -> Option<PathBuf> {
        self.layer.as_ref().map(|l| l.path.with_extension("nc"))
    }

    /// Writes the paths: G-code, or HP-GL for a name that ends in `.plt` and a DXF drawing for `.dxf`. They are worked out again first when the parameters changed.
    pub fn export(&mut self, out: &Path) -> bool {
        let Some(layer_path) = self.layer.as_ref().map(|l| l.path.clone()) else {
            self.status = Status::NoLayer;
            return false;
        };
        if (self.stale || self.passes.is_empty()) && !self.compute() {
            return false;
        }
        let same = match (fs::canonicalize(&layer_path), fs::canonicalize(out)) {
            (Ok(a), Ok(b)) => a == b,
            _ => layer_path == out,
        };
        if same {
            self.status = Status::SameFile;
            return false;
        }
        if let Some(block) = self.blocker() {
            self.status = block;
            return false;
        }
        let settings = match self.settings() {
            Ok(s) => s,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let rendered = match self.layer.as_ref() {
            Some(layer) => render(&self.passes, &settings, layer, out),
            None => Err(String::new()),
        };
        let text = match rendered {
            Ok(t) => t,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        match fs::write(out, text) {
            Ok(()) => {
                self.status = Status::Exported {
                    file: out
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| out.display().to_string()),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", out.display()));
                false
            }
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Reads a Gerber layer the way the command line does: with the size limit, refusing what is not a Gerber file, and with the arc tolerance of the settings.
pub fn read_layer(path: &Path, settings: &Settings) -> Result<Layer, String> {
    let name = file_name(path);
    let limits = InputLimits {
        max_bytes: (settings.max_input_mb * 1_048_576.0) as usize,
    };
    let bytes = read_file_limited(path, &limits).map_err(|e| format!("{name}: {e}"))?;
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => e.into_bytes().iter().map(|&b| char::from(b)).collect(),
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
    filetype::expect_gerber(&name, filetype::detect(&text))?;
    let text = settings
        .gerber_text(&text, None)
        .map_err(|e| format!("{name}: {e}"))?;
    let file = parse_bytes_with(text.as_bytes(), &limits).map_err(|e| format!("{name}: {e}"))?;
    let scene = from_gerber_with(
        &file,
        &SceneOptions {
            arc_tolerance_mm: settings.arc_tolerance_mm,
        },
    );
    if scene.shapes.is_empty() {
        return Err(format!("{name}: has nothing to draw"));
    }
    let drawing = Drawing::new(&scene.shapes);
    Ok(Layer {
        path: path.to_path_buf(),
        name,
        scene,
        drawing,
    })
}

/// The passes of a layer for these settings and the number of places where the tool does not fit and nothing was laid (as the command line works them out).
fn build(scene: &Scene, s: &Settings) -> Result<(Vec<Pass>, usize), String> {
    let pl = s.plan()?;
    pl.validate()?;
    let close = close_pieces_scoped(
        scene,
        pl.finest_mm(),
        s.conflict_tolerance_mm,
        s.conflict_tolerance_scope,
        pl.res_mm,
    )?;
    let mut passes = plan_with_pads(scene, &pl, s.around_pads())?;
    let mut unfixed = close.len();
    if let Some(f) = s.forced() {
        let r = forced_passes(scene, &pl, &f)?;
        unfixed = r.too_narrow.len();
        insert(&mut passes, r.passes);
    }
    let o = s.optimize();
    if !o.is_off() {
        optimize_passes(&mut passes, &o)?;
    }
    Ok((passes, unfixed))
}

/// The text of the output file for its name.
fn render(passes: &[Pass], s: &Settings, layer: &Layer, out: &Path) -> Result<String, String> {
    let ext = out
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let order = s.order();
    match ext.as_str() {
        "plt" => {
            let pl = s.plan()?;
            let pens = if pl.tools.len() > 1 || pl.rubout.is_some() {
                PenBy::Tool
            } else {
                PenBy::Pass
            };
            let b = layer.scene.bounds();
            let origin = (b.min_x.is_finite() && b.min_y.is_finite())
                .then_some((b.min_x - s.hpgl_margin_mm, b.min_y - s.hpgl_margin_mm));
            let opts = PlotOptions {
                keep_negative: !s.hpgl_shift_negative,
                pen_of_pass: Vec::new(),
                origin,
            };
            let text = set_pen_speed(
                &write_opts(passes, &order, pens, &opts)?,
                s.hpgl_pen_speed_cm_s,
            )?;
            Ok(if s.hpgl_isocam_style {
                isocam_style(&text)
            } else {
                text
            })
        }
        "dxf" => dxf::write(passes),
        _ => {
            let g = write_in(passes, &s.job(), &order, s.output_units)?;
            Ok(format!("{}{g}", gcode_header(&layer.path, s)))
        }
    }
}

/// The standard header of a G-code file: the generator and its version, the units, the file the work came from with its SHA-256 and the tools.
fn gcode_header(layer: &Path, s: &Settings) -> String {
    let named = fs::read(layer).ok().map(|bytes| sha256_hex(&bytes));
    let name = file_name(layer);
    let units = if s.output_units == Unit::Inch {
        "inch (G20)"
    } else {
        "mm (G21)"
    };
    header_with_units(
        &Header {
            generator: "eh3d-isocam",
            version: env!("CARGO_PKG_VERSION"),
            source: named.map(|hash| (name.as_str(), hash)),
            tools: "one tool per pass, see the comment of each pass",
        },
        units,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample.gtl")
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eh3d-isocam-window-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_parameters_start_with_the_values_of_the_settings() {
        let w = Window::new(Lang::En);
        let s = w.settings().unwrap();
        assert_eq!(s, Settings::default());
        assert_eq!(w.fields[0].key, "fine_tool_mm");
        assert_eq!(w.fields[0].text, "0.2");
    }

    #[test]
    fn a_value_that_is_not_a_number_names_its_parameter() {
        let mut w = Window::new(Lang::En);
        w.set_field(0, "abc".into());
        match w.settings() {
            Err(Status::BadValue { label, .. }) => assert_eq!(label, Msg::ToolDiameter),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_value_the_job_cannot_use_is_refused_by_the_checks_of_the_settings() {
        let mut w = Window::new(Lang::En);
        w.set_field(1, "0".into());
        assert!(w.settings().is_err());
    }

    #[test]
    fn changing_a_parameter_makes_the_paths_out_of_date_only_when_the_text_changes() {
        let mut w = Window::new(Lang::En);
        w.set_field(0, "0.2".into());
        assert!(!w.stale);
        w.set_field(0, "0.3".into());
        assert!(w.stale);
        w.set_field(99, "1".into());
    }

    #[test]
    fn opening_the_sample_draws_it_and_works_out_the_paths() {
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        assert!(!w.passes.is_empty());
        assert!(matches!(w.status, Status::Computed { .. }));
        assert!(w.layer.as_ref().unwrap().drawing.extent().is_some());
        w.fit(800.0, 600.0);
        assert!(w.viewport.px_per_mm > 0.0);
    }

    #[test]
    fn more_passes_give_more_paths() {
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        let one = w.passes.len();
        w.set_field(1, "3".into());
        assert!(w.stale);
        assert!(w.compute());
        assert!(!w.stale);
        assert!(w.passes.len() > one);
    }

    #[test]
    fn a_file_that_is_not_a_layer_is_refused_and_the_old_layer_stays() {
        let dir = scratch("refuse");
        let bad = dir.join("notes.gtl");
        fs::write(&bad, "this is not a Gerber file\n").unwrap();
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        assert!(!w.open(&bad));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(w.status.is_problem());
        assert_eq!(w.layer.as_ref().unwrap().name, "sample.gtl");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_is_missing_is_an_error_not_a_panic() {
        let mut w = Window::new(Lang::En);
        assert!(!w.open(Path::new("/this/file/does/not/exist.gtl")));
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn exporting_writes_gcode_with_the_header_and_the_status_says_where() {
        let dir = scratch("gcode");
        let out = dir.join("sample.nc");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("eh3d-isocam"));
        assert!(text.contains("G21"));
        assert_eq!(
            w.status,
            Status::Exported {
                file: "sample.nc".into()
            }
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_with_a_changed_parameter_works_the_paths_out_again() {
        let dir = scratch("again");
        let out = dir.join("again.nc");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        w.set_field(3, "0.1".into());
        assert!(w.export(&out));
        assert!(fs::read_to_string(&out).unwrap().contains("G1 Z-0.1 F"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_by_the_name_of_a_plot_or_a_drawing_gives_that_format() {
        let dir = scratch("formats");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        assert!(w.export(&dir.join("sample.plt")));
        assert!(fs::read_to_string(dir.join("sample.plt"))
            .unwrap()
            .contains("PD"));
        assert!(w.export(&dir.join("sample.dxf")));
        assert!(fs::read_to_string(dir.join("sample.dxf"))
            .unwrap()
            .contains("POLYLINE"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_output_may_not_be_the_layer_itself() {
        let dir = scratch("same");
        let copy = dir.join("copy.gtl");
        fs::copy(sample(), &copy).unwrap();
        let mut w = Window::new(Lang::En);
        assert!(w.open(&copy));
        assert!(!w.export(&copy));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(
            fs::read_to_string(&copy).unwrap(),
            fs::read_to_string(sample()).unwrap()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_without_a_layer_says_so() {
        let mut w = Window::new(Lang::En);
        assert!(!w.export(Path::new("nothing.nc")));
        assert_eq!(w.status, Status::NoLayer);
        assert!(!w.compute());
    }

    #[test]
    fn where_the_tool_does_not_fit_nothing_is_written_unless_asked() {
        let dir = scratch("gaps");
        let layer = dir.join("close.gtl");
        // two tracks 0.1 mm apart (edge to edge) and a tool of 0.2 mm
        fs::write(
            &layer,
            "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.3*%\nD10*\nX0Y0D02*\nX100000Y0D01*\nX0Y4000D02*\nX100000Y4000D01*\nM02*\n",
        )
        .unwrap();
        let out = dir.join("close.nc");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&layer));
        assert!(w.unfixed > 0);
        assert!(matches!(w.status, Status::Gaps(_)));
        assert!(!w.export(&out));
        assert!(!out.exists());
        w.force = true;
        assert!(w.export(&out));
        assert!(out.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sentences_of_the_status_are_in_the_language_chosen() {
        let st = Status::Computed {
            passes: 2,
            paths: 7,
            cut_mm: 123.456,
        };
        assert_eq!(st.text(Lang::En), "2 passes, 7 paths, 123.5 mm of cut");
        assert_eq!(
            st.text(Lang::Es),
            "2 pasadas, 7 trayectorias, 123.5 mm de corte"
        );
        assert_eq!(Status::Ready.text(Lang::De), "Bereit");
        assert!(!st.is_problem());
        assert!(Status::Stale.is_problem());
    }
}
