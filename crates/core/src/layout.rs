/// Tamanhos do painel, em pixels logicos (points no macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySize {
    Mini,
    Medium,
    Large,
}

impl OverlaySize {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "mini" => Self::Mini,
            "large" => Self::Large,
            _ => Self::Medium,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mini => "mini",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub fn dimensions(self) -> (f64, f64) {
        match self {
            Self::Mini => (158.0, 38.0),
            Self::Medium => (376.0, 58.0),
            Self::Large => (440.0, 148.0),
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Mini, Self::Medium, Self::Large]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenPosition {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Custom,
}

impl ScreenPosition {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "top-left" => Self::TopLeft,
            "bottom-right" => Self::BottomRight,
            "bottom-left" => Self::BottomLeft,
            "custom" => Self::Custom,
            _ => Self::TopRight,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopRight => "top-right",
            Self::TopLeft => "top-left",
            Self::BottomRight => "bottom-right",
            Self::BottomLeft => "bottom-left",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub const MARGIN: f64 = 24.0;

/// Posicao do painel em coordenadas de tela com origem no topo-esquerda.
/// `work_area` e a area util do monitor (sem menu bar/Dock/taskbar).
pub fn window_position(
    position: ScreenPosition,
    size: OverlaySize,
    custom_x: i32,
    custom_y: i32,
    work_area: Rect,
) -> Rect {
    let (width, height) = size.dimensions();
    let (x, y) = match position {
        ScreenPosition::TopLeft => (work_area.x + MARGIN, work_area.y + MARGIN),
        ScreenPosition::TopRight => (
            work_area.x + work_area.width - width - MARGIN,
            work_area.y + MARGIN,
        ),
        ScreenPosition::BottomLeft => (
            work_area.x + MARGIN,
            work_area.y + work_area.height - height - MARGIN,
        ),
        ScreenPosition::BottomRight => (
            work_area.x + work_area.width - width - MARGIN,
            work_area.y + work_area.height - height - MARGIN,
        ),
        ScreenPosition::Custom => {
            let clamped_x =
                (custom_x as f64).clamp(work_area.x - 4.0, work_area.x + work_area.width - 40.0);
            let clamped_top = (custom_y as f64).clamp(-4.0, work_area.height - 20.0);
            (clamped_x, work_area.y + clamped_top)
        }
    };

    Rect::new(x, y, width, height)
}

/// Distancia do topo da area util ate o topo da janela (valor de `customY`).
pub fn top_distance(window: Rect, work_area: Rect) -> i32 {
    (window.y - work_area.y).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(0.0, 0.0, 1440.0, 900.0)
    }

    #[test]
    fn dimensions_match_old_layouts() {
        assert_eq!(OverlaySize::Mini.dimensions(), (158.0, 38.0));
        assert_eq!(OverlaySize::Medium.dimensions(), (376.0, 58.0));
        assert_eq!(OverlaySize::Large.dimensions(), (440.0, 148.0));
    }

    #[test]
    fn corners_apply_margin() {
        let top_right = window_position(ScreenPosition::TopRight, OverlaySize::Large, 0, 0, area());
        assert_eq!(top_right.x, 1440.0 - 440.0 - 24.0);
        assert_eq!(top_right.y, 24.0);

        let bottom_left =
            window_position(ScreenPosition::BottomLeft, OverlaySize::Mini, 0, 0, area());
        assert_eq!(bottom_left.x, 24.0);
        assert_eq!(bottom_left.y, 900.0 - 38.0 - 24.0);
    }

    #[test]
    fn custom_is_top_left_origin() {
        let rect = window_position(ScreenPosition::Custom, OverlaySize::Medium, 300, 40, area());
        assert_eq!(rect.x, 300.0);
        assert_eq!(rect.y, 40.0);
        assert_eq!(top_distance(rect, area()), 40);
    }

    #[test]
    fn custom_is_clamped_to_screen() {
        let rect = window_position(
            ScreenPosition::Custom,
            OverlaySize::Medium,
            -500,
            5000,
            area(),
        );
        assert_eq!(rect.x, -4.0);
        assert_eq!(rect.y, 880.0);
    }

    #[test]
    fn works_with_offset_monitor() {
        let area = Rect::new(-1920.0, -200.0, 1920.0, 1080.0);
        let rect = window_position(ScreenPosition::TopRight, OverlaySize::Large, 0, 0, area);
        assert_eq!(rect.x, -1920.0 + 1920.0 - 440.0 - 24.0);
        assert_eq!(rect.y, -200.0 + 24.0);
    }
}
