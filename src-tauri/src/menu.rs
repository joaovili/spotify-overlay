use crate::commands;
use crate::window;
use spotify_overlay_core::Config;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager};

pub fn build(app: &AppHandle, config: &Config) -> tauri::Result<Menu<tauri::Wry>> {
    let toggle = MenuItem::with_id(app, "toggle", "Mostrar/Ocultar", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Configuracoes", true, None::<&str>)?;
    let mini = CheckMenuItem::with_id(
        app,
        "size-mini",
        "Mini",
        true,
        config.size == "mini",
        None::<&str>,
    )?;
    let medium = CheckMenuItem::with_id(
        app,
        "size-medium",
        "Medio",
        true,
        config.size == "medium",
        None::<&str>,
    )?;
    let large = CheckMenuItem::with_id(
        app,
        "size-large",
        "Grande",
        true,
        config.size == "large",
        None::<&str>,
    )?;
    let on_top = CheckMenuItem::with_id(
        app,
        "on-top",
        "Sempre no topo",
        true,
        config.always_on_top,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &toggle,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &mini,
            &medium,
            &large,
            &PredefinedMenuItem::separator(app)?,
            &on_top,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

pub fn handle(app: &AppHandle, id: &str) {
    match id {
        "toggle" => window::toggle(app),
        "settings" => {
            let _ = window::open_settings(app);
        }
        "size-mini" => set_size(app, "mini"),
        "size-medium" => set_size(app, "medium"),
        "size-large" => set_size(app, "large"),
        "on-top" => {
            let _ = commands::update_config(app, |config| {
                config.always_on_top = !config.always_on_top;
            });
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

fn set_size(app: &AppHandle, size: &str) {
    let _ = commands::update_config(app, |config| {
        config.size = size.to_string();
    });
}

/// Reconstroi o menu da bandeja para refletir os checkmarks de tamanho/topo.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    let config = app.state::<crate::state::AppState>().config();
    if let Ok(menu) = build(app, &config) {
        let _ = tray.set_menu(Some(menu));
    }
}
