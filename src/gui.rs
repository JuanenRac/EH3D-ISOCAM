//! The window itself (built with the `gui` feature): it draws the state of `window::Window` and passes on what the person does. Everything that can be decided without a screen is
//! decided there, in code that is tested.

use crate::browse::{self, Browser};
use crate::lang::{Lang, Msg};
use crate::transform::Axis;
use crate::window::{DrillUnits, Job, Section, Status, Window};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;

// the colours of the theme Forge, the one the other programs of the suite start with: amber on charcoal
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0d, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
const COPPER: Color32 = Color32::from_rgb(0xc8, 0xa3, 0x3c);
const HOLE: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xf0);
const OUTLINE: Color32 = Color32::from_rgb(0x4d, 0xd0, 0xe1);

/// The colours of the passes, the same as the SVG preview of the command line.
const PASS_COLOURS: [Color32; 8] = [
    Color32::from_rgb(0xd9, 0x5f, 0x02),
    Color32::from_rgb(0x1b, 0x9e, 0x77),
    Color32::from_rgb(0x75, 0x70, 0xb3),
    Color32::from_rgb(0xe7, 0x29, 0x8a),
    Color32::from_rgb(0x66, 0xa6, 0x1e),
    Color32::from_rgb(0xe6, 0xab, 0x02),
    Color32::from_rgb(0xa6, 0x76, 0x1d),
    Color32::from_rgb(0x1f, 0x78, 0xb4),
];

enum Outcome {
    Cancel,
    Pick(PathBuf),
}

/// The window that opens a file or asks for the name of the file to write.
struct Dialog {
    browser: Browser,
    /// What the file is for.
    job: Job,
    /// The name being typed when the window asks for a file to write.
    export: Option<String>,
}

struct App {
    win: Window,
    dialog: Option<Dialog>,
    fit_pending: bool,
}

impl App {
    fn new(lang: Lang, initial: Option<PathBuf>) -> App {
        let mut win = Window::new(lang);
        let mut fit_pending = false;
        if let Some(p) = initial {
            fit_pending = win.open(&p);
        }
        App {
            win,
            dialog: None,
            fit_pending,
        }
    }

    fn folder(&self) -> PathBuf {
        self.win.work_folder().unwrap_or_else(browse::start_dir)
    }

