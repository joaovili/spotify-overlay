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
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x3A, 0x3A, 0x3A);

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
    let corner = egui::CornerRadius::same(metrics.corner_radius as u8);
    painter.rect_filled(panel, corner, BG);
    painter.rect_stroke(
        panel,
        corner,
        egui::Stroke::new(1.0, BORDER),
        egui::StrokeKind::Inside,
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
    let stroke = egui::Stroke::new((size * 0.09).max(1.5), color);

    match kind {
        ButtonKind::Previous | ButtonKind::Next => {
            let direction = if kind == ButtonKind::Previous {
                -1.0
            } else {
                1.0
            };
            let tip = center.x + direction * size * 0.125;
            let back = center.x - direction * size * 0.29;
            painter.add(egui::Shape::closed_line(
                vec![
                    egui::pos2(back, center.y - size * 0.33),
                    egui::pos2(back, center.y + size * 0.33),
                    egui::pos2(tip, center.y),
                ],
                stroke,
            ));
            let bar_x = center.x + direction * size * 0.29;
            painter.line_segment(
                [
                    egui::pos2(bar_x, center.y - size * 0.33),
                    egui::pos2(bar_x, center.y + size * 0.33),
                ],
                stroke,
            );
        }
        ButtonKind::PlayPause => {
            if playing {
                let half_height = size * 0.33;
                let half_width = size * 0.085;
                let offset = size * 0.17;
                for x in [center.x - offset, center.x + offset] {
                    painter.add(egui::Shape::closed_line(
                        vec![
                            egui::pos2(x - half_width, center.y - half_height),
                            egui::pos2(x + half_width, center.y - half_height),
                            egui::pos2(x + half_width, center.y + half_height),
                            egui::pos2(x - half_width, center.y + half_height),
                        ],
                        stroke,
                    ));
                }
            } else {
                painter.add(egui::Shape::closed_line(
                    vec![
                        egui::pos2(center.x - size * 0.24, center.y - size * 0.34),
                        egui::pos2(center.x + size * 0.32, center.y),
                        egui::pos2(center.x - size * 0.24, center.y + size * 0.34),
                    ],
                    stroke,
                ));
            }
        }
        ButtonKind::Settings => {
            let glyph = egui::Rect::from_center_size(center, egui::vec2(size * 0.68, size * 0.68));
            draw_gear(painter, glyph, color);
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

enum GearSeg {
    M(f32, f32),
    L(f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

const GEAR: &[GearSeg] = &[
    GearSeg::M(12.22, 2.0),
    GearSeg::L(11.78, 2.0),
    GearSeg::C(10.675, 2.0, 9.78, 2.895, 9.78, 4.0),
    GearSeg::L(9.78, 4.18),
    GearSeg::C(9.779, 4.894, 9.398, 5.553, 8.78, 5.91),
    GearSeg::L(8.35, 6.16),
    GearSeg::C(7.731, 6.517, 6.969, 6.517, 6.35, 6.16),
    GearSeg::L(6.2, 6.08),
    GearSeg::C(5.244, 5.529, 4.023, 5.855, 3.47, 6.81),
    GearSeg::L(3.25, 7.19),
    GearSeg::C(2.699, 8.146, 3.025, 9.367, 3.98, 9.92),
    GearSeg::L(4.13, 10.02),
    GearSeg::C(4.745, 10.375, 5.126, 11.03, 5.13, 11.74),
    GearSeg::L(5.13, 12.25),
    GearSeg::C(5.133, 12.967, 4.751, 13.631, 4.13, 13.99),
    GearSeg::L(3.98, 14.08),
    GearSeg::C(3.025, 14.633, 2.699, 15.854, 3.25, 16.81),
    GearSeg::L(3.47, 17.19),
    GearSeg::C(4.023, 18.145, 5.244, 18.471, 6.2, 17.92),
    GearSeg::L(6.35, 17.84),
    GearSeg::C(6.969, 17.483, 7.731, 17.483, 8.35, 17.84),
    GearSeg::L(8.78, 18.09),
    GearSeg::C(9.398, 18.447, 9.779, 19.106, 9.78, 19.82),
    GearSeg::L(9.78, 20.0),
    GearSeg::C(9.78, 21.105, 10.675, 22.0, 11.78, 22.0),
    GearSeg::L(12.22, 22.0),
    GearSeg::C(13.325, 22.0, 14.22, 21.105, 14.22, 20.0),
    GearSeg::L(14.22, 19.82),
    GearSeg::C(14.221, 19.106, 14.602, 18.447, 15.22, 18.09),
    GearSeg::L(15.65, 17.84),
    GearSeg::C(16.269, 17.483, 17.031, 17.483, 17.65, 17.84),
    GearSeg::L(17.8, 17.92),
    GearSeg::C(18.756, 18.471, 19.977, 18.145, 20.53, 17.19),
    GearSeg::L(20.75, 16.8),
    GearSeg::C(21.301, 15.844, 20.975, 14.623, 20.02, 14.07),
    GearSeg::L(19.87, 13.99),
    GearSeg::C(19.249, 13.631, 18.867, 12.967, 18.87, 12.25),
    GearSeg::L(18.87, 11.75),
    GearSeg::C(18.867, 11.033, 19.249, 10.369, 19.87, 10.01),
    GearSeg::L(20.02, 9.92),
    GearSeg::C(20.975, 9.367, 21.301, 8.146, 20.75, 7.19),
    GearSeg::L(20.53, 6.81),
    GearSeg::C(19.977, 5.855, 18.756, 5.529, 17.8, 6.08),
    GearSeg::L(17.65, 6.16),
    GearSeg::C(17.031, 6.517, 16.269, 6.517, 15.65, 6.16),
    GearSeg::L(15.22, 5.91),
    GearSeg::C(14.602, 5.553, 14.221, 4.894, 14.22, 4.18),
    GearSeg::L(14.22, 4.0),
    GearSeg::C(14.22, 2.895, 13.325, 2.0, 12.22, 2.0),
    GearSeg::Z,
];

fn draw_gear(painter: &egui::Painter, box_rect: egui::Rect, color: egui::Color32) {
    let unit = box_rect.width().min(box_rect.height()) / 24.0;
    let origin = egui::pos2(
        box_rect.center().x - 12.0 * unit,
        box_rect.center().y - 12.0 * unit,
    );
    let stroke = egui::Stroke::new(1.8 * unit, color);
    let point = |x: f32, y: f32| egui::pos2(origin.x + x * unit, origin.y + y * unit);

    let mut points = Vec::new();
    let mut current = origin;
    for segment in GEAR {
        match *segment {
            GearSeg::M(x, y) => {
                current = point(x, y);
                points.push(current);
            }
            GearSeg::L(x, y) => {
                current = point(x, y);
                points.push(current);
            }
            GearSeg::C(c1x, c1y, c2x, c2y, x, y) => {
                let start = current;
                let c1 = point(c1x, c1y);
                let c2 = point(c2x, c2y);
                let end = point(x, y);
                const STEPS: usize = 6;
                for step in 1..=STEPS {
                    let t = step as f32 / STEPS as f32;
                    let mt = 1.0 - t;
                    points.push(egui::pos2(
                        mt * mt * mt * start.x
                            + 3.0 * mt * mt * t * c1.x
                            + 3.0 * mt * t * t * c2.x
                            + t * t * t * end.x,
                        mt * mt * mt * start.y
                            + 3.0 * mt * mt * t * c1.y
                            + 3.0 * mt * t * t * c2.y
                            + t * t * t * end.y,
                    ));
                }
                current = end;
            }
            GearSeg::Z => {}
        }
    }

    if points.len() > 1 {
        if points[0].distance(*points.last().unwrap()) < 0.01 {
            points.pop();
        }
        painter.add(egui::Shape::closed_line(points, stroke));
    }
    painter.circle_stroke(point(12.0, 12.0), 3.0 * unit, stroke);
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
