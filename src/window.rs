//! The state of the window, without any screen: the files that are open (the copper layer, the drills, the outline, the mask layer and the reference points), the parameters being
//! edited, the paths worked out from them, what was last said to the person and the zoom of the picture. The screen (`gui`, built with the `gui` feature) only draws this and passes
//! on what the person does, so everything here is tested as plain code. The steps that write files are the ones of the command line (`board`).

use crate::board::{self, DrillFile, OutlineFile};
use crate::config::Settings;
use crate::contour::Loop;
use crate::draw::Drawing;
use crate::drill::Hole;
use crate::forced::{forced_passes, insert};
use crate::gcode::write_in;
use crate::isolate::{plan_with_pads, Pass};
use crate::lang::{fill, Lang, Msg};
use crate::optimize::optimize_passes;
use crate::tolerance::close_pieces_scoped;
use crate::transform::{Axis, Board};
use crate::units::Unit;
use crate::viewport::Viewport;
use crate::{dxf, filetype};
use eh3d_gerberviewer::scene::{from_gerber_with, Bounds, Scene, SceneOptions};
use eh3d_parser_gcode::emit::{header_with_units, sha256_hex, Header};
use eh3d_parser_gerber::{parse_bytes_with, InputLimits};
use std::fs;
use std::path::{Path, PathBuf};

/// The extensions of the files the window offers to open as a Gerber layer.
pub const LAYER_EXTENSIONS: [&str; 14] = [
    "gtl", "gbl", "gts", "gbs", "gto", "gbo", "gtp", "gbp", "gko", "gm1", "gbr", "ger", "pho",
    "art",
];

/// The extensions of the files the window offers to open as a drill file.
pub const DRILL_EXTENSIONS: [&str; 6] = ["drl", "ncd", "txt", "xln", "exc", "drd"];

/// The extensions of the files the window offers to open as the outline of the board (a Gerber layer, a DXF drawing or a plot file).
pub const OUTLINE_EXTENSIONS: [&str; 10] = [
    "gko", "gm1", "gml", "gbr", "ger", "pho", "art", "dxf", "plt", "hpgl",
];

/// The extensions of the files the window offers to open as a solder mask layer.
pub const MASK_EXTENSIONS: [&str; 6] = ["gts", "gbs", "gbr", "ger", "pho", "art"];

/// The extensions of the files the window offers to open as a table of reference points.
pub const MARKS_EXTENSIONS: [&str; 3] = ["csv", "tsv", "txt"];

/// The extensions the window offers for the file of the paths of the copper.
pub const PATHS_EXTENSIONS: [&str; 3] = ["nc", "plt", "dxf"];

/// The extensions the window offers for the drill file or the plot of the holes.
pub const DRILL_OUT_EXTENSIONS: [&str; 3] = ["ncd", "drl", "plt"];

/// The extensions the window offers for the file of the cut round the board.
pub const ROUTE_EXTENSIONS: [&str; 2] = ["ncd", "plt"];

/// The extensions the window offers for the openings of the solder mask.
pub const MASK_OUT_EXTENSIONS: [&str; 3] = ["gts", "gbs", "gbr"];

/// What the window of files is asked for: a file to open or a file to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    OpenLayer,
    OpenDrills,
    OpenOutline,
    OpenMask,
    OpenMarks,
    ExportPaths,
    ExportDrills,
    ExportRoute,
    ExportMask,
}

impl Job {
    /// Whether a file is written (the window asks for a name) rather than opened.
    pub fn is_export(self) -> bool {
        matches!(
            self,
            Job::ExportPaths | Job::ExportDrills | Job::ExportRoute | Job::ExportMask
        )
    }

    /// The extensions the list of files shows first.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Job::OpenLayer => &LAYER_EXTENSIONS,
            Job::OpenDrills => &DRILL_EXTENSIONS,
            Job::OpenOutline => &OUTLINE_EXTENSIONS,
            Job::OpenMask => &MASK_EXTENSIONS,
            Job::OpenMarks => &MARKS_EXTENSIONS,
            Job::ExportPaths => &PATHS_EXTENSIONS,
            Job::ExportDrills => &DRILL_OUT_EXTENSIONS,
            Job::ExportRoute => &ROUTE_EXTENSIONS,
            Job::ExportMask => &MASK_OUT_EXTENSIONS,
        }
    }

    /// The title of the window of files.
    pub fn title(self) -> Msg {
        match self {
            Job::OpenLayer => Msg::OpenTitle,
            Job::OpenDrills => Msg::TitleDrills,
            Job::OpenOutline => Msg::TitleOutline,
            Job::OpenMask => Msg::TitleMask,
            Job::OpenMarks => Msg::TitleMarks,
            Job::ExportPaths => Msg::ExportTitle,
            Job::ExportDrills => Msg::TitleExportDrills,
            Job::ExportRoute => Msg::TitleExportRoute,
            Job::ExportMask => Msg::TitleExportMask,
        }
    }
}

/// The group of the window that a parameter belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Isolation,
    Drills,
    Outline,
    Layout,
    Mask,
}

impl Section {
    /// The title of the group.
    pub fn title(self) -> Msg {
        match self {
            Section::Isolation => Msg::Parameters,
            Section::Drills => Msg::SectionDrills,
            Section::Outline => Msg::SectionOutline,
            Section::Layout => Msg::SectionLayout,
            Section::Mask => Msg::SectionMask,
        }
    }
}

/// A parameter of the window: the setting it changes, the sentence that names it and the text being typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub section: Section,
    pub key: &'static str,
    pub label: Msg,
    pub text: String,
}

