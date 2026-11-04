use crate::platform;
use eframe::egui;
use spotify_overlay_core::Config;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

const PANEL: egui::Color32 = egui::Color32::from_rgb(0x1B, 0x1B, 0x1B);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF5, 0xF5, 0xF5);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0x96, 0x96, 0x96);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x1D, 0xB9, 0x54);
const DANGER: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x6B, 0x6B);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyField {
    Skip,
    Previous,
    Toggle,
}

pub struct SettingsState {
    pub config: Config,
    pub recording: Option<HotkeyField>,
    pub status: String,
    pub focused: bool,
    screenshot: Option<std::path::PathBuf>,
    screenshot_requested: bool,
    screenshot_done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    opened_at: std::time::Instant,
}

impl SettingsState {
    pub fn new(config: Config, screenshot: Option<std::path::PathBuf>) -> Self {
        Self {
            config,
            recording: None,
            status: String::new(),
            focused: false,
            screenshot,
            screenshot_requested: false,
            screenshot_done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            opened_at: std::time::Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Save(Config),
    Close,
    Quit,
    Recording(bool),
}

pub fn id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("settings")
}

pub fn viewport() -> egui::ViewportBuilder {
    let area = platform::primary_work_area();
    let x = area.x + (area.width - 420.0) / 2.0;
    let y = area.y + (area.height - 610.0) / 2.0;
    egui::ViewportBuilder::default()
        .with_title("Spotify Overlay")
        .with_inner_size([420.0, 610.0])
        .with_min_inner_size([420.0, 610.0])
        .with_position([x as f32, y as f32])
        .with_resizable(false)
        .with_decorations(true)
        .with_transparent(false)
}

pub fn show(ui: &mut egui::Ui, state: &Arc<Mutex<SettingsState>>, tx: &Arc<Mutex<Sender<Event>>>) {
    let focused = state.lock().unwrap().focused;
    if !focused {
        state.lock().unwrap().focused = true;
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    if ui.ctx().input(|input| input.viewport().close_requested()) {
        send(tx, Event::Close);
        return;
    }

    capture_hotkey(ui, state, tx);

    egui::Panel::bottom("settings_footer")
        .frame(
            egui::Frame::NONE
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(16, 12)),
        )
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([88.0, 30.0], egui::Button::new("Salvar").fill(ACCENT))
                    .clicked()
                {
                    let config = state.lock().unwrap().config.clone();
                    send(tx, Event::Save(config));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui
                    .add_sized([88.0, 30.0], egui::Button::new("Cancelar"))
                    .clicked()
                {
                    send(tx, Event::Close);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(16, 14)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading(
                        egui::RichText::new("Spotify Overlay")
                            .color(TEXT)
                            .size(18.0),
                    );
                    ui.label(
                        egui::RichText::new("Controle o player sem tirar o foco do app em uso.")
                            .color(MUTED)
                            .size(12.0),
                    );
                    ui.add_space(10.0);

                    section(ui, "Aparência");
                    egui::Grid::new("appearance")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Tamanho");
                            let size = state.lock().unwrap().config.size.clone();
                            let mut selected_size = size.clone();
                            egui::ComboBox::from_id_salt("size")
                                .selected_text(size_label(&size))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut selected_size, "mini".into(), "Mini");
                                    ui.selectable_value(
                                        &mut selected_size,
                                        "medium".into(),
                                        "Médio",
                                    );
                                    ui.selectable_value(
                                        &mut selected_size,
                                        "large".into(),
                                        "Grande",
                                    );
                                });
                            if selected_size != size {
                                state.lock().unwrap().config.size = selected_size;
                            }
                            ui.end_row();

