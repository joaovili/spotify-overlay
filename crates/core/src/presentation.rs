use crate::playback::{PlaybackClock, TrackSnapshot};
use crate::time::mmss;

/// Textos que o painel mostra. Fica no core para ser testavel sem GUI.
pub struct Presentation;

pub const STOPPED: &str = "Spotify parado";
pub const CONTROL_ERROR: &str = "erro no controle";
pub const APP_NAME: &str = "Spotify";
pub const PERMISSION_HELP: &str =
    "Permita o controle do Spotify em Ajustes > Privacidade e Seguranca > Automacao";

impl Presentation {
    pub fn title(snapshot: &TrackSnapshot) -> String {
        if !snapshot.error.is_empty() {
            return CONTROL_ERROR.into();
        }
        if !snapshot.available {
            return if snapshot.has_track() {
                snapshot.title.clone()
            } else {
                APP_NAME.into()
            };
        }
        snapshot.title.clone()
    }

    pub fn subtitle(snapshot: &TrackSnapshot) -> String {
        if !snapshot.error.is_empty() {
            return snapshot.error.clone();
        }
        if !snapshot.available {
            return STOPPED.into();
        }
        snapshot.artist.clone()
    }

    pub fn album(snapshot: &TrackSnapshot) -> String {
        if !snapshot.error.is_empty() || !snapshot.available {
            return String::new();
        }
        snapshot.album.clone()
    }

    pub fn time(
        snapshot: &TrackSnapshot,
        clock: &mut PlaybackClock,
        now: std::time::Instant,
    ) -> String {
        if snapshot.duration <= 0.0 {
            return String::new();
        }
        let position = clock.update(snapshot, now);
        format!("{} / {}", mmss(position), mmss(snapshot.duration))
    }

    pub fn is_permission_denied(snapshot: &TrackSnapshot) -> bool {
        snapshot.error.contains("-1743")
            || snapshot
                .error
                .to_ascii_lowercase()
                .contains("not permitted")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playback::TrackSnapshot;

    #[test]
    fn stopped_shows_spotify() {
        let snapshot = TrackSnapshot::empty();
        assert_eq!(Presentation::title(&snapshot), "Spotify");
        assert_eq!(Presentation::subtitle(&snapshot), "Spotify parado");
        assert_eq!(Presentation::album(&snapshot), "");
    }

    #[test]
    fn error_takes_priority() {
        let snapshot = TrackSnapshot {
            error: "boom".into(),
            ..TrackSnapshot::empty()
        };
        assert_eq!(Presentation::title(&snapshot), "erro no controle");
        assert_eq!(Presentation::subtitle(&snapshot), "boom");
        assert_eq!(Presentation::album(&snapshot), "");
    }

    #[test]
    fn detects_permission_denied() {
        let snapshot = TrackSnapshot {
            error: "execution error: Not authorized. (-1743)".into(),
            ..TrackSnapshot::empty()
        };
        assert!(Presentation::is_permission_denied(&snapshot));
    }

    #[test]
    fn album_only_when_playing_available() {
        let snapshot = TrackSnapshot {
            album: "Album".into(),
            available: true,
            ..TrackSnapshot::empty()
        };
        assert_eq!(Presentation::album(&snapshot), "Album");
    }
}
