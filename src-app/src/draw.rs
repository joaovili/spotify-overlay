use eframe::egui;
use spotify_overlay_core::layout::{metrics, ButtonKind, Metrics, Rect};
use spotify_overlay_core::{NowPlaying, OverlaySize};

pub const BG: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x18);
const FG: egui::Color32 = egui::Color32::from_rgb(0xF5, 0xF5, 0xF5);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0x96, 0x96, 0x96);
const HOVER: egui::Color32 = egui::Color32::from_rgb(0x2C, 0x2C, 0x2C);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x1D, 0xB9, 0x54);
const TRACK: egui::Color32 = egui::Color32::from_rgb(0x40, 0x40, 0x40);
const COVER: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Previous,
    PlayPause,
    Next,
    Settings,
    Close,
}

pub fn button_at(size: OverlaySize, point: egui::Pos2) -> Option<ButtonKind> {
    let metrics = metrics(size);
    metrics
        .buttons
        .iter()
        .find(|(_, rect)| contains(rect, point))
        .map(|(kind, _)| *kind)
}

pub fn draw(
    ui: &mut egui::Ui,
    size: OverlaySize,
    snapshot: &NowPlaying,
    position: f64,
    artwork: Option<&egui::TextureHandle>,
) -> Option<Action> {
    let metrics = metrics(size);
    let origin = ui.max_rect().min;
    let panel = egui::Rect::from_min_size(
        origin,
        egui::vec2(metrics.width as f32, metrics.height as f32),
    );
    let painter = ui.painter().with_clip_rect(panel);
    painter.rect_filled(
        panel,
        egui::CornerRadius::same(metrics.corner_radius as u8),
        BG,
    );

    let pointer = ui.ctx().pointer_hover_pos();
    let hovered = pointer.and_then(|point| button_at(size, point));

    draw_cover(ui, &painter, &metrics, origin, artwork);
    draw_texts(&painter, size, &metrics, origin, snapshot, position);
    draw_progress(&painter, &metrics, origin, snapshot, position);

    let mut action = None;
    for (kind, rect) in metrics.buttons {
        let screen = to_screen(origin, rect);
        let response = ui.interact(
            screen,
            ui.scope_id().with(("btn", kind as u8)),
            egui::Sense::click(),
        );
        if response.hovered() {
            painter.rect_filled(screen.shrink(1.0), egui::CornerRadius::same(6), HOVER);
        }
        draw_glyph(
            &painter,
            kind,
            screen,
            snapshot.playing,
            hovered == Some(kind),
        );
        if response.clicked() {
            action = Some(match kind {
                ButtonKind::Previous => Action::Previous,
                ButtonKind::PlayPause => Action::PlayPause,
                ButtonKind::Next => Action::Next,
                ButtonKind::Settings => Action::Settings,
                ButtonKind::Close => Action::Close,
            });
        }
    }

    action
}

fn draw_cover(
    ui: &egui::Ui,
    painter: &egui::Painter,
    metrics: &Metrics,
    origin: egui::Pos2,
    artwork: Option<&egui::TextureHandle>,
) {
    let Some(rect) = metrics.cover else {
        return;
    };
    let rect = to_screen(origin, rect);
    painter.rect_filled(rect, egui::CornerRadius::same(8), COVER);
    if let Some(texture) = artwork {
        egui::Image::new((texture.id(), texture.size_vec2()))
            .corner_radius(egui::CornerRadius::same(8))
            .paint_at(ui, rect);
    }
}

fn draw_texts(
    painter: &egui::Painter,
    size: OverlaySize,
    metrics: &Metrics,
    origin: egui::Pos2,
    snapshot: &NowPlaying,
    _position: f64,
) {
    let title_size = match size {
        OverlaySize::Mini => 10.0,
        OverlaySize::Medium => 11.5,
        OverlaySize::Large => 14.0,
    };
    let artist_size = match size {
        OverlaySize::Mini => 9.0,
        OverlaySize::Medium => 10.0,
        OverlaySize::Large => 11.0,
    };

    let title = if snapshot.has_track {
        snapshot.title.as_str()
    } else {
        "Spotify"
    };
    if let Some(rect) = metrics.title {
        draw_text(
            painter,
            origin,
            rect,
            title,
            title_size,
            FG,
            egui::FontFamily::Name(std::sync::Arc::from("bold")),
        );
    }
    if let Some(rect) = metrics.artist {
        let (text, color) = if snapshot.is_permission_denied {
            (
                "Permita o controle em Ajustes > Privacidade".to_string(),
                ACCENT,
            )
        } else {
            (snapshot.artist.clone(), MUTED)
        };
        draw_text(
            painter,
            origin,
            rect,
            &text,
            artist_size,
            color,
            egui::FontFamily::Proportional,
        );
    }
    if let Some(rect) = metrics.album {
        draw_text(
            painter,
            origin,
            rect,
            &snapshot.album,
            10.0,
            MUTED,
            egui::FontFamily::Proportional,
        );
    }
    if let Some(rect) = metrics.time_label {
        let time = if snapshot.duration > 0.0 {
            format!("{} / {}", mmss(_position), mmss(snapshot.duration))
        } else {
            String::new()
        };
        draw_text(
            painter,
            origin,
            rect,
            &time,
            9.5,
            MUTED,
            egui::FontFamily::Monospace,
        );
    }
}

