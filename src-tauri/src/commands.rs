use crate::state::AppState;
use crate::worker::WorkerMsg;
use crate::{hotkeys, menu, window};
use spotify_overlay_core::{Config, MediaCommand, NowPlaying};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Option<NowPlaying> {
    state.last_snapshot()
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: Config) -> Result<Config, String> {
    update_config(&app, |current| *current = config.clone())
}

/// Aplica, persiste e propaga uma mudanca de configuracao. Usado pela janela
/// de configuracoes e pelos itens do menu/bandeja.
pub fn update_config(app: &AppHandle, mutate: impl FnOnce(&mut Config)) -> Result<Config, String> {
    let state = app.state::<AppState>();
    let mut config = state.config();
    mutate(&mut config);
    config.normalize();
    config
        .save(&state.config_path)
        .map_err(|error| error.to_string())?;
    state.set_config(config.clone());

    window::apply(app, &config)?;
    state.send(WorkerMsg::SetPlayer(config.player.clone()));
    state.send(WorkerMsg::SetWantArtwork(config.size == "large"));
    state.send(WorkerMsg::Refresh);

    let hotkey_error = hotkeys::register(app, &config).err();
    apply_autostart(app, config.start_with_windows);
    let _ = app.emit("config", &config);
    crate::menu::refresh(app);

    if let Some(error) = hotkey_error {
        return Err(format!("config salva, mas o atalho falhou: {error}"));
    }
    Ok(config)
}

#[tauri::command]
pub fn control(state: State<'_, AppState>, command: String) -> Result<(), String> {
    let command = match command.as_str() {
        "next" => MediaCommand::Next,
        "previous" => MediaCommand::Previous,
        "toggle" => MediaCommand::Toggle,
        other => return Err(format!("comando desconhecido: {other}")),
    };
    state.send(WorkerMsg::Control(command));
    Ok(())
}

#[tauri::command]
pub fn hide_overlay(app: AppHandle) {
    window::hide(&app);
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) {
    let config = app.state::<AppState>().config();
    if config.close_hides_to_tray {
        window::hide(&app);
    } else {
        app.exit(0);
    }
}

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) {
    window::toggle(&app);
}

#[tauri::command]
pub fn show_overlay(app: AppHandle) {
    window::show(&app);
}

#[tauri::command]
pub fn save_position(app: AppHandle) -> Result<(), String> {
    window::save_position(&app)
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    window::open_settings(&app)
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn popup_menu(app: AppHandle) -> Result<(), String> {
    let config = app.state::<AppState>().config();
    let menu = menu::build(&app, &config).map_err(|error| error.to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "janela principal nao encontrada".to_string())?;
    window.popup_menu(&menu).map_err(|error| error.to_string())
}

/// Enquanto o usuario grava um atalho, os atalhos globais ficam desligados para
/// a combinacao nao disparar a acao antiga.
#[tauri::command]
pub fn set_hotkey_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    capturing: bool,
) -> Result<(), String> {
    if capturing {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        app.global_shortcut()
            .unregister_all()
            .map_err(|error| error.to_string())
    } else {
        hotkeys::register(&app, &state.config())
    }
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let _ = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
}
