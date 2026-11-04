use crate::artwork::Artwork;
use crate::autostart::Autostart;
use crate::draw::{self, Action};
use crate::hotkeys::{HotkeyAction, Hotkeys};
use crate::platform;
use crate::settings;
use crate::tray;
use crate::worker::{self, Worker, WorkerMsg};
use eframe::egui;
use single_instance::SingleInstance;
use spotify_overlay_core::layout::{top_distance, window_position};
use spotify_overlay_core::{Config, MediaCommand, NowPlaying, OverlaySize, ScreenPosition};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconEvent};
use winit::dpi::{LogicalPosition, LogicalSize};

pub struct OverlayApp {
    config: Config,
    config_path: PathBuf,
    worker: Worker,
    snapshot: NowPlaying,
    base_position: f64,
    base_time: Instant,
    playing: bool,
    duration: f64,
    window_ready: bool,
    visible: bool,
    settings_requested: bool,
    hotkeys: Hotkeys,
    autostart: Autostart,
    tray: Option<TrayIcon>,
    _instance: Option<SingleInstance>,
    settings: Option<Arc<Mutex<settings::SettingsState>>>,
    settings_tx: Arc<Mutex<Sender<settings::Event>>>,
    settings_rx: Receiver<settings::Event>,
    artwork: Artwork,
    screenshot_path: Option<PathBuf>,
    settings_screenshot: Option<PathBuf>,
    screenshot_requested: bool,
    screenshot_taken: Arc<AtomicBool>,
    started: Instant,
}

