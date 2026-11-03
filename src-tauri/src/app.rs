use crate::commands;
use crate::state::AppState;
use crate::worker::{self, WorkerMsg};
use crate::{hotkeys, platform, tray, window};
use spotify_overlay_core::Config;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WindowEvent};

pub fn run() {
    let mut builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app);
        }));

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(hotkeys::plugin())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::get_snapshot,
            commands::save_config,
            commands::control,
            commands::hide_overlay,
            commands::toggle_overlay,
            commands::show_overlay,
            commands::save_position,
            commands::open_settings,
            commands::quit,
            commands::set_hotkey_capture,
            commands::close_overlay,
            commands::popup_menu,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let path = config_path(&handle);
            let config = Config::load(&path);

            app.manage(AppState::new(config.clone(), path));
            platform::convert_overlay(&handle);

            window::apply(&handle, &config)?;
            if let Some(window) = handle.get_webview_window("main") {
                window.on_menu_event(|window, event| {
                    crate::menu::handle(window.app_handle(), event.id.as_ref())
                });
            }
            tray::setup(&handle)?;

            if let Err(error) = hotkeys::register(&handle, &config) {
                eprintln!("atalhos: {error}");
            }
            commands::apply_autostart(&handle, config.start_with_windows);

            let sender = worker::spawn(handle.clone(), config.clone());
            handle
                .state::<AppState>()
                .worker
                .lock()
                .unwrap()
                .replace(sender);
            handle
                .state::<AppState>()
                .send(WorkerMsg::SetWantArtwork(config.size == "large"));

            window::show(&handle);

            if std::env::args().any(|arg| arg == "--settings") {
                let _ = window::open_settings(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let app = window.app_handle();
                    if app.state::<AppState>().config().close_hides_to_tray {
                        window::hide(app);
                    } else {
                        app.exit(0);
                    }
                }
            }

            if window.label() == "settings" {
                if matches!(event, WindowEvent::Destroyed) {
                    #[cfg(target_os = "macos")]
                    {
                        let _ = window
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                    let app = window.app_handle();
                    let config = app.state::<AppState>().config();
                    let _ = hotkeys::register(app, &config);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn config_path(app: &AppHandle) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // Mantem o comportamento historico: config ao lado do .exe quando a pasta
        // e gravavel (versao portatil); senao, %APPDATA%\SpotifyOverlay.
        if let Ok(executable) = std::env::current_exe() {
            if let Some(directory) = executable.parent() {
                let probe = directory.join(".spotify-overlay-probe");
                if std::fs::write(&probe, b"1").is_ok() {
                    let _ = std::fs::remove_file(&probe);
                    return directory.join("config.json");
                }
            }
        }
    }

    let base = app
        .path()
        .data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("SpotifyOverlay").join("config.json")
}
