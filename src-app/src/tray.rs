use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use spotify_overlay_core::Config;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub fn build_menu(config: &Config) -> muda::Result<Menu> {
    let toggle = MenuItem::with_id("toggle", "Mostrar/Ocultar", true, None);
    let settings = MenuItem::with_id("settings", "Configurações", true, None);
    let mini = CheckMenuItem::with_id("size-mini", "Mini", true, config.size == "mini", None);
    let medium =
        CheckMenuItem::with_id("size-medium", "Médio", true, config.size == "medium", None);
    let large = CheckMenuItem::with_id("size-large", "Grande", true, config.size == "large", None);
    let size = Submenu::with_items("Tamanho", true, &[&mini, &medium, &large])?;
    let on_top =
        CheckMenuItem::with_id("on-top", "Sempre no topo", true, config.always_on_top, None);
    let quit = MenuItem::with_id("quit", "Sair", true, None);

    Menu::with_items(&[
        &toggle,
        &settings,
        &PredefinedMenuItem::separator(),
        &size,
        &PredefinedMenuItem::separator(),
        &on_top,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
}

pub fn create(config: &Config) -> Option<TrayIcon> {
    let menu = build_menu(config).ok()?;
    let icon = load_icon()?;
    #[cfg(target_os = "macos")]
    let builder = TrayIconBuilder::new().with_icon_templated(icon);
    #[cfg(not(target_os = "macos"))]
    let builder = TrayIconBuilder::new().with_icon(icon);

    builder
        .with_tooltip("Spotify Overlay")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .ok()
}

pub fn refresh(tray: &TrayIcon, config: &Config) {
    if let Ok(menu) = build_menu(config) {
        tray.set_menu(Some(Box::new(menu)));
    }
}

fn load_icon() -> Option<Icon> {
    #[cfg(target_os = "macos")]
    let size = 22u32;
    #[cfg(not(target_os = "macos"))]
    let size = 32u32;

    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let samples = 4u32;
    for py in 0..size {
        for px in 0..size {
            let mut covered = 0u32;
            for sy in 0..samples {
                for sx in 0..samples {
                    let x = (px as f32 + (sx as f32 + 0.5) / samples as f32) / size as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / samples as f32) / size as f32;
                    if inside_glyph(x, y) {
                        covered += 1;
                    }
                }
            }
            let alpha = (covered * 255 / (samples * samples)) as u8;
            let index = ((py * size + px) * 4) as usize;
            rgba[index] = 255;
            rgba[index + 1] = 255;
            rgba[index + 2] = 255;
            rgba[index + 3] = alpha;
        }
    }
    Icon::from_rgba(rgba, size, size).ok()
}

fn inside_glyph(x: f32, y: f32) -> bool {
    let triangle = point_in_triangle((x, y), (0.18, 0.10), (0.18, 0.62), (0.66, 0.36));
    let bar = (0.12..=0.82).contains(&x) && (0.76..=0.90).contains(&y);
    triangle || bar
}

fn point_in_triangle(point: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let sign = |p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    };
    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}