fn draw_progress(
    painter: &egui::Painter,
    metrics: &Metrics,
    origin: egui::Pos2,
    snapshot: &NowPlaying,
    position: f64,
) {
    let Some(rect) = metrics.progress else {
        return;
    };
    let rect = to_screen(origin, rect);
    painter.rect_filled(rect, egui::CornerRadius::same(2), TRACK);
    if snapshot.duration > 0.0 {
        let fraction = (position / snapshot.duration).clamp(0.0, 1.0);
        if fraction > 0.001 {
            let filled = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() * fraction as f32, rect.height()),
            );
            painter.rect_filled(filled, egui::CornerRadius::same(2), ACCENT);
        }
    }
}

fn draw_glyph(
    painter: &egui::Painter,
    kind: ButtonKind,
    rect: egui::Rect,
    playing: bool,
    hovered: bool,
) {
    let size = rect.width().min(rect.height());
    let center = rect.center();
    let color = match kind {
        ButtonKind::PlayPause => ACCENT,
        ButtonKind::Settings | ButtonKind::Close => {
            if hovered {
                FG
            } else {
                MUTED
            }
        }
        _ => FG,
    };
    let stroke = egui::Stroke::new((size * 0.11).max(1.5), color);

    match kind {
        ButtonKind::Previous | ButtonKind::Next => {
            let direction = if kind == ButtonKind::Previous {
                -1.0
            } else {
                1.0
            };
            let bar_x = center.x - direction * size * 0.34;
            painter.line_segment(
                [
                    egui::pos2(bar_x, center.y - size * 0.34),
                    egui::pos2(bar_x, center.y + size * 0.34),
                ],
                egui::Stroke::new((size * 0.14).max(2.0), color),
            );
            let tip = center.x + direction * size * 0.36;
            let back = center.x - direction * size * 0.2;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(back, center.y - size * 0.36),
                    egui::pos2(back, center.y + size * 0.36),
                    egui::pos2(tip, center.y),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        ButtonKind::PlayPause => {
            if playing {
                let offset = size * 0.16;
                painter.line_segment(
                    [
                        egui::pos2(center.x - offset, center.y - size * 0.32),
                        egui::pos2(center.x - offset, center.y + size * 0.32),
                    ],
                    egui::Stroke::new((size * 0.18).max(3.0), color),
                );
                painter.line_segment(
                    [
                        egui::pos2(center.x + offset, center.y - size * 0.32),
                        egui::pos2(center.x + offset, center.y + size * 0.32),
                    ],
                    egui::Stroke::new((size * 0.18).max(3.0), color),
                );
            } else {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(center.x - size * 0.26, center.y - size * 0.34),
                        egui::pos2(center.x - size * 0.26, center.y + size * 0.34),
                        egui::pos2(center.x + size * 0.34, center.y),
                    ],
                    color,
                    egui::Stroke::NONE,
                ));
            }
        }
        ButtonKind::Settings => {
            let half = size * 0.30;
            for (index, y) in [-0.24, 0.0, 0.24].into_iter().enumerate() {
                let y = center.y + y * size;
                painter.line_segment(
                    [
                        egui::pos2(center.x - half, y),
                        egui::pos2(center.x + half, y),
                    ],
                    egui::Stroke::new((size * 0.1).max(1.5), color),
                );
                let knob_x = match index {
                    0 => -0.10,
                    1 => 0.18,
                    _ => -0.18,
                };
                painter.circle_filled(
                    egui::pos2(center.x + knob_x * size, y),
                    (size * 0.085).max(1.5),
                    color,
                );
            }
        }
        ButtonKind::Close => {
            let d = size * 0.28;
            painter.line_segment(
                [
                    egui::pos2(center.x - d, center.y - d),
                    egui::pos2(center.x + d, center.y + d),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + d, center.y - d),
                    egui::pos2(center.x - d, center.y + d),
                ],
                stroke,
            );
        }
    }
}

fn draw_text(
    painter: &egui::Painter,
    origin: egui::Pos2,
    rect: Rect,
    text: &str,
    size: f32,
    color: egui::Color32,
    family: egui::FontFamily,
) {
    if text.is_empty() {
        return;
    }
    let rect = to_screen(origin, rect);
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: egui::FontId::new(size, family),
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: rect.width(),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    let y = rect.center().y - galley.size().y * 0.5;
    painter.galley(egui::pos2(rect.left(), y), galley, color);
}

fn contains(rect: &Rect, point: egui::Pos2) -> bool {
    let x = point.x as f64;
    let y = point.y as f64;
    x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

fn to_screen(origin: egui::Pos2, rect: Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(origin.x + rect.x as f32, origin.y + rect.y as f32),
        egui::vec2(rect.width as f32, rect.height as f32),
    )
}

fn mmss(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}