    /// Opens the window of files for a job: the list of files to open, or the name of the file to write.
    fn ask(&mut self, job: Job) {
        let export = job.is_export().then(|| {
            self.win
                .suggested_output_for(job)
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| default_name(job).to_string())
        });
        self.dialog = Some(Dialog {
            browser: Browser::new(self.folder(), job.extensions()),
            job,
            export,
        });
    }

    /// Takes away the file that is open for a job.
    fn clear(&mut self, job: Job) {
        match job {
            Job::OpenDrills => self.win.clear_drills(),
            Job::OpenOutline => self.win.clear_outline(),
            Job::OpenMask => self.win.clear_mask(),
            Job::OpenMarks => self.win.clear_marks(),
            _ => {}
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("EH3D-ISOCAM")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                if ui.button(Msg::Open.tr(lang)).clicked() {
                    self.ask(Job::OpenLayer);
                }
                let has_layer = self.win.layer.is_some();
                let has_work = self.win.has_work();
                if ui
                    .add_enabled(has_work, egui::Button::new(Msg::Compute.tr(lang)))
                    .clicked()
                {
                    self.win.compute();
                }
                if ui
                    .add_enabled(has_layer, egui::Button::new(Msg::Export.tr(lang)))
                    .clicked()
                {
                    self.ask(Job::ExportPaths);
                }
                if ui
                    .add_enabled(has_work, egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.win.lang.name())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.win.lang, l, l.name());
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("parameters")
            .default_size(290.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    self.isolation_group(ui);
                    self.drills_group(ui);
                    self.outline_group(ui);
                    self.layout_group(ui);
                    self.marks_group(ui);
                    self.mask_group(ui);
                    self.notes_group(ui);
                });
            });
    }

    /// The title of a group of the side panel.
    fn heading(&self, msg: Msg) -> egui::RichText {
        egui::RichText::new(msg.tr(self.win.lang))
            .strong()
            .color(ACCENT)
    }

    /// The parameters of one group, each with its name above a box to type in.
    fn fields_of(&mut self, ui: &mut egui::Ui, section: Section) {
        let lang = self.win.lang;
        for i in 0..self.win.fields.len() {
            if self.win.fields[i].section != section {
                continue;
            }
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
            if ui.add(edit).changed() {
                self.win.set_field(i, text);
            }
        }
    }

    /// The buttons that open and take away a file, and its name.
    fn file_row(&mut self, ui: &mut egui::Ui, job: Job, open: Msg) {
        let lang = self.win.lang;
        let name = self.win.open_name(job);
        ui.horizontal(|ui| {
            if ui.button(open.tr(lang)).clicked() {
                self.ask(job);
            }
            if ui
                .add_enabled(name.is_some(), egui::Button::new(Msg::Clear.tr(lang)))
                .clicked()
            {
                self.clear(job);
            }
        });
        let shown = name.unwrap_or_else(|| Msg::NoFile.tr(lang).to_string());
        ui.label(egui::RichText::new(shown).color(DIM));
    }

    /// A button that asks for the name of a file to write.
    fn export_button(&mut self, ui: &mut egui::Ui, job: Job, label: Msg, enabled: bool) {
        let lang = self.win.lang;
        if ui
            .add_enabled(enabled, egui::Button::new(label.tr(lang)))
            .clicked()
        {
            self.ask(job);
        }
    }

    fn isolation_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Section::Isolation.title()))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Isolation);
                ui.add_space(8.0);
                let mut force = self.win.force;
                if ui.checkbox(&mut force, Msg::Force.tr(lang)).changed() {
                    self.win.set_force(force);
                }
            });
    }

    fn drills_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Section::Drills.title()))
            .default_open(true)
            .show(ui, |ui| {
                self.file_row(ui, Job::OpenDrills, Msg::OpenDrills);
                ui.label(Msg::DrillUnits.tr(lang));
                let mut units = self.win.drill_units;
                egui::ComboBox::from_id_salt("drill_units")
                    .selected_text(units.label().tr(lang))
                    .show_ui(ui, |ui| {
                        for u in DrillUnits::ALL {
                            ui.selectable_value(&mut units, u, u.label().tr(lang));
                        }
                    });
                if units != self.win.drill_units {
                    self.win.set_drill_units(units);
                }
                self.fields_of(ui, Section::Drills);
                let mut align = self.win.align;
                if ui
                    .checkbox(&mut align, Msg::AddAlignment.tr(lang))
                    .changed()
                {
                    self.win.align = align;
                }
                let has = self.win.drills.is_some();
                self.export_button(ui, Job::ExportDrills, Msg::ExportDrills, has);
            });
    }

    fn outline_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Section::Outline.title()))
            .default_open(true)
            .show(ui, |ui| {
                self.file_row(ui, Job::OpenOutline, Msg::OpenOutline);
                self.fields_of(ui, Section::Outline);
                let mut plot_outline = self.win.plot_outline;
                if ui
                    .checkbox(&mut plot_outline, Msg::PlotOutline.tr(lang))
                    .changed()
                {
                    self.win.plot_outline = plot_outline;
                }
                let has = self.win.outline.is_some();
                self.export_button(ui, Job::ExportRoute, Msg::ExportRoute, has);
            });
    }

    fn layout_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Section::Layout.title()))
            .default_open(false)
            .show(ui, |ui| {
                ui.label(Msg::Mirror.tr(lang));
                let mut mirror = self.win.mirror;
                let shown = match mirror {
                    None => Msg::MirrorNone,
                    Some(Axis::Vertical) => Msg::MirrorVertical,
                    Some(Axis::Horizontal) => Msg::MirrorHorizontal,
                };
                egui::ComboBox::from_id_salt("mirror")
                    .selected_text(shown.tr(lang))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut mirror, None, Msg::MirrorNone.tr(lang));
                        ui.selectable_value(
                            &mut mirror,
                            Some(Axis::Vertical),
                            Msg::MirrorVertical.tr(lang),
                        );
                        ui.selectable_value(
                            &mut mirror,
                            Some(Axis::Horizontal),
                            Msg::MirrorHorizontal.tr(lang),
                        );
                    });
                if mirror != self.win.mirror {
                    self.win.set_mirror(mirror);
                }
                self.fields_of(ui, Section::Layout);
            });
    }

    fn marks_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionMarks))
            .default_open(false)
            .show(ui, |ui| {
                self.file_row(ui, Job::OpenMarks, Msg::LoadMarks);
                ui.label(egui::RichText::new(Msg::MarksHint.tr(lang)).color(DIM));
                if let Some(text) = self.win.marks_fit_text() {
                    ui.label(egui::RichText::new(text).color(OK));
                }
            });
    }

    fn mask_group(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.heading(Section::Mask.title()))
            .default_open(false)
            .show(ui, |ui| {
                self.file_row(ui, Job::OpenMask, Msg::OpenMask);
                self.fields_of(ui, Section::Mask);
                let has = self.win.layer.is_some();
                self.export_button(ui, Job::ExportMask, Msg::ExportMask, has);
            });
    }

    /// What the readers did not understand or did not use, from the last file that was opened.
    fn notes_group(&self, ui: &mut egui::Ui) {
        for note in &self.win.notes {
            ui.label(egui::RichText::new(note.as_str()).color(DIM).small());
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let (text, problem) = if self.win.stale {
                    (Status::Stale.text(lang), true)
                } else {
                    (self.win.status.text(lang), self.win.status.is_problem())
                };
                ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                });
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
            painter.rect_filled(rect, 0.0, BACKGROUND);
            if !self.win.has_work() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            }
            if self.fit_pending && w > 40.0 && h > 40.0 {
                self.win.fit(w, h);
                self.fit_pending = false;
            }
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.win.viewport.pan_px(f64::from(d.x), f64::from(d.y));
            }
            if let Some(p) = response.hover_pos() {
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
                if (factor - 1.0).abs() > 1e-9 {
                    let at = p - rect.min;
                    self.win
                        .viewport
                        .zoom_at(f64::from(at.x), f64::from(at.y), w, h, factor);
                }
            }
            self.paint(&painter, rect);
        });
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let vp = self.win.viewport;
        let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
        let scale = vp.px_per_mm;
        let at = |x: f64, y: f64| -> Pos2 {
            let (px, py) = vp.to_screen(x, y, w, h);
            Pos2::new(rect.min.x + px as f32, rect.min.y + py as f32)
        };
        let px = |mm: f64| (mm * scale) as f32;
        let view = &self.win.view;
        let painter = painter.with_clip_rect(rect);
        for (shape, tris) in view.drawing.shapes.iter().zip(&view.drawing.tris) {
            use eh3d_gerberviewer::scene::Shape;
            match shape {
                Shape::Disc { x, y, r } => {
                    painter.circle_filled(at(*x, *y), px(*r).max(0.6), COPPER);
                }
                Shape::Rect { x, y, w, h } => {
                    let size = Vec2::new(px(*w).max(1.0), px(*h).max(1.0));
                    painter.rect_filled(Rect::from_center_size(at(*x, *y), size), 0.0, COPPER);
                }
                Shape::Obround { x, y, w, h } => {
                    let size = Vec2::new(px(*w).max(1.0), px(*h).max(1.0));
                    let round = px(w.min(*h)) / 2.0;
                    painter.rect_filled(Rect::from_center_size(at(*x, *y), size), round, COPPER);
                }
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    let width = px(*w).max(1.0);
                    let (a, b) = (at(*x0, *y0), at(*x1, *y1));
                    painter.line_segment([a, b], Stroke::new(width, COPPER));
                    painter.circle_filled(a, width / 2.0, COPPER);
                    painter.circle_filled(b, width / 2.0, COPPER);
                }
                Shape::Region { pts } => {
                    if tris.is_empty() {
                        let line: Vec<Pos2> = pts.iter().map(|p| at(p.0, p.1)).collect();
                        painter.add(egui::Shape::closed_line(line, Stroke::new(1.0, COPPER)));
                    } else {
                        let mut mesh = egui::epaint::Mesh::default();
                        for p in pts {
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: at(p.0, p.1),
                                uv: egui::epaint::WHITE_UV,
                                color: COPPER,
                            });
                        }
                        for t in tris {
                            mesh.add_triangle(t[0], t[1], t[2]);
                        }
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }
            }
        }
        for path in &view.outline {
            let line: Vec<Pos2> = path.iter().map(|p| at(p.0, p.1)).collect();
            painter.add(egui::Shape::closed_line(line, Stroke::new(1.5, OUTLINE)));
        }
        for (k, pass) in self.win.passes.iter().enumerate() {
            let colour = PASS_COLOURS[k % PASS_COLOURS.len()];
            let width = px(pass.diameter_mm).max(1.5);
            let band = Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), 110);
            for path in &pass.paths {
                let line: Vec<Pos2> = path.iter().map(|p| at(p.0, p.1)).collect();
                painter.add(egui::Shape::closed_line(
                    line.clone(),
                    Stroke::new(width, band),
                ));
                painter.add(egui::Shape::closed_line(line, Stroke::new(1.0, colour)));
            }
            for path in &pass.open {
                let line: Vec<Pos2> = path.iter().map(|p| at(p.0, p.1)).collect();
                painter.add(egui::Shape::line(line.clone(), Stroke::new(width, band)));
                painter.add(egui::Shape::line(line, Stroke::new(1.0, colour)));
            }
        }
        // the holes are drawn over the paths
        for h in &view.holes {
            let centre = at(h.x_mm, h.y_mm);
            let r = px(h.diameter_mm / 2.0).max(2.0);
            painter.circle_filled(centre, r, BACKGROUND);
            painter.circle_stroke(centre, r, Stroke::new(1.5, HOLE));
        }
        // the key of the colours
        let lang = self.win.lang;
        let mut y = rect.min.y + 12.0;
        let mut key = |colour: Color32, text: &str| {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(rect.min.x + 12.0, y), Vec2::new(14.0, 14.0)),
                2.0,
                colour,
            );
            painter.text(
                Pos2::new(rect.min.x + 32.0, y + 7.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(13.0),
                TEXT,
            );
            y += 20.0;
        };
        if self.win.layer.is_some() {
            key(COPPER, Msg::LegendCopper.tr(lang));
        }
        if !self.win.passes.is_empty() {
            key(PASS_COLOURS[0], Msg::LegendPaths.tr(lang));
        }
        if !view.holes.is_empty() {
            key(HOLE, Msg::LegendHoles.tr(lang));
        }
        if !view.outline.is_empty() {
            key(OUTLINE, Msg::LegendOutline.tr(lang));
        }
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match show_dialog(ctx, &mut dialog, self.win.lang) {
            None => self.dialog = Some(dialog),
            Some(Outcome::Cancel) => {}
            Some(Outcome::Pick(path)) => {
                if self.win.run_job(dialog.job, &path) {
                    if matches!(
                        dialog.job,
                        Job::OpenLayer | Job::OpenDrills | Job::OpenOutline
                    ) {
                        self.fit_pending = true;
                    }
                } else {
                    // the window stays open so that another file or name can be tried
                    self.dialog = Some(dialog);
                }
            }
        }
    }
}

