use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    Next,
    Previous,
    Toggle,
}

/// Fotografia imutavel do estado do player.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackSnapshot {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub available: bool,
    pub position: f64,
    pub duration: f64,
    pub position_stamp: Option<Instant>,
    pub error: String,
    pub artwork_key: String,
    /// URL (`https://...`) ou data URL com a capa.
    pub artwork: Option<String>,
}

impl Default for TrackSnapshot {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            playing: false,
            available: false,
            position: 0.0,
            duration: 0.0,
            position_stamp: None,
            error: String::new(),
            artwork_key: String::new(),
            artwork: None,
        }
    }
}

impl TrackSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn has_track(&self) -> bool {
        !self.title.is_empty()
    }
}

/// Payload enviado a interface. A posicao ja vem extrapolada pelo clock.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub available: bool,
    pub position: f64,
    pub duration: f64,
    pub error: String,
    pub artwork_key: String,
    pub artwork: Option<String>,
    pub is_permission_denied: bool,
    pub has_track: bool,
}

impl NowPlaying {
    pub fn from_snapshot(
        snapshot: &TrackSnapshot,
        clock: &mut PlaybackClock,
        now: Instant,
    ) -> Self {
        let position = clock.update(snapshot, now);
        Self {
            title: crate::presentation::Presentation::title(snapshot),
            artist: crate::presentation::Presentation::subtitle(snapshot),
            album: crate::presentation::Presentation::album(snapshot),
            playing: snapshot.playing,
            available: snapshot.available,
            position,
            duration: snapshot.duration.max(0.0),
            error: snapshot.error.clone(),
            artwork_key: snapshot.artwork_key.clone(),
            artwork: snapshot.artwork.clone(),
            is_permission_denied: crate::presentation::Presentation::is_permission_denied(snapshot),
            has_track: snapshot.has_track(),
        }
    }
}

pub struct PlaybackClock {
    last_key: String,
    last_display: f64,
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackClock {
    pub fn new() -> Self {
        Self {
            last_key: String::new(),
            last_display: 0.0,
        }
    }

    /// Posicao estimada para exibir, sem nunca andar para tras entre polls.
    pub fn update(&mut self, snapshot: &TrackSnapshot, now: Instant) -> f64 {
        let key = format!(
            "{}|{}|{}",
            snapshot.title, snapshot.artist, snapshot.duration as i64
        );
        if key != self.last_key {
            self.last_key = key;
            self.last_display = snapshot.position;
        }

        let estimate = display_position(snapshot, now);
        if snapshot.playing {
            self.last_display = self.last_display.max(estimate);
        } else {
            self.last_display = estimate;
        }

        if snapshot.duration > 0.0 {
            self.last_display = self.last_display.min(snapshot.duration);
        }
        self.last_display.max(0.0)
    }

    pub fn reset(&mut self) {
        self.last_key.clear();
        self.last_display = 0.0;
    }
}

pub const VISIBLE_INTERVAL_MS: u64 = 500;
pub const HIDDEN_INTERVAL_MS: u64 = 2500;

pub fn interval_ms(visible: bool) -> u64 {
    if visible {
        VISIBLE_INTERVAL_MS
    } else {
        HIDDEN_INTERVAL_MS
    }
}

/// Posicao estimada agora: ultima leitura mais o tempo decorrido, limitada a duracao.
pub fn display_position(snapshot: &TrackSnapshot, now: Instant) -> f64 {
    if snapshot.duration <= 0.0 {
        return 0.0;
    }
    let mut position = snapshot.position;
    if snapshot.playing {
        if let Some(stamp) = snapshot.position_stamp {
            position += now.saturating_duration_since(stamp).as_secs_f64();
        }
    }
    position.clamp(0.0, snapshot.duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn snapshot() -> TrackSnapshot {
        TrackSnapshot {
            title: "Song".into(),
            artist: "Artist".into(),
            duration: 100.0,
            available: true,
            playing: true,
            position: 10.0,
            position_stamp: Some(Instant::now()),
            ..TrackSnapshot::empty()
        }
    }

    #[test]
    fn clock_extrapolates_while_playing() {
        let now = Instant::now();
        let mut snap = snapshot();
        snap.position_stamp = Some(now - Duration::from_secs(5));
        let mut clock = PlaybackClock::new();
        let position = clock.update(&snap, now);
        assert!((position - 15.0).abs() < 0.01, "{position}");
    }

    #[test]
    fn clock_never_goes_backwards() {
        let now = Instant::now();
        let mut snap = snapshot();
        snap.position_stamp = Some(now - Duration::from_secs(30));
        let mut clock = PlaybackClock::new();
        let first = clock.update(&snap, now);
        assert!((first - 40.0).abs() < 0.01);

        snap.position = 12.0;
        snap.position_stamp = Some(now);
        let second = clock.update(&snap, now);
        assert!(second >= first);
    }

    #[test]
    fn clock_resets_on_track_change() {
        let now = Instant::now();
        let mut clock = PlaybackClock::new();
        let mut snap = snapshot();
        snap.position_stamp = Some(now - Duration::from_secs(30));
        let _ = clock.update(&snap, now);

        snap.title = "Other".into();
        snap.position = 2.0;
        snap.position_stamp = Some(now);
        let position = clock.update(&snap, now);
        assert!((position - 2.0).abs() < 0.01);
    }

    #[test]
    fn position_is_clamped_to_duration() {
        let now = Instant::now();
        let mut snap = snapshot();
        snap.position = 98.0;
        snap.position_stamp = Some(now - Duration::from_secs(10));
        assert!((display_position(&snap, now) - 100.0).abs() < 0.01);
    }

    #[test]
    fn paused_position_does_not_advance() {
        let now = Instant::now();
        let mut snap = snapshot();
        snap.playing = false;
        snap.position = 42.0;
        assert!((display_position(&snap, now) - 42.0).abs() < 0.01);
    }
}
