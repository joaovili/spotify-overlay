use tauri::AppHandle;

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(OverlayPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            hides_on_deactivate: false,
            is_floating_panel: true
        }
    })
}

#[cfg(target_os = "macos")]
pub fn convert_overlay(app: &AppHandle) {
    use objc2_app_kit::NSWindowStyleMask;
    use tauri::Manager;
    use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelLevel, WebviewWindowExt};

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let panel = match window.to_panel::<OverlayPanel>() {
        Ok(panel) => panel,
        Err(error) => {
            eprintln!("nspanel: {error}");
            return;
        }
    };

    panel.set_level(PanelLevel::Floating.value());
    panel.set_floating_panel(true);
    panel.set_hides_on_deactivate(false);
    panel.set_becomes_key_only_if_needed(true);
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .full_screen_auxiliary()
            .ignores_cycle()
            .value(),
    );
    let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
    let _ = app.get_webview_panel("main");
}

#[cfg(target_os = "windows")]
pub fn convert_overlay(app: &AppHandle) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    spotify_overlay_media::windows::apply_overlay_window_style(hwnd.0 as isize);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn convert_overlay(_app: &AppHandle) {}

#[cfg(target_os = "macos")]
pub fn set_content_size(app: &AppHandle, width: f64, height: f64) {
    use tauri_nspanel::ManagerExt;
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.set_content_size(width, height);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_content_size(_app: &AppHandle, _width: f64, _height: f64) {}

#[cfg(target_os = "macos")]
pub fn set_level(app: &AppHandle, floating: bool) {
    use tauri_nspanel::{ManagerExt, PanelLevel};
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.set_level(if floating {
            PanelLevel::Floating.value()
        } else {
            PanelLevel::Normal.value()
        });
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_level(_app: &AppHandle, _floating: bool) {}

#[cfg(target_os = "macos")]
pub fn show(app: &AppHandle) {
    use tauri_nspanel::ManagerExt;
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.show();
    }
    show_window(app);
}

#[cfg(target_os = "macos")]
pub fn hide(app: &AppHandle) {
    use tauri_nspanel::ManagerExt;
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.hide();
        return;
    }
    hide_window(app);
}

#[cfg(not(target_os = "macos"))]
pub fn show(app: &AppHandle) {
    show_window(app);
}

#[cfg(not(target_os = "macos"))]
pub fn hide(app: &AppHandle) {
    hide_window(app);
}

#[cfg(not(target_os = "macos"))]
fn show_window(app: &AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

#[cfg(not(target_os = "macos"))]
fn hide_window(app: &AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(target_os = "macos")]
fn show_window(app: &AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

#[cfg(target_os = "macos")]
fn hide_window(app: &AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}
