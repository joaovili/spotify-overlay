use crate::state::AppState;
use crate::worker::WorkerMsg;
use spotify_overlay_core::hotkey::parse_optional;
use spotify_overlay_core::{Config, MediaCommand};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if let Some(command) = action_for(app, shortcut) {
                app.state::<AppState>().send(WorkerMsg::Control(command));
            }
        })
        .build()
}

pub fn register(app: &AppHandle, config: &Config) -> Result<(), String> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let mut errors = Vec::new();
    for (spec, _) in bindings(config) {
        let Some(Ok(parsed)) = parse_optional(&spec) else {
            if !spec.trim().is_empty() && parse_optional(&spec).is_some() {
                errors.push(format!("{spec}: atalho invalido"));
            }
            continue;
        };
        let accelerator = parsed.accelerator();
        if let Err(error) = manager.register(accelerator.as_str()) {
            errors.push(format!("{accelerator}: {error}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn bindings(config: &Config) -> [(String, MediaCommand); 3] {
    [
        (config.hotkey_skip.clone(), MediaCommand::Next),
        (config.hotkey_prev.clone(), MediaCommand::Previous),
        (config.hotkey_toggle.clone(), MediaCommand::Toggle),
    ]
}

fn action_for(app: &AppHandle, shortcut: &Shortcut) -> Option<MediaCommand> {
    let config = app.state::<AppState>().config();
    for (spec, command) in bindings(&config) {
        let Some(Ok(parsed)) = parse_optional(&spec) else {
            continue;
        };
        let target = parsed.accelerator().parse::<Shortcut>().ok()?;
        if target == *shortcut {
            return Some(command);
        }
    }
    None
}