/// Draws the window that opens a file (or asks for the name of the file to write) and says what the person chose.
fn show_dialog(ctx: &egui::Context, d: &mut Dialog, lang: Lang) -> Option<Outcome> {
    let mut outcome = None;
    let mut open = true;
    egui::Window::new(d.job.title().tr(lang))
        .open(&mut open)
        .collapsible(false)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(Msg::Up.tr(lang)).clicked() {
                    d.browser.up();
                }
                let edit = egui::TextEdit::singleline(&mut d.browser.path_text)
                    .desired_width(ui.available_width());
                let response = ui.add(edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(p) = d.browser.go_typed() {
                        if d.export.is_none() {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                }
            });
            if let Some(e) = &d.browser.error {
                ui.colored_label(DANGER, e.as_str());
            }
            if ui
                .checkbox(&mut d.browser.show_all, Msg::AllFiles.tr(lang))
                .changed()
            {
                d.browser.refresh();
            }
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if d.browser.entries.is_empty() {
                        ui.label(egui::RichText::new(Msg::EmptyFolder.tr(lang)).color(DIM));
                    }
                    for e in d.browser.entries.clone() {
                        let label = if e.is_dir {
                            format!("{}/", e.name)
                        } else {
                            format!("{}    ({})", e.name, browse::size_text(e.size))
                        };
                        if !ui.selectable_label(false, label).clicked() {
                            continue;
                        }
                        if e.is_dir {
                            d.browser.enter(&e.name);
                        } else if d.export.is_some() {
                            d.export = Some(e.name.clone());
                        } else if let Some(p) = d.browser.enter(&e.name) {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(name) = d.export.as_mut() {
                    ui.label(Msg::FileName.tr(lang));
                    ui.add(egui::TextEdit::singleline(name).desired_width(260.0));
                    if ui.button(Msg::Export.tr(lang)).clicked() && !name.trim().is_empty() {
                        outcome = Some(Outcome::Pick(d.browser.dir.join(name.trim())));
                    }
                }
                if ui.button(Msg::Cancel.tr(lang)).clicked() {
                    outcome = Some(Outcome::Cancel);
                }
            });
        });
    if !open && outcome.is_none() {
        outcome = Some(Outcome::Cancel);
    }
    outcome
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // a file dropped on the window is opened
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(p) = dropped.first() {
            if self.win.open_dropped(p) {
                self.fit_pending = true;
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.ask(Job::OpenLayer);
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.side_panel(ui);
        self.canvas(ui);
        self.dialog_window(&ctx);
    }
}

/// The name offered for a file to write when nothing is open to take it from.
fn default_name(job: Job) -> &'static str {
    match job {
        Job::ExportDrills | Job::ExportRoute => "output.ncd",
        Job::ExportMask => "mask.gts",
        _ => "paths.nc",
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. A layer given here is opened at once.
pub fn run(initial: Option<PathBuf>, lang: Lang) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("EH3D-ISOCAM");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-ISOCAM",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(lang, initial)))
        }),
    )
}
