//! The editor: document + history + view + tool state, and the top-level window layout.

use std::path::{Path, PathBuf};

use egui::{Frame, Panel, Ui};
use paint_core::shape::ShapeKind;
use paint_core::tool::{BrushKind, SelectionKind, Tool};
use paint_core::{io, zoom, Document, History, Rgba};

use crate::actions::{Action, Keymap};
use crate::canvas::gpu::CanvasGpu;
use crate::canvas::mips::MipChain;
use crate::canvas::{self, CanvasDraw, CanvasView};
use crate::i18n::Tr;
use crate::settings::Settings;
use crate::theme::{self, Palette, ThemeMode};

pub const MENU_BAR_H: f32 = 40.0;
pub const RIBBON_H: f32 = 108.0;
pub const STATUS_H: f32 = 38.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSlot {
    One,
    Two,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pending {
    New,
    Open(Option<PathBuf>),
    Exit,
}

#[derive(Clone, Debug)]
pub enum Modal {
    /// "Keep your changes?" before `then`.
    SaveChanges {
        then: Pending,
    },
    Message {
        title: String,
        text: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Editor,
    Settings,
}

pub struct Editor {
    pub settings: Settings,
    pub(crate) keymap: Keymap,
    pub(crate) doc: Document,
    pub(crate) history: History,
    pub(crate) saved_state: u64,
    pub(crate) path: Option<PathBuf>,
    pub(crate) file_size: Option<u64>,

    pub(crate) view: CanvasView,
    pub(crate) mips: Option<MipChain>,
    pub(crate) gpu: Option<CanvasGpu>,
    pub(crate) size_generation: Option<u64>,

    pub(crate) tool: Tool,
    pub(crate) brush: BrushKind,
    pub(crate) shape: ShapeKind,
    pub(crate) selection_kind: SelectionKind,
    pub(crate) transparent_selection: bool,
    pub(crate) color1: Rgba,
    pub(crate) color2: Rgba,
    pub(crate) active_slot: ColorSlot,
    pub(crate) custom_colors: [Option<Rgba>; 10],
    pub(crate) shapes_scroll: usize,
    pub(crate) layers_panel: bool,

    pub(crate) page: Page,
    pub(crate) modal: Option<Modal>,
    pub(crate) hover_pixel: Option<(i32, i32)>,
    pub(crate) zoom_text: Option<String>,
    pub(crate) applied_theme: Option<bool>,
    /// Set when the app should close the window (after the user agreed).
    pub(crate) exit_requested: bool,
    pub(crate) fullscreen: bool,
}

impl Editor {
    /// `render_state` is `None` when the wgpu renderer is not available (no canvas drawing).
    pub fn new(settings: Settings, render_state: Option<&egui_wgpu::RenderState>) -> Self {
        let tr = settings.language.tr();
        let (w, h) = settings.last_canvas_size;
        let doc = Document::new(w.max(1), h.max(1), format!("{} 1", tr.layer_n));
        let keymap = Keymap::new(&settings.keymap);
        Self {
            keymap,
            doc,
            history: History::default(),
            saved_state: 0,
            path: None,
            file_size: None,
            view: CanvasView::default(),
            mips: None,
            gpu: render_state.map(CanvasGpu::new),
            size_generation: None,
            tool: Tool::default(),
            brush: BrushKind::default(),
            shape: ShapeKind::Rectangle,
            selection_kind: SelectionKind::default(),
            transparent_selection: false,
            color1: Rgba::BLACK,
            color2: Rgba::WHITE,
            active_slot: ColorSlot::One,
            custom_colors: [None; 10],
            shapes_scroll: 0,
            layers_panel: false,
            page: Page::Editor,
            modal: None,
            hover_pixel: None,
            zoom_text: None,
            applied_theme: None,
            exit_requested: false,
            fullscreen: false,
            settings,
        }
    }

    pub fn tr(&self) -> &'static Tr {
        self.settings.language.tr()
    }

    pub fn is_modified(&self) -> bool {
        self.history.state_id() != self.saved_state
    }

    pub fn document_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.tr().untitled.to_owned())
    }

    /// Window title: "*name — PaintRS" (the star marks unsaved changes).
    pub fn title(&self) -> String {
        let star = if self.is_modified() { "*" } else { "" };
        format!("{star}{} — {}", self.document_name(), self.tr().app_name)
    }

    /// True once the user agreed to close (the app then closes the window).
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Called when the window's close button was pressed.
    pub fn request_exit(&mut self) {
        self.run_or_ask(Pending::Exit);
    }

    /// Opens a file given on the command line or dropped onto the window.
    pub fn open_path(&mut self, path: PathBuf) {
        self.run_or_ask(Pending::Open(Some(path)));
    }

    pub(crate) fn palette(&self, ctx: &egui::Context) -> &'static Palette {
        Palette::for_dark(ctx.global_style().visuals.dark_mode)
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let dark = match self.settings.theme {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => ctx.input(|i| i.raw.system_theme) != Some(egui::Theme::Light),
        };
        if self.applied_theme != Some(dark) {
            theme::apply(ctx, Palette::for_dark(dark));
            self.applied_theme = Some(dark);
        }
    }

    // ---------------------------------------------------------------- documents

    pub(crate) fn run_or_ask(&mut self, action: Pending) {
        if self.is_modified() {
            self.modal = Some(Modal::SaveChanges { then: action });
        } else {
            self.run_pending(action);
        }
    }

    pub(crate) fn run_pending(&mut self, action: Pending) {
        match action {
            Pending::New => self.new_document(),
            Pending::Open(path) => {
                let path = path.or_else(|| self.pick_open_path());
                if let Some(path) = path {
                    self.load(&path);
                }
            }
            Pending::Exit => self.exit_requested = true,
        }
    }

    fn new_document(&mut self) {
        let tr = self.tr();
        let (w, h) = self.settings.last_canvas_size;
        self.doc = Document::new(w.max(1), h.max(1), format!("{} 1", tr.layer_n));
        self.reset_after_load();
        self.path = None;
        self.file_size = None;
        // The original resets colors 1/2 on "New".
        self.color1 = Rgba::BLACK;
        self.color2 = Rgba::WHITE;
    }

    fn reset_after_load(&mut self) {
        self.history.clear();
        self.saved_state = self.history.state_id();
        self.size_generation = None;
        self.view.zoom = 1.0;
        self.view.scroll = egui::Vec2::ZERO;
    }

    fn load(&mut self, path: &Path) {
        let tr = self.tr();
        match io::load(path) {
            Ok(raster) => {
                self.doc = Document::from_raster(raster, format!("{} 1", tr.layer_n));
                self.reset_after_load();
                self.path = Some(path.to_path_buf());
                self.file_size = std::fs::metadata(path).ok().map(|m| m.len());
                self.settings.push_recent(path.to_path_buf());
                self.settings.last_canvas_size = (self.doc.width(), self.doc.height());
                // Large images open fitted, small ones at 100%.
                self.fit_if_larger();
            }
            Err(e) => self.error(tr.open_failed, format!("{}\n{}", path.display(), e)),
        }
    }

    fn fit_if_larger(&mut self) {
        let v = self.view.last_viewport;
        if v.is_positive() && (self.doc.width() as f32 > v.width() || self.doc.height() as f32 > v.height()) {
            self.view.fit((self.doc.width(), self.doc.height()), 1.0);
        }
    }

    pub(crate) fn error(&mut self, title: &str, text: String) {
        tracing::warn!("{title}: {text}");
        self.modal = Some(Modal::Message { title: title.to_owned(), text });
    }

    fn pick_open_path(&self) -> Option<PathBuf> {
        let tr = self.tr();
        let exts: Vec<&str> = io::Format::ALL.iter().flat_map(|f| f.extensions().iter().copied()).collect();
        let mut dlg = rfd::FileDialog::new().add_filter(tr.all_images, &exts);
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dlg = dlg.set_directory(dir);
        }
        dlg.pick_file()
    }

    /// Saves to the current path; returns false if the user cancelled or saving failed.
    pub(crate) fn save(&mut self) -> bool {
        match self.path.clone() {
            Some(path) if io::Format::from_path(&path) == Some(io::Format::Png) => self.save_to(&path),
            _ => self.save_as(io::Format::Png),
        }
    }

    pub(crate) fn save_as(&mut self, format: io::Format) -> bool {
        let tr = self.tr();
        if format != io::Format::Png {
            self.error(tr.save_failed, tr.not_yet.to_owned());
            return false;
        }
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| tr.untitled.to_owned());
        let ext = format.extensions()[0];
        let mut dlg = rfd::FileDialog::new()
            .add_filter(ext.to_uppercase(), format.extensions())
            .set_file_name(format!("{stem}.{ext}"));
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dlg = dlg.set_directory(dir);
        }
        match dlg.save_file() {
            Some(path) => self.save_to(&path),
            None => false,
        }
    }

    fn save_to(&mut self, path: &Path) -> bool {
        let flat = self.doc.flatten();
        match io::save(&flat, path) {
            Ok(()) => {
                self.path = Some(path.to_path_buf());
                self.saved_state = self.history.state_id();
                self.file_size = std::fs::metadata(path).ok().map(|m| m.len());
                self.settings.push_recent(path.to_path_buf());
                true
            }
            Err(e) => {
                let tr = self.tr();
                self.error(tr.save_failed, format!("{}\n{}", path.display(), e));
                false
            }
        }
    }

    // ---------------------------------------------------------------- commands

    pub(crate) fn run_action(&mut self, ctx: &egui::Context, a: Action) {
        let image = (self.doc.width(), self.doc.height());
        let ppp = ctx.pixels_per_point();
        match a {
            Action::New => self.run_or_ask(Pending::New),
            Action::Open => self.run_or_ask(Pending::Open(None)),
            Action::Save => {
                self.save();
            }
            Action::SaveAs => {
                self.save_as(io::Format::Png);
            }
            Action::Undo => {
                self.history.undo(&mut self.doc);
            }
            Action::Redo => {
                self.history.redo(&mut self.doc);
            }
            Action::Zoom100 => self.view.zoom_to(1.0, None, image, ppp),
            Action::ZoomIn | Action::ZoomInAlt => {
                self.view.zoom_to(zoom::zoom_in(self.view.zoom), None, image, ppp)
            }
            Action::ZoomOut | Action::ZoomOutAlt => {
                self.view.zoom_to(zoom::zoom_out(self.view.zoom), None, image, ppp)
            }
            Action::FitToWindow => self.view.fit(image, ppp),
            Action::ToggleRulers => self.settings.show_rulers ^= true,
            Action::ToggleGrid => self.settings.show_grid ^= true,
            Action::FullScreen => {
                self.fullscreen ^= true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
            Action::ToolSelect => self.set_tool(Tool::Select),
            Action::ToolPencil => self.set_tool(Tool::Pencil),
            Action::ToolFill => self.set_tool(Tool::Fill),
            Action::ToolText => self.set_tool(Tool::Text),
            Action::ToolEraser => self.set_tool(Tool::Eraser),
            Action::ToolPicker => self.set_tool(Tool::Picker),
            Action::ToolMagnifier => self.set_tool(Tool::Magnifier),
            // Implemented in later stages.
            Action::Print
            | Action::ImageProperties
            | Action::Cut
            | Action::Copy
            | Action::Paste
            | Action::CopyVisibleLayers
            | Action::SelectAll
            | Action::Delete
            | Action::Crop
            | Action::InvertColors
            | Action::ResizeSkew => {}
        }
    }

    pub(crate) fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }

    pub(crate) fn active_color(&self) -> Rgba {
        match self.active_slot {
            ColorSlot::One => self.color1,
            ColorSlot::Two => self.color2,
        }
    }

    pub(crate) fn set_slot_color(&mut self, slot: ColorSlot, c: Rgba) {
        match slot {
            ColorSlot::One => self.color1 = c,
            ColorSlot::Two => self.color2 = c,
        }
    }

    // ---------------------------------------------------------------- frame

    /// Brings the GPU texture up to date with the document.
    fn sync_canvas(&mut self) {
        let size_gen = self.doc.size_generation();
        let (w, h) = (self.doc.width(), self.doc.height());
        let needs_new = self.size_generation != Some(size_gen)
            || self.mips.as_ref().map(|m| (m.level(0).width(), m.level(0).height())) != Some((w, h));
        if needs_new {
            let mut mips = MipChain::new(w, h);
            self.doc.take_dirty();
            self.doc.composite_into(mips.base_mut(), self.doc.bounds());
            mips.update(self.doc.bounds());
            if let Some(gpu) = &mut self.gpu {
                gpu.recreate(&mips);
            }
            self.mips = Some(mips);
            self.size_generation = Some(size_gen);
            self.settings.last_canvas_size = (w, h);
            return;
        }
        let dirty = self.doc.take_dirty();
        if dirty.is_empty() {
            return;
        }
        if let Some(mips) = &mut self.mips {
            self.doc.composite_into(mips.base_mut(), dirty);
            let changed = mips.update(dirty);
            if let Some(gpu) = &mut self.gpu {
                gpu.upload(mips, &changed);
            }
        }
    }

    /// Draws the whole window content.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);
        let p = self.palette(&ctx);

        if self.page == Page::Settings {
            self.settings_page(ui);
            return;
        }

        let typing = ctx.egui_wants_keyboard_input();
        if self.modal.is_none() {
            for a in self.keymap.pressed(&ctx, !typing) {
                self.run_action(&ctx, a);
            }
        }

        Panel::top("menu_bar")
            .exact_size(MENU_BAR_H)
            .frame(Frame::NONE.fill(p.chrome))
            .show_separator_line(false)
            .show(ui, |ui| self.menu_bar(ui));
        Panel::top("ribbon")
            .exact_size(RIBBON_H + 2.0)
            .frame(Frame::NONE.fill(p.ribbon))
            .show_separator_line(false)
            .show(ui, |ui| self.ribbon(ui));
        if self.settings.show_status_bar {
            Panel::bottom("status_bar")
                .exact_size(STATUS_H)
                .frame(Frame::NONE.fill(p.chrome))
                .show_separator_line(false)
                .show(ui, |ui| self.status_bar(ui));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            self.sync_canvas();
            let draw = CanvasDraw {
                palette: p,
                smoothing: self.settings.smoothing,
                srgb_target: self.gpu.as_ref().is_some_and(CanvasGpu::srgb_target),
                gpu_ready: self.gpu.as_ref().is_some_and(CanvasGpu::has_texture),
            };
            let image = (self.doc.width(), self.doc.height());
            let (geom, resp) = canvas::show(ui, &mut self.view, image, &draw);
            self.hover_pixel = resp.hover_pos().and_then(|pos| geom.pixel_at(pos));
            if self.gpu.is_none() {
                ui.painter().text(
                    geom.viewport.center(),
                    egui::Align2::CENTER_CENTER,
                    "GPU renderer is not available",
                    crate::fonts::regular(14.0),
                    p.text,
                );
            }
        });

        self.modals(&ctx);
    }
}