/// The parameters the window offers, in the order they are shown, with the values of the settings.
pub fn default_fields(s: &Settings) -> Vec<Field> {
    let f = |section: Section, key: &'static str, label: Msg, text: String| Field {
        section,
        key,
        label,
        text,
    };
    let iso = Section::Isolation;
    vec![
        f(
            iso,
            "fine_tool_mm",
            Msg::ToolDiameter,
            s.fine_tool_mm.to_string(),
        ),
        f(iso, "fine_passes", Msg::Passes, s.fine_passes.to_string()),
        f(
            iso,
            "fine_stepover_mm",
            Msg::Stepover,
            s.fine_stepover_mm.to_string(),
        ),
        f(iso, "depth_mm", Msg::Depth, s.depth_mm.to_string()),
        f(iso, "feed_mm_min", Msg::Feed, s.feed_mm_min.to_string()),
        f(
            iso,
            "plunge_mm_min",
            Msg::Plunge,
            s.plunge_mm_min.to_string(),
        ),
        f(iso, "safe_z_mm", Msg::SafeZ, s.safe_z_mm.to_string()),
        f(iso, "spindle_rpm", Msg::Spindle, s.spindle_rpm.to_string()),
        f(iso, "res_mm", Msg::Resolution, s.res_mm.to_string()),
        f(
            Section::Drills,
            "alignment_hole_mm",
            Msg::AlignHole,
            s.alignment_hole_mm.to_string(),
        ),
        f(
            Section::Drills,
            "alignment_margin_mm",
            Msg::AlignMargin,
            s.alignment_margin_mm.to_string(),
        ),
        f(
            Section::Drills,
            "alignment_holes",
            Msg::AlignCount,
            s.alignment_holes.to_string(),
        ),
        f(
            Section::Outline,
            "route_tool_mm",
            Msg::RouteTool,
            s.route_tool_mm.to_string(),
        ),
        f(
            Section::Outline,
            "tab_count",
            Msg::TabCount,
            s.tab_count.to_string(),
        ),
        f(
            Section::Outline,
            "tab_width_mm",
            Msg::TabWidth,
            s.tab_width_mm.to_string(),
        ),
        f(
            Section::Layout,
            "rotate_quarters",
            Msg::Quarters,
            s.rotate_quarters.to_string(),
        ),
        f(
            Section::Layout,
            "panel_cols",
            Msg::PanelCols,
            s.panel_cols.to_string(),
        ),
        f(
            Section::Layout,
            "panel_rows",
            Msg::PanelRows,
            s.panel_rows.to_string(),
        ),
        f(
            Section::Layout,
            "panel_gap_mm",
            Msg::PanelGap,
            s.panel_gap_mm.to_string(),
        ),
        f(
            Section::Layout,
            "scale_x",
            Msg::ScaleX,
            s.scale_x.to_string(),
        ),
        f(
            Section::Layout,
            "scale_y",
            Msg::ScaleY,
            s.scale_y.to_string(),
        ),
        f(
            Section::Mask,
            "mask_margin_mm",
            Msg::MaskMargin,
            s.mask_margin_mm.to_string(),
        ),
    ]
}

/// The units of a drill file that does not say them (or says them wrong).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrillUnits {
    /// The ones the file says.
    #[default]
    Auto,
    Mm,
    Inch,
}

impl DrillUnits {
    pub const ALL: [DrillUnits; 3] = [DrillUnits::Auto, DrillUnits::Mm, DrillUnits::Inch];

    /// The units for the reader, or `None` to take the ones of the file.
    pub fn units(self) -> Option<eh3d_parser_excellon::Units> {
        match self {
            DrillUnits::Auto => None,
            DrillUnits::Mm => Some(eh3d_parser_excellon::Units::Millimetres),
            DrillUnits::Inch => Some(eh3d_parser_excellon::Units::Inches),
        }
    }

    /// The word that names them.
    pub fn label(self) -> Msg {
        match self {
            DrillUnits::Auto => Msg::UnitsAuto,
            DrillUnits::Mm => Msg::UnitsMm,
            DrillUnits::Inch => Msg::UnitsInch,
        }
    }
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
    HolesOpened {
        name: String,
        holes: usize,
        tools: usize,
    },
    OutlineOpened {
        name: String,
        paths: usize,
        open_runs: usize,
    },
    MaskOpened {
        name: String,
        shapes: usize,
    },
    MarksLoaded(usize),
    NoMarks(String),
    NeedDrills,
    NeedOutline,
    /// Written, but some paths of the outline are too short for a bridge and are cut right round.
    ExportedLoose {
        file: String,
        paths: usize,
    },
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
            Status::HolesOpened { name, holes, tools } => {
                let (holes, tools) = (holes.to_string(), tools.to_string());
                fill(
                    Msg::HolesOpened.tr(lang),
                    &[name.as_str(), holes.as_str(), tools.as_str()],
                )
            }
            Status::OutlineOpened {
                name,
                paths,
                open_runs,
            } => {
                let paths = paths.to_string();
                let base = fill(
                    Msg::OutlineOpened.tr(lang),
                    &[name.as_str(), paths.as_str()],
                );
                if *open_runs > 0 {
                    let runs = open_runs.to_string();
                    format!(
                        "{base}; {}",
                        fill(Msg::NoteOpenRuns.tr(lang), &[runs.as_str()])
                    )
                } else {
                    base
                }
            }
            Status::MaskOpened { name, shapes } => fill(
                Msg::MaskOpened.tr(lang),
                &[name.as_str(), shapes.to_string().as_str()],
            ),
            Status::MarksLoaded(n) => fill(Msg::MarksLoaded.tr(lang), &[n.to_string().as_str()]),
            Status::NoMarks(name) => fill(Msg::NoMarks.tr(lang), &[name.as_str()]),
            Status::NeedDrills => Msg::NeedDrills.tr(lang).to_string(),
            Status::NeedOutline => Msg::NeedOutline.tr(lang).to_string(),
            Status::ExportedLoose { file, paths } => fill(
                Msg::ExportedLoose.tr(lang),
                &[file.as_str(), paths.to_string().as_str()],
            ),
        }
    }

    /// Whether the sentence is a problem or a warning (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        match self {
            Status::Ready
            | Status::Opened { .. }
            | Status::Computed { .. }
            | Status::Exported { .. }
            | Status::HolesOpened { .. }
            | Status::MaskOpened { .. }
            | Status::MarksLoaded(_) => false,
            Status::OutlineOpened { open_runs, .. } => *open_runs > 0,
            _ => true,
        }
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

/// What the picture shows: the copper, the holes and the outline, laid out as the parameters say (turned over, repeated on a panel).
#[derive(Debug, Clone, Default)]
pub struct View {
    pub drawing: Drawing,
    pub holes: Vec<Hole>,
    pub outline: Vec<Loop>,
}