impl OverlayApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        config: Config,
        config_path: PathBuf,
        worker: Worker,
        instance: Option<SingleInstance>,
        screenshot_path: Option<PathBuf>,
        settings_screenshot: Option<PathBuf>,
    ) -> Self {
        configure_fonts(&cc.egui_ctx);
        let hotkeys = Hotkeys::new(&config);
        let autostart = Autostart::new();
        autostart.apply(config.start_with_windows);
        let tray = tray::create(&config);
        let (settings_tx, settings_rx) = mpsc::channel();
        Self {
            config,
            config_path,
            worker,
            snapshot: NowPlaying::default(),
            base_position: 0.0,
            base_time: Instant::now(),
            playing: false,
            duration: 0.0,
            window_ready: false,
            visible: true,
            settings_requested: false,
            hotkeys,
            autostart,
            tray,
            _instance: instance,
            settings: None,
            settings_tx: Arc::new(Mutex::new(settings_tx)),
            settings_rx,
            artwork: Artwork::new(),
            screenshot_path,
            settings_screenshot,
            screenshot_requested: false,
            screenshot_taken: Arc::new(AtomicBool::new(false)),
            started: Instant::now(),
        }
    }

    fn poll_worker(&mut self) {
        let latest = self.worker.latest.lock().unwrap().clone();
        let Some(next) = latest else {
            return;
        };
        if next == self.snapshot {
            return;
        }
        let same_track = next.title == self.snapshot.title && next.artist == self.snapshot.artist;
        let current = self.current_position();
        self.snapshot = next;
        self.playing = self.snapshot.playing;
        self.duration = self.snapshot.duration;
        self.base_position = if same_track {
            self.snapshot.position.max(current - 0.35)
        } else {
            self.snapshot.position
        };
        self.base_time = Instant::now();
    }

    fn current_position(&self) -> f64 {
        if self.playing {
            (self.base_position + self.base_time.elapsed().as_secs_f64())
                .min(self.duration.max(0.0))
        } else {
            self.base_position
        }
    }

    fn apply_window(&mut self, window: &winit::window::Window) {
        let size = OverlaySize::parse(&self.config.size);
        let (width, height) = size.dimensions();
        let _ = window.request_inner_size(LogicalSize::new(width, height));

        let area = platform::work_area_for(&self.config, size);
        let position = window_position(
            ScreenPosition::parse(&self.config.position),
            size,
            self.config.custom_x,
            self.config.custom_y,
            area,
        );
        window.set_outer_position(LogicalPosition::new(position.x, position.y));
        platform::set_level(window, self.config.always_on_top);
    }

    fn save_position(&mut self, window: &winit::window::Window) {
        let scale = window.scale_factor();
        let position = window.outer_position().map(|p| p.to_logical::<f64>(scale));
        let Ok(position) = position else {
            return;
        };
        let area = platform::work_area_at_point(position.x, position.y);
        let window_rect = spotify_overlay_core::layout::Rect::new(position.x, position.y, 0.0, 0.0);
        self.config.position = ScreenPosition::Custom.as_str().to_string();
        self.config.custom_x = position.x.round() as i32;
        self.config.custom_y = top_distance(window_rect, area);
        let _ = self.config.save(&self.config_path);
    }

    fn set_visible(
        &mut self,
        window: Option<&winit::window::Window>,
        visible: bool,
        ctx: &egui::Context,
    ) {
        self.visible = visible;
        self.worker
            .visible
            .store(visible, std::sync::atomic::Ordering::Relaxed);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
        if let Some(window) = window {
            if visible {
                platform::show(window);
            } else {
                platform::hide(window);
            }
        }
        self.worker.tx.send(WorkerMsg::Refresh).ok();
        ctx.request_repaint();
    }

    fn update_config(
        &mut self,
        update: impl FnOnce(&mut Config),
        window: Option<&winit::window::Window>,
        ctx: &egui::Context,
    ) {
        let mut config = self.config.clone();
        update(&mut config);
        config.normalize();
        if let Err(error) = config.save(&self.config_path) {
            eprintln!("config: {error}");
        }
        self.config = config.clone();

        if let Some(window) = window {
            self.apply_window(window);
        }
        self.worker
            .tx
            .send(WorkerMsg::SetPlayer(config.player.clone()))
            .ok();
        self.worker
            .tx
            .send(WorkerMsg::SetWantArtwork(config.size == "large"))
            .ok();
        self.worker.tx.send(WorkerMsg::Refresh).ok();

        if let Err(error) = self.hotkeys.register(&config) {
            eprintln!("atalhos: {error}");
        }
        self.autostart.apply(config.start_with_windows);
        if let Some(tray) = &self.tray {
            tray::refresh(tray, &config);
        }
        ctx.request_repaint();
    }

    fn handle_menu(
        &mut self,
        id: &str,
        window: Option<&winit::window::Window>,
        ctx: &egui::Context,
    ) {
        match id {
            "toggle" => {
                let visible = !self.visible;
                self.set_visible(window, visible, ctx);
            }
            "settings" => self.open_settings(),
            "size-mini" => self.update_config(|config| config.size = "mini".into(), window, ctx),
            "size-medium" => {
                self.update_config(|config| config.size = "medium".into(), window, ctx)
            }
            "size-large" => self.update_config(|config| config.size = "large".into(), window, ctx),
            "on-top" => self.update_config(
                |config| config.always_on_top = !config.always_on_top,
                window,
                ctx,
            ),
            "quit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            _ => {}
        }
    }

    fn show_context_menu(&self, window: &winit::window::Window) {
        let Ok(menu) = tray::build_menu(&self.config) else {
            return;
        };
        use muda::ContextMenu;
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = window.window_handle() else {
            return;
        };
        match handle.as_raw() {
            #[cfg(target_os = "macos")]
            RawWindowHandle::AppKit(handle) => unsafe {
                menu.show_context_menu_for_nsview(handle.ns_view.as_ptr(), None);
            },
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(handle) => unsafe {
                menu.show_context_menu_for_hwnd(handle.hwnd.get() as isize, None);
            },
            _ => {}
        }
    }

    fn open_settings(&mut self) {
        platform::set_regular_policy();
        let screenshot = self.settings_screenshot.take();
        self.settings = Some(Arc::new(Mutex::new(settings::SettingsState::new(
            self.config.clone(),
            screenshot,
        ))));
        self.settings_requested = false;
    }

    fn close_settings(&mut self, ctx: &egui::Context) {
        self.settings = None;
        let _ = self.hotkeys.end_capture(&self.config);
        platform::set_accessory_policy();
        ctx.send_viewport_cmd_to(settings::id(), egui::ViewportCommand::Close);
    }

    fn poll_settings(&mut self, window: Option<&winit::window::Window>, ctx: &egui::Context) {
        while let Ok(event) = self.settings_rx.try_recv() {
            match event {
                settings::Event::Save(config) => {
                    self.update_config(|current| *current = config, window, ctx);
                    self.close_settings(ctx);
                }
                settings::Event::Close => self.close_settings(ctx),
                settings::Event::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                settings::Event::Recording(capturing) => {
                    if capturing {
                        self.hotkeys.begin_capture();
                    } else {
                        self.hotkeys.end_capture(&self.config);
                    }
                }
            }
        }
    }

    fn apply_action(
        &mut self,
        action: Action,
        window: Option<&winit::window::Window>,
        ctx: &egui::Context,
    ) {
        match action {
            Action::Previous => {
                self.worker
                    .tx
                    .send(WorkerMsg::Control(MediaCommand::Previous))
                    .ok();
            }
            Action::Next => {
                self.worker
                    .tx
                    .send(WorkerMsg::Control(MediaCommand::Next))
                    .ok();
            }
            Action::PlayPause => {
                self.worker
                    .tx
                    .send(WorkerMsg::Control(MediaCommand::Toggle))
                    .ok();
            }
            Action::Settings => self.open_settings(),
            Action::Close => {
                if self.config.close_hides_to_tray {
                    self.set_visible(window, false, ctx);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // O teardown do winit no macOS 26 entra num panico nao-unwinding ao
        // liberar o WindowDelegate. A config ja foi persistida; sair aqui evita
        // o abort no fechamento.
        std::process::exit(0);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let window = frame.winit_window().cloned();

        if !self.window_ready {
            self.window_ready = true;
            if let Some(window) = &window {
                platform::apply_overlay_style(window);
                self.apply_window(window);
                platform::show(window);
            }
            platform::set_accessory_policy();
        }

        self.poll_worker();
        self.poll_settings(window.as_deref(), &ctx);

        if self.settings_requested {
            self.open_settings();
        }

        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            self.handle_menu(event.id.0.as_str(), window.as_deref(), &ctx);
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let visible = !self.visible;
                self.set_visible(window.as_deref(), visible, &ctx);
            }
        }
        if let Some(action) = self.hotkeys.poll() {
            match action {
                HotkeyAction::ToggleVisibility => {
                    let visible = !self.visible;
                    self.set_visible(window.as_deref(), visible, &ctx);
                }
                HotkeyAction::Previous => {
                    self.worker
                        .tx
                        .send(WorkerMsg::Control(MediaCommand::Previous))
                        .ok();
                }
                HotkeyAction::Next => {
                    self.worker
                        .tx
                        .send(WorkerMsg::Control(MediaCommand::Next))
                        .ok();
                }
            }
        }

        if let Some(window) = &window {
            if ctx.input(|input| input.viewport().close_requested()) {
                if self.config.close_hides_to_tray {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.set_visible(Some(window), false, &ctx);
                } else {
                    self.visible = false;
                }
            }

            if ctx.input(|input| input.pointer.secondary_clicked())
                && ctx.input(|input| input.pointer.interact_pos().is_some())
            {
                self.show_context_menu(window);
            }

            let button_pressed = ctx.input(|input| {
                input
                    .pointer
                    .interact_pos()
                    .map(|position| {
                        draw::button_at(OverlaySize::parse(&self.config.size), position).is_none()
                            && input.pointer.primary_pressed()
                    })
                    .unwrap_or(false)
            });
            if button_pressed {
                let _ = window.drag_window();
                self.save_position(window);
            }
        }

        self.artwork.update(&ctx, &self.snapshot);
        let size = OverlaySize::parse(&self.config.size);
        let action = draw::draw(
            ui,
            size,
            &self.snapshot,
            self.current_position(),
            self.artwork.texture(),
        );
        if let Some(action) = action {
            self.apply_action(action, window.as_deref(), &ctx);
        }

        if let Some(settings) = self.settings.clone() {
            let tx = Arc::clone(&self.settings_tx);
            ctx.show_viewport_deferred(settings::id(), settings::viewport(), move |ui, _class| {
                settings::show(ui, &settings, &tx);
            });
        }

        if let Some(path) = self.screenshot_path.clone() {
            if !self.screenshot_requested && self.started.elapsed() > Duration::from_millis(1500) {
                self.screenshot_requested = true;
                let taken = Arc::clone(&self.screenshot_taken);
                ctx.request_screenshot(move |capture| {
                    crate::screenshot::save_png(&path, &capture);
                    taken.store(true, Ordering::Relaxed);
                });
            }
            if self.screenshot_taken.load(Ordering::Relaxed) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

pub fn run() {
    let Some(instance) = crate::instance::acquire() else {
        return;
    };

    let config_path = config_path();
    let config = Config::load(&config_path);
    let args: Vec<String> = std::env::args().collect();
    let screenshot_path = args
        .iter()
        .position(|arg| arg == "--screenshot")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from);
    let settings_screenshot = args
        .iter()
        .position(|arg| arg == "--settings-screenshot")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from);
    let open_settings = args.iter().any(|arg| arg == "--settings") || settings_screenshot.is_some();

    platform::set_accessory_policy();

    let size = OverlaySize::parse(&config.size);
    let (width, height) = size.dimensions();
    let area = platform::work_area_for(&config, size);
    let position = window_position(
        ScreenPosition::parse(&config.position),
        size,
        config.custom_x,
        config.custom_y,
        area,
    );

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Spotify Overlay")
        .with_inner_size([width as f32, height as f32])
        .with_position([position.x as f32, position.y as f32])
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_visible(false)
        .with_active(false)
        .with_taskbar(false);
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icons/128x128.png"))
    {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        centered: false,
        persist_window: false,
        run_and_return: false,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Spotify Overlay",
        options,
        Box::new(move |cc| {
            let worker = worker::spawn(&config, cc.egui_ctx.clone());
            let mut app = OverlayApp::new(
                cc,
                config,
                config_path,
                worker,
                Some(instance),
                screenshot_path,
                settings_screenshot,
            );
            app.settings_requested = open_settings;
            Ok(Box::new(app))
        }),
    );

    if let Err(error) = result {
        eprintln!("falha ao iniciar a interface: {error}");
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut add = |name: &str, path: &str, family: egui::FontFamily| {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                name.to_string(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.to_string());
        }
    };

    #[cfg(target_os = "macos")]
    {
        add(
            "sf",
            "/System/Library/Fonts/SFNS.ttf",
            egui::FontFamily::Proportional,
        );
        add(
            "sf-mono",
            "/System/Library/Fonts/SFNSMono.ttf",
            egui::FontFamily::Monospace,
        );
        add(
            "sf-bold",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            egui::FontFamily::Name(std::sync::Arc::from("bold")),
        );
    }
    #[cfg(target_os = "windows")]
    {
        add(
            "segoe",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            egui::FontFamily::Proportional,
        );
        add(
            "consola",
            "C:\\Windows\\Fonts\\consola.ttf",
            egui::FontFamily::Monospace,
        );
        add(
            "segoe-bold",
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            egui::FontFamily::Name(std::sync::Arc::from("bold")),
        );
    }

    let fallback = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    fonts
        .families
        .entry(egui::FontFamily::Name(std::sync::Arc::from("bold")))
        .or_insert(fallback);

    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(0x1B, 0x1B, 0x1B);
    visuals.window_fill = visuals.panel_fill;
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x1D, 0xB9, 0x54);
    ctx.set_visuals(visuals);
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    ctx.set_global_style(style);
}

fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(executable) = std::env::current_exe() {
            if let Some(directory) = executable.parent() {
                let probe = directory.join(".spotify-overlay-probe");
                if std::fs::write(&probe, b"1").is_ok() {
                    let _ = std::fs::remove_file(&probe);
                    return directory.join("config.json");
                }
            }
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("SpotifyOverlay")
                .join("config.json");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library/Application Support/SpotifyOverlay/config.json");
        }
    }

    PathBuf::from("config.json")
}
