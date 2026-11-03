use crate::platform;
use crate::state::AppState;
use crate::worker::WorkerMsg;
use spotify_overlay_core::layout::{top_distance, window_position, Rect};
use spotify_overlay_core::{Config, OverlaySize, ScreenPosition};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

pub fn apply(app: &AppHandle, config: &Config) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let size = OverlaySize::parse(&config.size);
    let (width, height) = size.dimensions();
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    platform::set_content_size(app, width, height);

    let custom = ScreenPosition::parse(&config.position) == ScreenPosition::Custom;
    let monitor = if custom {
        window
            .available_monitors()
            .ok()
            .and_then(|monitors| {
                monitors.into_iter().find(|monitor| {
                    let scale = monitor.scale_factor();
                    let area = monitor.work_area();
                    let rect = Rect::new(
                        area.position.x as f64 / scale,
                        area.position.y as f64 / scale,
                        area.size.width as f64 / scale,
                        area.size.height as f64 / scale,
                    );
                    let x = config.custom_x as f64;
                    let y = rect.y + config.custom_y as f64;
                    x >= rect.x
                        && x <= rect.x + rect.width
                        && y >= rect.y
                        && y <= rect.y + rect.height
                })
            })
            .or_else(|| window.primary_monitor().ok().flatten())
    } else {
        window.primary_monitor().ok().flatten()
    };

    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let work_area = Rect::new(
            area.position.x as f64 / scale,
            area.position.y as f64 / scale,
            area.size.width as f64 / scale,
            area.size.height as f64 / scale,
        );
        let position = window_position(
            ScreenPosition::parse(&config.position),
            size,
            config.custom_x,
            config.custom_y,
            work_area,
        );
        window
            .set_position(LogicalPosition::new(position.x, position.y))
            .map_err(|error| error.to_string())?;
    }

    window
        .set_always_on_top(config.always_on_top)
        .map_err(|error| error.to_string())?;
    platform::set_level(app, config.always_on_top);
    Ok(())
}

pub fn show(app: &AppHandle) {
    app.state::<AppState>().set_visible(true);
    platform::show(app);
    app.state::<AppState>().send(WorkerMsg::Refresh);
}

pub fn hide(app: &AppHandle) {
    app.state::<AppState>().set_visible(false);
    platform::hide(app);
}

pub fn toggle(app: &AppHandle) {
    if app.state::<AppState>().is_visible() {
        hide(app);
    } else {
        show(app);
    }
}

/// Grava a posicao atual como `custom`. Chamado depois que o usuario arrasta.
pub fn save_position(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let Some(monitor) = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
    else {
        return Ok(());
    };
    let area = monitor.work_area();
    let work_area = Rect::new(
        area.position.x as f64 / scale,
        area.position.y as f64 / scale,
        area.size.width as f64 / scale,
        area.size.height as f64 / scale,
    );
    let rect = Rect::new(
        position.x as f64 / scale,
        position.y as f64 / scale,
        0.0,
        0.0,
    );
    let top = top_distance(rect, work_area);

    let state = app.state::<AppState>();
    let mut config = state.config();
    config.position = ScreenPosition::Custom.as_str().into();
    config.custom_x = rect.x.round() as i32;
    config.custom_y = top;
    config
        .save(&state.config_path)
        .map_err(|error| error.to_string())?;
    state.set_config(config.clone());
    let _ = app.emit("config", &config);
    Ok(())
}

pub fn open_settings(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("Spotify Overlay")
    .inner_size(420.0, 610.0)
    .resizable(false)
    .maximizable(false)
    .center()
    .build()
    .map_err(|error| error.to_string())?;

    Ok(())
}