/// What the reference points make of the work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarksFit {
    pub scale_x: f64,
    pub scale_y: f64,
    pub rotation_deg: f64,
    /// How far the farthest mark is from where the fit puts it.
    pub worst_mm: f64,
}

impl MarksFit {
    fn of(f: &crate::fiducial::Fit) -> MarksFit {
        MarksFit {
            scale_x: f.scale_x,
            scale_y: f.scale_y,
            rotation_deg: f.rotation_deg,
            worst_mm: f.residuals_mm.iter().copied().fold(0.0, f64::max),
        }
    }
}

/// The window.
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    /// Write although the tool does not fit between pieces of copper or the layer has copper that is not drawn.
    pub force: bool,
    pub layer: Option<Layer>,
    /// The holes of a drill file.
    pub drills: Option<DrillFile>,
    /// The outline of the board.
    pub outline: Option<OutlineFile>,
    /// A solder mask layer, when the openings are to follow it instead of the pads of the copper.
    pub mask: Option<Layer>,
    /// The rows of the table of reference points, as the settings read them.
    pub marks: Vec<String>,
    /// The file the reference points came from.
    pub marks_file: Option<PathBuf>,
    /// Turn the board over (the line it is turned on).
    pub mirror: Option<Axis>,
    /// Add the holes that hold the board when it is turned over to the drill file.
    pub align: bool,
    /// Put the cut round the board first in a plot of the paths.
    pub plot_outline: bool,
    /// The units of the drill file when it does not say them.
    pub drill_units: DrillUnits,
    pub passes: Vec<Pass>,
    /// The places where the tool does not fit and no path was laid.
    pub unfixed: usize,
    pub status: Status,
    /// What the readers did not understand or did not use, from the last file that was opened.
    pub notes: Vec<String>,
    /// The parameters changed after the paths were worked out.
    pub stale: bool,
    pub viewport: Viewport,
    pub view: View,
    /// What the reference points make of the work, when there are some.
    pub fit_note: Option<MarksFit>,
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
            drills: None,
            outline: None,
            mask: None,
            marks: Vec::new(),
            marks_file: None,
            mirror: None,
            align: false,
            plot_outline: false,
            drill_units: DrillUnits::Auto,
            passes: Vec::new(),
            unfixed: 0,
            status: Status::Ready,
            notes: Vec::new(),
            stale: false,
            viewport: Viewport::default(),
            view: View::default(),
            fit_note: None,
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
        s.mirror = self.mirror;
        s.plot_outline = self.plot_outline;
        for row in &self.marks {
            s.set("fiducial", row).map_err(Status::Failed)?;
        }
        s.validate().map_err(Status::Failed)?;
        Ok(s)
    }

    /// The settings, or the reason they cannot be used (which is then what the window says).
    fn ready_settings(&mut self) -> Option<Settings> {
        match self.settings() {
            Ok(s) => Some(s),
            Err(st) => {
                self.status = st;
                None
            }
        }
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

    /// Turns the board over on a line (or not at all); the paths shown are then out of date.
    pub fn set_mirror(&mut self, mirror: Option<Axis>) {
        if self.mirror != mirror {
            self.mirror = mirror;
            self.stale = true;
        }
    }

    /// Chooses the units of the drill file. A drill file that is open is read again with them.
    pub fn set_drill_units(&mut self, units: DrillUnits) {
        if self.drill_units == units {
            return;
        }
        self.drill_units = units;
        if let Some(path) = self.drills.as_ref().map(|d| d.path.clone()) {
            self.open_drills(&path);
        }
    }

    /// Whether there is anything to show: a layer, drills or an outline.
    pub fn has_work(&self) -> bool {
        self.layer.is_some() || self.drills.is_some() || self.outline.is_some()
    }

    /// The folder of the work: where the layer is, or else the drills or the outline.
    pub fn work_folder(&self) -> Option<PathBuf> {
        let paths = [
            self.layer.as_ref().map(|l| l.path.as_path()),
            self.drills.as_ref().map(|d| d.path.as_path()),
            self.outline.as_ref().map(|o| o.path.as_path()),
        ];
        paths
            .into_iter()
            .flatten()
            .filter_map(Path::parent)
            .find(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
    }

    /// The name of the file that is open for a job (none for the jobs that write).
    pub fn open_name(&self, job: Job) -> Option<String> {
        match job {
            Job::OpenLayer => self.layer.as_ref().map(|l| l.name.clone()),
            Job::OpenDrills => self.drills.as_ref().map(|d| d.name.clone()),
            Job::OpenOutline => self.outline.as_ref().map(|o| o.name.clone()),
            Job::OpenMask => self.mask.as_ref().map(|m| m.name.clone()),
            Job::OpenMarks => self.marks_file.as_deref().map(board::file_name),
            _ => None,
        }
    }

    /// The sentence about what the reference points make of the work, in the language of the person.
    pub fn marks_fit_text(&self) -> Option<String> {
        self.fit_note.map(|f| {
            let (sx, sy, rot, worst) = (
                format!("{:.5}", f.scale_x),
                format!("{:.5}", f.scale_y),
                format!("{:.4}", f.rotation_deg),
                format!("{:.3}", f.worst_mm),
            );
            fill(
                Msg::MarksFit.tr(self.lang),
                &[sx.as_str(), sy.as_str(), rot.as_str(), worst.as_str()],
            )
        })
    }

    /// What the picture shows before the board is laid out: the files as they were read.
    fn raw_view(&self) -> View {
        View {
            drawing: self
                .layer
                .as_ref()
                .map(|l| l.drawing.clone())
                .unwrap_or_default(),
            holes: self
                .drills
                .as_ref()
                .map(|d| d.holes.clone())
                .unwrap_or_default(),
            outline: self
                .outline
                .as_ref()
                .map(|o| o.closed.clone())
                .unwrap_or_default(),
        }
    }

    /// The box of the picture: the copper, the holes, the outline and the paths.
    fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        let (mut lo, mut hi) = self.view.drawing.extent().unwrap_or((
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ));
        let mut put = |x: f64, y: f64, pad: f64| {
            lo = (lo.0.min(x - pad), lo.1.min(y - pad));
            hi = (hi.0.max(x + pad), hi.1.max(y + pad));
        };
        for h in &self.view.holes {
            put(h.x_mm, h.y_mm, h.diameter_mm / 2.0);
        }
        for q in self.view.outline.iter().flatten() {
            put(q.0, q.1, 0.0);
        }
        for q in self
            .passes
            .iter()
            .flat_map(|p| p.paths.iter().chain(p.open.iter()))
            .flatten()
        {
            put(q.0, q.1, 0.0);
        }
        (lo.0 <= hi.0 && lo.1 <= hi.1).then_some((lo, hi))
    }

    /// Shows the whole drawing in a picture of this size.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((lo, hi)) = self.extent() {
            self.viewport = Viewport::fit(lo, hi, width_px, height_px, 24.0);
        }
    }

    /// Lays the copper, the holes and the outline out together as the parameters say. Returns the board and the box of the whole panel.
    fn lay_all(&self, s: &Settings) -> Result<(Board, Bounds), String> {
        let none = Scene::default();
        let scene = match &self.layer {
            Some(l) => &l.scene,
            None => &none,
        };
        let holes: &[Hole] = match &self.drills {
            Some(d) => &d.holes,
            None => &[],
        };
        let outline: &[Loop] = match &self.outline {
            Some(o) => &o.closed,
            None => &[],
        };
        board::lay(scene, holes, outline, s)
    }

    /// Keeps what the picture shows from a board that was laid out, and what the reference points make of it. Says why when the reference points cannot be used.
    fn show(&mut self, board: Board, whole: &Bounds, s: &Settings) -> Result<(), String> {
        let fit = s.transform(board::centre_of(whole));
        self.view = View {
            drawing: Drawing::new(&board.scene.shapes),
            holes: board.holes,
            outline: board.outline,
        };
        match fit {
            Ok((_, f)) => {
                self.fit_note = f.as_ref().map(MarksFit::of);
                Ok(())
            }
            Err(why) => {
                self.fit_note = None;
                Err(why)
            }
        }
    }

    /// Lays the files out again for the picture. Returns whether it could (the status says why not).
    pub fn refresh_view(&mut self) -> bool {
        if !self.has_work() {
            self.view = View::default();
            self.fit_note = None;
            return true;
        }
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        let (board, whole) = match self.lay_all(&settings) {
            Ok(b) => b,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        match self.show(board, &whole, &settings) {
            Ok(()) => {
                self.stale = false;
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// After a file was opened or taken away: the picture and, when the copper has paths, the paths are worked out again. `opened` is what the window says unless something is wrong.
    fn settle(&mut self, opened: Status) {
        self.view = self.raw_view();
        self.status = Status::Ready;
        let done = if self.layer.is_some() && !self.passes.is_empty() {
            self.compute()
        } else {
            self.refresh_view()
        };
        if done && !self.status.is_problem() {
            self.status = opened;
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
                self.view = self.raw_view();
                self.compute();
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens a drill file (Excellon) and shows its holes. It is refused when it is not a drill file.
    pub fn open_drills(&mut self, path: &Path) -> bool {
        match board::read_drill_file(path, &self.base, self.drill_units.units()) {
            Ok(d) => {
                let opened = Status::HolesOpened {
                    name: d.name.clone(),
                    holes: d.holes.len(),
                    tools: d.tools(),
                };
                self.notes = d.notes.clone();
                self.drills = Some(d);
                self.settle(opened);
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens the outline of the board (a Gerber outline layer, a DXF drawing or a plot file).
    pub fn open_outline(&mut self, path: &Path) -> bool {
        match board::read_outline_file(path, &self.base) {
            Ok(o) => {
                let opened = Status::OutlineOpened {
                    name: o.name.clone(),
                    paths: o.closed.len(),
                    open_runs: o.open_runs,
                };
                self.notes.clear();
                self.outline = Some(o);
                self.settle(opened);
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens a solder mask layer; the openings of the mask then follow it instead of the pads of the copper.
    pub fn open_mask(&mut self, path: &Path) -> bool {
        match read_layer(path, &self.base) {
            Ok(layer) => {
                self.status = Status::MaskOpened {
                    name: layer.name.clone(),
                    shapes: layer.scene.shapes.len(),
                };
                self.notes = layer.scene.notes.clone();
                self.mask = Some(layer);
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Reads a table of reference points (one pair on each row) and uses it. A file with no pairs, or with pairs that cannot be fitted, is refused.
    pub fn load_marks(&mut self, path: &Path) -> bool {
        let name = board::file_name(path);
        let text = match board::read_text(path, &self.base) {
            Ok(t) => t,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let rows: Vec<String> = board::mark_rows(&text)
            .into_iter()
            .map(str::to_string)
            .collect();
        if rows.is_empty() {
            self.status = Status::NoMarks(name);
            return false;
        }
        let mut probe = self.base.clone();
        for row in &rows {
            if let Err(why) = probe.set("fiducial", row) {
                self.status = Status::Failed(format!("{name}: {why}"));
                return false;
            }
        }
        if let Err(why) = probe.transform((0.0, 0.0)) {
            self.status = Status::Failed(format!("{name}: {why}"));
            return false;
        }
        let count = rows.len();
        self.marks = rows;
        self.marks_file = Some(path.to_path_buf());
        self.notes.clear();
        self.settle(Status::MarksLoaded(count));
        true
    }

    /// Opens a file by what it is: a drill file, an outline (a DXF drawing, a plot file or an outline layer) or else a copper layer.
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        let text = match board::read_text(path, &self.base) {
            Ok(t) => t,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        match filetype::detect(&text) {
            filetype::FileKind::Excellon => self.open_drills(path),
            filetype::FileKind::Hpgl | filetype::FileKind::Dxf => self.open_outline(path),
            _ if matches!(board::extension(path).as_str(), "gko" | "gm1" | "gml") => {
                self.open_outline(path)
            }
            _ => self.open(path),
        }
    }

    /// Takes the drill file away.
    pub fn clear_drills(&mut self) {
        self.drills = None;
        self.notes.clear();
        self.settle(Status::Ready);
    }

    /// Takes the outline away.
    pub fn clear_outline(&mut self) {
        self.outline = None;
        self.notes.clear();
        self.settle(Status::Ready);
    }

    /// Takes the solder mask layer away (the openings follow the pads of the copper again).
    pub fn clear_mask(&mut self) {
        self.mask = None;
        self.notes.clear();
        self.status = Status::Ready;
    }

    /// Takes the reference points away.
    pub fn clear_marks(&mut self) {
        self.marks.clear();
        self.marks_file = None;
        self.fit_note = None;
        self.notes.clear();
        self.settle(Status::Ready);
    }

    /// Does what a window of files was asked for with the file the person chose.
    pub fn run_job(&mut self, job: Job, path: &Path) -> bool {
        match job {
            Job::OpenLayer => self.open(path),
            Job::OpenDrills => self.open_drills(path),
            Job::OpenOutline => self.open_outline(path),
            Job::OpenMask => self.open_mask(path),
            Job::OpenMarks => self.load_marks(path),
            Job::ExportPaths => self.export(path),
            Job::ExportDrills => self.export_drills(path),
            Job::ExportRoute => self.export_route(path),
            Job::ExportMask => self.export_mask(path),
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

    /// Works out the paths of the layer with the parameters (and shows the board as it is laid out). Returns whether it could. With no layer only the picture is laid out again.
    pub fn compute(&mut self) -> bool {
        if !self.has_work() {
            self.status = Status::NoLayer;
            return false;
        }
        if self.layer.is_none() {
            self.passes.clear();
            self.unfixed = 0;
            return self.refresh_view();
        }
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        let (board, whole) = match self.lay_all(&settings) {
            Ok(b) => b,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let built = build(&board.scene, &settings).and_then(|(passes, unfixed)| {
            board::move_passes(&passes, &settings, &whole).map(|(moved, _)| (moved, unfixed))
        });
        let shown = self.show(board, &whole, &settings);
        match (built, shown) {
            (Ok((passes, unfixed)), Ok(())) => {
                self.passes = passes;
                self.unfixed = unfixed;
                self.stale = false;
                self.status = self.blocker().unwrap_or_else(|| self.computed_status());
                true
            }
            (Err(why), _) | (_, Err(why)) => {
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

    /// The name suggested for the file of a job that writes, next to the file it comes from; none when the file it comes from is not open.
    pub fn suggested_output_for(&self, job: Job) -> Option<PathBuf> {
        let beside = |path: &Path, tail: &str, ext: &str| {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "board".to_string());
            path.with_file_name(format!("{stem}{tail}.{ext}"))
        };
        match job {
            Job::ExportPaths => self.layer.as_ref().map(|l| l.path.with_extension("nc")),
            Job::ExportDrills => self
                .drills
                .as_ref()
                .map(|d| beside(d.path.as_path(), "-out", "ncd")),
            Job::ExportRoute => self
                .outline
                .as_ref()
                .map(|o| beside(o.path.as_path(), "-cut", "ncd")),
            Job::ExportMask => self
                .layer
                .as_ref()
                .map(|l| beside(l.path.as_path(), "-mask", "gts")),
            _ => None,
        }
    }

    /// The name suggested for the file of the paths: the name of the layer with `.nc`, next to it.
    pub fn suggested_output(&self) -> Option<PathBuf> {
        self.suggested_output_for(Job::ExportPaths)
    }

    /// The files the work reads: an output may not be one of them, or it would be destroyed.
    fn inputs(&self) -> Vec<PathBuf> {
        let mut v = Vec::new();
        v.extend(self.layer.as_ref().map(|l| l.path.clone()));
        v.extend(self.drills.as_ref().map(|d| d.path.clone()));
        v.extend(self.outline.as_ref().map(|o| o.path.clone()));
        v.extend(self.mask.as_ref().map(|m| m.path.clone()));
        v.extend(self.marks_file.clone());
        v
    }

    /// Whether a file may be written there: it may not be one of the files that are open. Says so when it may not.
    fn output_allowed(&mut self, out: &Path) -> bool {
        let inputs = self.inputs();
        let refs: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
        if board::is_input(out, &refs) {
            self.status = Status::SameFile;
            false
        } else {
            true
        }
    }

    /// Writes a file and says where.
    fn write_out(&mut self, out: &Path, text: String) -> bool {
        match fs::write(out, text) {
            Ok(()) => {
                self.status = Status::Exported {
                    file: board::file_name(out),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", out.display()));
                false
            }
        }
    }

    /// Writes the paths: G-code, or HP-GL for a name that ends in `.plt` and a DXF drawing for `.dxf`. They are worked out again first when the parameters changed.
    pub fn export(&mut self, out: &Path) -> bool {
        if self.layer.is_none() {
            self.status = Status::NoLayer;
            return false;
        }
        if (self.stale || self.passes.is_empty()) && !self.compute() {
            return false;
        }
        if !self.output_allowed(out) {
            return false;
        }
        if let Some(block) = self.blocker() {
            self.status = block;
            return false;
        }
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        match self.render_paths(&settings, out) {
            Ok(text) => self.write_out(out, text),
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// The text of the file of the paths for its name.
    fn render_paths(&self, s: &Settings, out: &Path) -> Result<String, String> {
        let Some(layer) = &self.layer else {
            return Err(String::new());
        };
        match board::extension(out).as_str() {
            "plt" => {
                let (laid, whole) = self.lay_all(s)?;
                board::isolation_plot(&self.passes, s, &whole, &laid.outline)
            }
            "dxf" => dxf::write(&self.passes),
            _ => {
                let g = write_in(&self.passes, &s.job(), &s.order(), s.output_units)?;
                Ok(format!("{}{g}", gcode_header(&layer.path, s)))
            }
        }
    }

    /// Writes the holes of the drill file as the command line does: turned, repeated on a panel and corrected as the parameters say, with the holes that hold the board when that is
    /// asked, as a drill file or (for a name that ends in `.plt`) as a plot with a pen for each drill.
    pub fn export_drills(&mut self, out: &Path) -> bool {
        if self.drills.is_none() {
            self.status = Status::NeedDrills;
            return false;
        }
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        if !self.output_allowed(out) {
            return false;
        }
        let (laid, whole) = match self.lay_all(&settings) {
            Ok(b) => b,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let plot = board::extension(out) == "plt";
        match board::drills_job(laid.holes, &whole, &settings, self.align, plot) {
            Ok(job) => self.write_out(out, job.text),
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Writes the cut round the board, with the bridges the parameters ask for, as a routing file or (for a name that ends in `.plt`) as a plot. A path too short for a bridge is
    /// said, because the piece would not be held.
    pub fn export_route(&mut self, out: &Path) -> bool {
        if self.outline.is_none() {
            self.status = Status::NeedOutline;
            return false;
        }
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        if !self.output_allowed(out) {
            return false;
        }
        let (laid, whole) = match self.lay_all(&settings) {
            Ok(b) => b,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let plot = board::extension(out) == "plt";
        let job = match board::route_job(&laid.outline, &whole, &settings, plot) {
            Ok(j) => j,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let loose = job.tabs.paths_left_closed;
        if !self.write_out(out, job.text) {
            return false;
        }
        if loose > 0 {
            self.status = Status::ExportedLoose {
                file: board::file_name(out),
                paths: loose,
            };
        }
        true
    }

    /// Writes the openings of the solder mask as a Gerber file: from the mask layer when one is open, else from the pads of the copper layer.
    pub fn export_mask(&mut self, out: &Path) -> bool {
        let Some(copper) = self.layer.as_ref().map(|l| l.scene.clone()) else {
            self.status = Status::NoLayer;
            return false;
        };
        let mask_scene = self.mask.as_ref().map(|m| m.scene.clone());
        let Some(settings) = self.ready_settings() else {
            return false;
        };
        if !self.output_allowed(out) {
            return false;
        }
        match board::mask_job(&copper, mask_scene.as_ref(), settings.mask_margin_mm) {
            Ok(job) => {
                self.notes = job.notes;
                self.write_out(out, job.text)
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }
}

/// Reads a Gerber layer the way the command line does: with the size limit, refusing what is not a Gerber file, and with the arc tolerance of the settings.
pub fn read_layer(path: &Path, settings: &Settings) -> Result<Layer, String> {
    let name = board::file_name(path);
    let text = board::read_text(path, settings)?;
    filetype::expect_gerber(&name, filetype::detect(&text))?;
    let text = settings
        .gerber_text(&text, None)
        .map_err(|e| format!("{name}: {e}"))?;
    let limits = InputLimits {
        max_bytes: board::input_limit(settings),
    };
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

/// The standard header of a G-code file: the generator and its version, the units, the file the work came from with its SHA-256 and the tools.
fn gcode_header(layer: &Path, s: &Settings) -> String {
    let named = fs::read(layer).ok().map(|bytes| sha256_hex(&bytes));
    let name = board::file_name(layer);
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

    const DRILLS: &str =
        "M48\nMETRIC,TZ\nT01C0.8\nT02C3.2\n%\nT01\nX1.0Y2.0\nX3.0Y2.0\nT02\nX5.0Y5.0\nM30\n";
    const OUTLINE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";

    fn set(w: &mut Window, key: &str, text: &str) {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    }

    /// A drill file and an outline in a scratch folder.
    fn board_files(dir: &Path) -> (PathBuf, PathBuf) {
        let d = dir.join("board.drl");
        let o = dir.join("board.gko");
        fs::write(&d, DRILLS).unwrap();
        fs::write(&o, OUTLINE).unwrap();
        (d, o)
    }

    fn min_x(passes: &[Pass]) -> f64 {
        passes
            .iter()
            .flat_map(|p| p.paths.iter().chain(p.open.iter()))
            .flatten()
            .map(|q| q.0)
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn the_parameters_are_in_groups_and_the_first_group_is_the_isolation() {
        let w = Window::new(Lang::En);
        assert!(w
            .fields
            .iter()
            .take(9)
            .all(|f| f.section == Section::Isolation));
        for s in [
            Section::Drills,
            Section::Outline,
            Section::Layout,
            Section::Mask,
        ] {
            assert!(w.fields.iter().any(|f| f.section == s), "{s:?}");
        }
        let keys: Vec<&str> = w.fields.iter().map(|f| f.key).collect();
        for (i, k) in keys.iter().enumerate() {
            assert!(!keys[i + 1..].contains(k), "{k} is listed twice");
        }
    }

    #[test]
    fn every_job_has_extensions_and_a_title_and_four_of_them_write() {
        let jobs = [
            Job::OpenLayer,
            Job::OpenDrills,
            Job::OpenOutline,
            Job::OpenMask,
            Job::OpenMarks,
            Job::ExportPaths,
            Job::ExportDrills,
            Job::ExportRoute,
            Job::ExportMask,
        ];
        for j in jobs {
            assert!(!j.extensions().is_empty(), "{j:?}");
            assert!(!j.title().tr(Lang::En).is_empty());
        }
        assert_eq!(jobs.iter().filter(|j| j.is_export()).count(), 4);
        assert!(Job::OpenDrills.extensions().contains(&"drl"));
        assert!(Job::OpenOutline.extensions().contains(&"dxf"));
    }

    #[test]
    fn opening_a_drill_file_shows_its_holes_and_says_how_many() {
        let dir = scratch("drills-open");
        let (d, _) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        assert_eq!(
            w.status,
            Status::HolesOpened {
                name: "board.drl".into(),
                holes: 3,
                tools: 2
            }
        );
        assert_eq!(w.status.text(Lang::En), "board.drl: 3 holes, 2 tools");
        assert_eq!(w.view.holes.len(), 3);
        assert!(w.has_work());
        w.fit(800.0, 600.0);
        assert!(w.viewport.px_per_mm > 0.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_is_not_a_drill_file_is_refused_and_a_drill_file_is_not_a_layer() {
        let dir = scratch("drills-refuse");
        let (d, _) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(!w.open_drills(&sample()));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(w.drills.is_none());
        assert!(!w.open(&d));
        assert!(w.layer.is_none());
        assert!(!w.open_drills(Path::new("/this/file/does/not/exist.drl")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_units_of_a_drill_file_can_be_given_and_the_file_is_read_again() {
        let dir = scratch("drills-units");
        let d = dir.join("small.drl");
        fs::write(
            &d,
            "M48
METRIC
T1C0.8
%
T1
X1.0Y1.0
M30
",
        )
        .unwrap();
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        let before = w.drills.as_ref().unwrap().holes[0].x_mm;
        assert!((before - 1.0).abs() < 1e-6);
        w.set_drill_units(DrillUnits::Inch);
        let after = w.drills.as_ref().unwrap().holes[0].x_mm;
        assert!((after - 25.4).abs() < 0.1, "{after}");
        assert!(DrillUnits::Auto.units().is_none());
        assert!(matches!(
            DrillUnits::Mm.units(),
            Some(eh3d_parser_excellon::Units::Millimetres)
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn opening_an_outline_shows_its_paths_and_a_file_without_one_is_refused() {
        let dir = scratch("outline-open");
        let (_, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_outline(&o));
        assert_eq!(
            w.status,
            Status::OutlineOpened {
                name: "board.gko".into(),
                paths: 1,
                open_runs: 0
            }
        );
        assert!(!w.status.is_problem());
        assert_eq!(w.view.outline.len(), 1);
        let junk = dir.join("junk.gko");
        fs::write(&junk, "nothing here\n").unwrap();
        assert!(!w.open_outline(&junk));
        assert!(matches!(w.status, Status::Failed(_)));
        assert_eq!(w.outline.as_ref().unwrap().name, "board.gko");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_outline_with_runs_that_do_not_close_says_so_in_every_language() {
        let st = Status::OutlineOpened {
            name: "a.gko".into(),
            paths: 1,
            open_runs: 3,
        };
        assert!(st.is_problem());
        for l in Lang::ALL {
            let t = st.text(l);
            assert!(t.contains("a.gko") && t.contains('3'), "{}: {t}", l.code());
        }
        let loose = Status::ExportedLoose {
            file: "a.ncd".into(),
            paths: 2,
        };
        assert!(loose.is_problem());
        for l in Lang::ALL {
            assert!(loose.text(l).contains("a.ncd"), "{}", l.code());
        }
    }

    #[test]
    fn a_file_dropped_on_the_window_is_opened_as_what_it_is() {
        let dir = scratch("dropped");
        let (d, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_dropped(&d));
        assert!(w.drills.is_some() && w.layer.is_none());
        assert!(w.open_dropped(&o));
        assert!(w.outline.is_some() && w.layer.is_none());
        assert!(w.open_dropped(&sample()));
        assert!(w.layer.is_some());
        assert!(!w.open_dropped(Path::new("/this/file/does/not/exist.gtl")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_drills_are_written_as_a_drill_file_that_reads_back() {
        let dir = scratch("drills-export");
        let (d, _) = board_files(&dir);
        let out = dir.join("out.ncd");
        let mut w = Window::new(Lang::En);
        assert!(!w.export_drills(&out));
        assert_eq!(w.status, Status::NeedDrills);
        assert!(w.open_drills(&d));
        assert!(w.export_drills(&out));
        assert_eq!(
            w.status,
            Status::Exported {
                file: "out.ncd".into()
            }
        );
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("M48"));
        assert_eq!(crate::drill::read_drills(&text).unwrap().holes.len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_drills_are_written_as_a_plot_for_a_plt_name() {
        let dir = scratch("drills-plot");
        let (d, _) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        assert!(w.export_drills(&dir.join("holes.plt")));
        assert!(fs::read_to_string(dir.join("holes.plt"))
            .unwrap()
            .contains("PD"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_alignment_holes_the_mirror_and_the_panel_change_the_drill_file() {
        let dir = scratch("drills-layout");
        let (d, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        assert!(w.open_outline(&o));
        w.align = true;
        let out = dir.join("aligned.ncd");
        assert!(w.export_drills(&out));
        let aligned = crate::drill::read_drills(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(aligned.holes.len(), 3 + 3);
        w.align = false;
        w.set_mirror(Some(Axis::Vertical));
        assert!(w.stale);
        let mirrored = dir.join("mirrored.ncd");
        assert!(w.export_drills(&mirrored));
        let back = crate::drill::read_drills(&fs::read_to_string(&mirrored).unwrap()).unwrap();
        assert!(back.holes.iter().any(|h| (h.x_mm - 49.0).abs() < 0.05));
        w.set_mirror(None);
        set(&mut w, "panel_cols", "2");
        let panel = dir.join("panel.ncd");
        assert!(w.export_drills(&panel));
        let wide = crate::drill::read_drills(&fs::read_to_string(&panel).unwrap()).unwrap();
        assert_eq!(wide.holes.len(), 6);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_drill_file_is_never_written_over_an_input() {
        let dir = scratch("drills-same");
        let (d, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        assert!(w.open_outline(&o));
        assert!(!w.export_drills(&d));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(fs::read_to_string(&d).unwrap(), DRILLS);
        assert!(!w.export_drills(&dir.join(".").join("board.gko")));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(fs::read_to_string(&o).unwrap(), OUTLINE);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cut_round_the_board_is_written_as_a_routing_file_or_a_plot() {
        let dir = scratch("route");
        let (_, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        set(&mut w, "res_mm", "0.05");
        assert!(!w.export_route(&dir.join("cut.ncd")));
        assert_eq!(w.status, Status::NeedOutline);
        assert!(w.open_outline(&o));
        assert!(w.export_route(&dir.join("cut.ncd")));
        assert!(fs::read_to_string(dir.join("cut.ncd"))
            .unwrap()
            .contains("M15"));
        assert!(w.export_route(&dir.join("cut.plt")));
        assert!(fs::read_to_string(dir.join("cut.plt"))
            .unwrap()
            .contains("PD"));
        assert!(!w.export_route(&o));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(fs::read_to_string(&o).unwrap(), OUTLINE);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bridges_open_the_cut_and_a_cutter_out_of_range_is_refused() {
        let dir = scratch("route-tabs");
        let (_, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        set(&mut w, "res_mm", "0.05");
        assert!(w.open_outline(&o));
        set(&mut w, "tab_count", "2");
        assert!(w.export_route(&dir.join("tabs.ncd")));
        assert_eq!(
            w.status,
            Status::Exported {
                file: "tabs.ncd".into()
            }
        );
        set(&mut w, "route_tool_mm", "50");
        assert!(!w.export_route(&dir.join("big.ncd")));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(!dir.join("big.ncd").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_openings_of_the_mask_follow_the_pads_or_the_mask_layer() {
        let dir = scratch("mask");
        let mut w = Window::new(Lang::En);
        assert!(!w.export_mask(&dir.join("m.gts")));
        assert_eq!(w.status, Status::NoLayer);
        assert!(w.open(&sample()));
        assert!(w.export_mask(&dir.join("pads.gts")));
        assert!(fs::read_to_string(dir.join("pads.gts"))
            .unwrap()
            .contains("D03"));
        let mask = dir.join("mask.gts");
        fs::copy(sample(), &mask).unwrap();
        assert!(w.open_mask(&mask));
        assert!(matches!(w.status, Status::MaskOpened { .. }));
        assert!(w.export_mask(&dir.join("layer.gts")));
        assert!(!w.export_mask(&mask));
        assert_eq!(w.status, Status::SameFile);
        assert!(!w.open_mask(&dir.join("missing.gts")));
        w.clear_mask();
        assert!(w.mask.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_reference_points_shift_the_paths_and_can_be_taken_away() {
        let dir = scratch("marks");
        let table = dir.join("marks.csv");
        fs::write(&table, "dx,dy,mx,my\n# a note\n-5,0,-4.9,0\n5,0,5.1,0\n").unwrap();
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        let before = min_x(&w.passes);
        assert!(w.load_marks(&table));
        assert_eq!(w.status, Status::MarksLoaded(2));
        assert_eq!(w.open_name(Job::OpenMarks).as_deref(), Some("marks.csv"));
        assert!(w.marks_fit_text().unwrap().contains("1.00000"));
        assert!((min_x(&w.passes) - before - 0.1).abs() < 1e-6);
        // the table is an input: nothing is written over it
        assert!(!w.export(&table));
        assert_eq!(w.status, Status::SameFile);
        w.clear_marks();
        assert!(w.marks.is_empty() && w.fit_note.is_none());
        assert!((min_x(&w.passes) - before).abs() < 1e-6);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_table_without_pairs_or_with_one_pair_is_refused() {
        let dir = scratch("marks-bad");
        let empty = dir.join("empty.csv");
        fs::write(&empty, "dx,dy,mx,my\n").unwrap();
        let one = dir.join("one.csv");
        fs::write(&one, "1,2,1.1,2.1\n").unwrap();
        let mut w = Window::new(Lang::En);
        assert!(!w.load_marks(&empty));
        assert_eq!(w.status, Status::NoMarks("empty.csv".into()));
        assert!(!w.load_marks(&one));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(!w.load_marks(Path::new("/this/file/does/not/exist.csv")));
        assert!(w.marks.is_empty() && w.marks_file.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_panel_and_the_mirror_are_laid_out_in_the_picture_and_in_the_paths() {
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        let single = w.passes.len();
        let shapes = w.view.drawing.shapes.clone();
        assert!(!shapes.is_empty());
        w.set_mirror(Some(Axis::Vertical));
        assert!(w.stale);
        assert!(w.compute());
        assert_ne!(w.view.drawing.shapes, shapes);
        assert_eq!(w.view.drawing.shapes.len(), shapes.len());
        w.set_mirror(None);
        set(&mut w, "panel_cols", "2");
        assert!(w.stale);
        assert!(w.compute());
        assert_eq!(w.view.drawing.shapes.len(), 2 * shapes.len());
        assert_eq!(w.passes.len(), single);
    }

    #[test]
    fn a_value_of_the_layout_that_is_not_valid_names_its_parameter_even_with_no_layer() {
        let dir = scratch("layout-bad");
        let (d, _) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.open_drills(&d));
        set(&mut w, "rotate_quarters", "7");
        assert!(!w.compute());
        match &w.status {
            Status::BadValue { label, .. } => assert_eq!(*label, Msg::Quarters),
            other => panic!("{other:?}"),
        }
        set(&mut w, "rotate_quarters", "1");
        assert!(w.compute());
        assert!(w.passes.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_names_suggested_for_the_files_follow_the_files_that_are_open() {
        let dir = scratch("suggest");
        let (d, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        assert!(w.suggested_output_for(Job::ExportDrills).is_none());
        assert!(w.suggested_output().is_none());
        assert!(w.work_folder().is_none());
        assert!(w.open_drills(&d));
        assert!(w.open_outline(&o));
        assert_eq!(w.work_folder().as_deref(), Some(dir.as_path()));
        let name = |j: Job| {
            w.suggested_output_for(j)
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        };
        assert_eq!(name(Job::ExportDrills).as_deref(), Some("board-out.ncd"));
        assert_eq!(name(Job::ExportRoute).as_deref(), Some("board-cut.ncd"));
        assert_eq!(name(Job::ExportMask), None);
        assert_eq!(name(Job::OpenLayer), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_window_of_files_does_what_it_was_asked_for() {
        let dir = scratch("jobs");
        let (d, o) = board_files(&dir);
        let mut w = Window::new(Lang::En);
        set(&mut w, "res_mm", "0.05");
        assert!(w.run_job(Job::OpenLayer, &sample()));
        assert!(w.run_job(Job::OpenDrills, &d));
        assert!(w.run_job(Job::OpenOutline, &o));
        assert!(w.run_job(Job::ExportPaths, &dir.join("p.nc")));
        assert!(w.run_job(Job::ExportDrills, &dir.join("d.ncd")));
        assert!(w.run_job(Job::ExportRoute, &dir.join("r.ncd")));
        assert!(w.run_job(Job::ExportMask, &dir.join("m.gts")));
        assert_eq!(w.open_name(Job::OpenDrills).as_deref(), Some("board.drl"));
        assert_eq!(w.open_name(Job::ExportMask), None);
        w.clear_drills();
        w.clear_outline();
        assert!(w.drills.is_none() && w.outline.is_none());
        assert!(w.view.holes.is_empty() && w.view.outline.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
