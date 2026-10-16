//! PaintRS — entry point: window, config, command line.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use paint_ui::{Editor, Settings};

struct App {
    editor: Editor,
    saved_settings: Settings,
    title: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, settings: Settings, open: Option<PathBuf>) -> Self {
        paint_ui::setup(&cc.egui_ctx);
        let mut editor = Editor::new(settings.clone(), cc.wgpu_render_state.as_ref());
        if let Some(path) = open {
            editor.open_path(path);
        }
        Self { editor, saved_settings: settings, title: String::new() }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && !self.editor.exit_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.editor.request_exit();
        }
        if let Some(path) = ctx.input(|i| {
            i.raw.dropped_files.iter().find_map(|f| {
                let p = f.path();
                (!p.as_os_str().is_empty()).then(|| p.to_path_buf())
            })
        }) {
            self.editor.open_path(path);
        }

        self.editor.ui(ui);

        let title = self.editor.title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.editor.settings != self.saved_settings {
            config::save(&self.editor.settings);
            self.saved_settings = self.editor.settings.clone();
        }
        if self.editor.exit_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn wgpu_options() -> egui_wgpu::WgpuConfiguration {
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.device_descriptor = Arc::new(|adapter: &wgpu::Adapter| {
        let base = if adapter.get_info().backend == wgpu::Backend::Gl {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        // Large canvases need large textures: ask for whatever the adapter supports.
        let max_tex = adapter.limits().max_texture_dimension_2d;
        wgpu::DeviceDescriptor {
            label: Some("paintrs device"),
            required_limits: wgpu::Limits { max_texture_dimension_2d: max_tex, ..base },
            ..Default::default()
        }
    });
    egui_wgpu::WgpuConfiguration { wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(setup), ..Default::default() }
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,paint_app=info,paint_ui=info")),
        )
        .init();

    let open = std::env::args_os().nth(1).map(PathBuf::from);
    let settings = config::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PaintRS")
            .with_app_id("paintrs")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([720.0, 480.0])
            .with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: wgpu_options(),
        depth_buffer: 0,
        multisampling: 0,
        centered: true,
        ..Default::default()
    };
    eframe::run_native("PaintRS", options, Box::new(move |cc| Ok(Box::new(App::new(cc, settings, open)))))
}