                            ui.label("Posição");
                            let position = state.lock().unwrap().config.position.clone();
                            let mut selected_position = position.clone();
                            egui::ComboBox::from_id_salt("position")
                                .selected_text(position_label(&position))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut selected_position,
                                        "top-right".into(),
                                        "Superior direita",
                                    );
                                    ui.selectable_value(
                                        &mut selected_position,
                                        "top-left".into(),
                                        "Superior esquerda",
                                    );
                                    ui.selectable_value(
                                        &mut selected_position,
                                        "bottom-right".into(),
                                        "Inferior direita",
                                    );
                                    ui.selectable_value(
                                        &mut selected_position,
                                        "bottom-left".into(),
                                        "Inferior esquerda",
                                    );
                                    ui.selectable_value(
                                        &mut selected_position,
                                        "custom".into(),
                                        "Personalizada",
                                    );
                                });
                            if selected_position != position {
                                state.lock().unwrap().config.position = selected_position;
                            }
                            ui.end_row();

                            let custom = state.lock().unwrap().config.position == "custom";
                            if custom {
                                ui.label("Coordenadas");
                                let (mut custom_x, mut custom_y) = {
                                    let locked = state.lock().unwrap();
                                    (locked.config.custom_x, locked.config.custom_y)
                                };
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut custom_x).speed(1.0));
                                    ui.add(egui::DragValue::new(&mut custom_y).speed(1.0));
                                });
                                {
                                    let mut locked = state.lock().unwrap();
                                    locked.config.custom_x = custom_x;
                                    locked.config.custom_y = custom_y;
                                }
                                ui.end_row();
                            }

                            ui.label("Player");
                            let player = state.lock().unwrap().config.player.clone();
                            let mut selected_player = player.clone();
                            egui::ComboBox::from_id_salt("player")
                                .selected_text(player_label(&player))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut selected_player,
                                        "auto".into(),
                                        "Automático",
                                    );
                                    ui.selectable_value(
                                        &mut selected_player,
                                        "spotify".into(),
                                        "Spotify",
                                    );
                                    ui.selectable_value(
                                        &mut selected_player,
                                        "spotifast".into(),
                                        "Spotifast",
                                    );
                                });
                            if selected_player != player {
                                state.lock().unwrap().config.player = selected_player;
                            }
                            ui.end_row();
                        });

                    ui.add_space(10.0);
                    section(ui, "Atalhos globais");
                    hotkey_row(ui, state, tx, HotkeyField::Skip, "Próxima faixa");
                    hotkey_row(ui, state, tx, HotkeyField::Previous, "Faixa anterior");
                    hotkey_row(ui, state, tx, HotkeyField::Toggle, "Mostrar/ocultar");
                    ui.label(
                        egui::RichText::new(
                            "Clique no campo e pressione a combinação. Esc cancela.",
                        )
                        .color(MUTED)
                        .size(11.0),
                    );

                    ui.add_space(10.0);
                    section(ui, "Comportamento");
                    let (mut always_on_top, mut close_hides_to_tray, mut start_with_windows) = {
                        let locked = state.lock().unwrap();
                        (
                            locked.config.always_on_top,
                            locked.config.close_hides_to_tray,
                            locked.config.start_with_windows,
                        )
                    };
                    ui.checkbox(&mut always_on_top, "Sempre no topo");
                    ui.checkbox(
                        &mut close_hides_to_tray,
                        "O botão fechar esconde para a bandeja",
                    );
                    #[cfg(target_os = "macos")]
                    ui.checkbox(&mut start_with_windows, "Iniciar no login");
                    #[cfg(not(target_os = "macos"))]
                    ui.checkbox(&mut start_with_windows, "Iniciar com o Windows");
                    {
                        let mut locked = state.lock().unwrap();
                        locked.config.always_on_top = always_on_top;
                        locked.config.close_hides_to_tray = close_hides_to_tray;
                        locked.config.start_with_windows = start_with_windows;
                    }

                    let status = state.lock().unwrap().status.clone();
                    if !status.is_empty() {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(status).color(DANGER).size(11.5));
                    }
                });
        });

    let screenshot = {
        let locked = state.lock().unwrap();
        locked.screenshot.clone().map(|path| {
            (
                path,
                locked.opened_at.elapsed(),
                locked.screenshot_requested,
                locked.screenshot_done.clone(),
            )
        })
    };
    if let Some((path, elapsed, requested, done)) = screenshot {
        if !requested && elapsed > std::time::Duration::from_millis(1500) {
            state.lock().unwrap().screenshot_requested = true;
            let done_clone = done.clone();
            ui.ctx().request_screenshot(move |capture| {
                crate::screenshot::save_png(&path, &capture);
                done_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
        if done.load(std::sync::atomic::Ordering::Relaxed) {
            send(tx, Event::Quit);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn send(tx: &Arc<Mutex<Sender<Event>>>, event: Event) {
    if let Ok(sender) = tx.lock() {
        sender.send(event).ok();
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .color(MUTED)
            .size(11.0)
            .strong(),
    );
    ui.add_space(4.0);
}

fn hotkey_row(
    ui: &mut egui::Ui,
    state: &Arc<Mutex<SettingsState>>,
    tx: &Arc<Mutex<Sender<Event>>>,
    field: HotkeyField,
    label: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([130.0, 26.0], egui::Label::new(label));
        let (value, recording) = {
            let locked = state.lock().unwrap();
            (
                hotkey_value(&locked.config, field).to_string(),
                locked.recording == Some(field),
            )
        };
        let text = if recording {
            "Pressione...".to_string()
        } else if value.is_empty() {
            "(desativado)".to_string()
        } else {
            value
        };
        if ui
            .add_sized([150.0, 26.0], egui::Button::new(text))
            .clicked()
        {
            state.lock().unwrap().recording = Some(field);
            state.lock().unwrap().status.clear();
            send(tx, Event::Recording(true));
        }
        if ui.small_button("Limpar").clicked() {
            set_hotkey_value(&mut state.lock().unwrap().config, field, String::new());
        }
    });
}

fn capture_hotkey(
    ui: &egui::Ui,
    state: &Arc<Mutex<SettingsState>>,
    tx: &Arc<Mutex<Sender<Event>>>,
) {
    let field = match state.lock().unwrap().recording {
        Some(field) => field,
        None => return,
    };

    let events = ui.ctx().input(|input| input.events.clone());
    for event in events {
        let egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            continue;
        };

        if key == egui::Key::Escape {
            state.lock().unwrap().recording = None;
            send(tx, Event::Recording(false));
            return;
        }

        let Some(key_name) = key_name(key) else {
            continue;
        };

        let mut parts: Vec<&str> = Vec::new();
        if modifiers.ctrl {
            parts.push("Ctrl");
        }
        if modifiers.alt {
            parts.push("Alt");
        }
        if modifiers.shift {
            parts.push("Shift");
        }
        #[cfg(target_os = "macos")]
        if modifiers.mac_cmd {
            parts.push("Cmd");
        }

        if parts.is_empty() {
            state.lock().unwrap().status =
                "Use ao menos um modificador (Ctrl, Alt, Shift ou Cmd).".into();
            continue;
        }

        parts.push(&key_name);
        let spec = parts.join("+");
        match spotify_overlay_core::hotkey::parse(&spec) {
            Ok(_) => {
                let mut locked = state.lock().unwrap();
                set_hotkey_value(&mut locked.config, field, spec);
                locked.recording = None;
                locked.status.clear();
                drop(locked);
                send(tx, Event::Recording(false));
            }
            Err(error) => {
                state.lock().unwrap().status = error.to_string();
            }
        }
        return;
    }
}

fn hotkey_value(config: &Config, field: HotkeyField) -> &str {
    match field {
        HotkeyField::Skip => &config.hotkey_skip,
        HotkeyField::Previous => &config.hotkey_prev,
        HotkeyField::Toggle => &config.hotkey_toggle,
    }
}

fn set_hotkey_value(config: &mut Config, field: HotkeyField, value: String) {
    match field {
        HotkeyField::Skip => config.hotkey_skip = value,
        HotkeyField::Previous => config.hotkey_prev = value,
        HotkeyField::Toggle => config.hotkey_toggle = value,
    }
}

fn key_name(key: egui::Key) -> Option<String> {
    use egui::Key;
    let name = match key {
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::Space => "Space",
        Key::Enter => "Enter",
        Key::Tab => "Tab",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PgUp",
        Key::PageDown => "PgDn",
        Key::Delete => "Delete",
        Key::Backspace => "Backspace",
        Key::MediaTrackNext => "MediaNextTrack",
        Key::MediaTrackPrevious => "MediaPrevTrack",
        Key::MediaPlayPause => "MediaPlayPause",
        Key::Minus => "-",
        Key::Equals | Key::Plus => "=",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Backslash => "\\",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        Key::Backtick => "`",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        other => {
            let debug = format!("{other:?}");
            if let Some(number) = debug.strip_prefix('F') {
                if number.chars().all(|c| c.is_ascii_digit()) {
                    return Some(debug);
                }
            }
            return None;
        }
    };
    Some(name.to_string())
}

fn size_label(value: &str) -> &'static str {
    match value {
        "mini" => "Mini",
        "large" => "Grande",
        _ => "Médio",
    }
}

fn position_label(value: &str) -> &'static str {
    match value {
        "top-left" => "Superior esquerda",
        "bottom-right" => "Inferior direita",
        "bottom-left" => "Inferior esquerda",
        "custom" => "Personalizada",
        _ => "Superior direita",
    }
}

fn player_label(value: &str) -> &'static str {
    match value {
        "spotify" => "Spotify",
        "spotifast" => "Spotifast",
        _ => "Automático",
    }
}
