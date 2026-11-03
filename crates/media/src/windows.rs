use crate::MediaBackend;
use base64::Engine;
use spotify_overlay_core::{MediaCommand, TrackSnapshot};
use std::time::{Duration, Instant};
use windows::Foundation::TimeSpan;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

const PROPERTIES_TTL: Duration = Duration::from_secs(15);

/// Deixa o painel fora do Alt+Tab e sem roubar foco ao receber clique
/// (WS_EX_TOOLWINDOW + WS_EX_NOACTIVATE), como na versao nativa antiga.
pub fn apply_overlay_window_style(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    unsafe {
        let hwnd = HWND(hwnd as *mut core::ffi::c_void);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let extra = (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | extra);
    }
}
const MAX_THUMBNAIL_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
struct Properties {
    title: String,
    artist: String,
    album: String,
    read_at: Option<Instant>,
}

pub struct WindowsMedia {
    manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
    session: Option<GlobalSystemMediaTransportControlsSession>,
    player: String,
    want_artwork: bool,
    properties: Properties,
    last: TrackSnapshot,
    last_thumbnail_key: String,
    last_thumbnail: Option<String>,
    last_session_key: String,
    initialized: bool,
}

impl WindowsMedia {
    pub fn new(player: &str) -> Self {
        Self {
            manager: None,
            session: None,
            player: normalize_player(player),
            want_artwork: false,
            properties: Properties::default(),
            last: TrackSnapshot::empty(),
            last_thumbnail_key: String::new(),
            last_thumbnail: None,
            last_session_key: String::new(),
            initialized: false,
        }
    }

    fn ensure_manager(&mut self) -> Result<(), String> {
        if !self.initialized {
            // O worker roda numa thread propria; WinRT/SMTC espera um apartment
            // inicializado antes do primeiro RequestAsync.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            self.initialized = true;
        }
        if self.manager.is_none() {
            let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                .map_err(|error| error.to_string())?
                .join()
                .map_err(|error| error.to_string())?;
            self.manager = Some(manager);
        }
        Ok(())
    }

    fn pick_session(&self) -> Option<GlobalSystemMediaTransportControlsSession> {
        let manager = self.manager.as_ref()?;
        let sessions = manager.GetSessions().ok()?;
        let count = sessions.Size().unwrap_or(0);
        let mut list = Vec::with_capacity(count as usize);
        for index in 0..count {
            if let Ok(session) = sessions.GetAt(index) {
                list.push(session);
            }
        }

        let source = |session: &GlobalSystemMediaTransportControlsSession| {
            session
                .SourceAppUserModelId()
                .map(|value| value.to_string().to_ascii_lowercase())
                .unwrap_or_default()
        };
        let playing = |session: &GlobalSystemMediaTransportControlsSession| {
            session
                .GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
                .map(|status| {
                    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                })
                .unwrap_or(false)
        };

        let find = |token: &str| {
            list.iter()
                .find(|session| source(session).contains(token))
                .cloned()
        };
        let find_playing = |token: &str| {
            list.iter()
                .find(|session| source(session).contains(token) && playing(session))
                .cloned()
        };

        match self.player.as_str() {
            "spotify" => find("spotify"),
            "spotifast" => find("spotifast"),
            _ => find_playing("spotifast")
                .or_else(|| find_playing("spotify"))
                .or_else(|| find("spotifast"))
                .or_else(|| find("spotify"))
                .or_else(|| manager.GetCurrentSession().ok()),
        }
    }

    fn read_properties(
        &mut self,
        session: &GlobalSystemMediaTransportControlsSession,
        force: bool,
    ) -> Result<(), String> {
        let stale = self
            .properties
            .read_at
            .map(|at| at.elapsed() >= PROPERTIES_TTL)
            .unwrap_or(true);
        if !force && !stale {
            return Ok(());
        }

        let read = (|| -> Result<(), String> {
            let props = session
                .TryGetMediaPropertiesAsync()
                .map_err(|error| error.to_string())?
                .join()
                .map_err(|error| error.to_string())?;
            let title = props.Title().map(|v| v.to_string()).unwrap_or_default();
            let artist = props.Artist().map(|v| v.to_string()).unwrap_or_default();
            let album = props
                .AlbumTitle()
                .map(|v| v.to_string())
                .unwrap_or_default();
            if !title.is_empty() {
                self.properties.title = title;
            }
            if !artist.is_empty() {
                self.properties.artist = artist;
            }
            self.properties.album = album;

            let key = format!("{}|{}", self.properties.title, self.properties.album);
            if self.want_artwork && key != self.last_thumbnail_key {
                self.last_thumbnail = None;
                if let Ok(thumbnail) = props.Thumbnail() {
                    if let Some(data_url) = read_thumbnail(&thumbnail) {
                        self.last_thumbnail = Some(data_url);
                        self.last_thumbnail_key = key;
                    }
                }
            }
            self.properties.read_at = Some(Instant::now());
            Ok(())
        })();

        read
    }

    fn send(&self, command: MediaCommand) -> Result<(), String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| "nenhum player aberto".to_string())?;
        let result = match command {
            MediaCommand::Next => session.TrySkipNextAsync(),
            MediaCommand::Previous => session.TrySkipPreviousAsync(),
            MediaCommand::Toggle => session.TryTogglePlayPauseAsync(),
        };
        result
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

impl MediaBackend for WindowsMedia {
    fn snapshot(&mut self) -> TrackSnapshot {
        if let Err(error) = self.ensure_manager() {
            self.session = None;
            let mut snapshot = self.last.clone();
            snapshot.available = false;
            snapshot.playing = false;
            snapshot.error = format!("API de midia indisponivel: {error}");
            self.last = snapshot.clone();
            return snapshot;
        }

        let session = self.pick_session();
        self.session = session.clone();
        let Some(session) = session else {
            let mut snapshot = self.last.clone();
            snapshot.available = false;
            snapshot.playing = false;
            snapshot.error.clear();
            self.last = snapshot.clone();
            return snapshot;
        };

        let mut snapshot = self.last.clone();
        snapshot.available = true;
        snapshot.error.clear();
        snapshot.position_stamp = Some(Instant::now());

        let session_key = source_app_id(&session);
        let force = self.last_session_key != session_key;
        self.last_session_key = session_key;

        match session.GetPlaybackInfo() {
            Ok(info) => {
                snapshot.playing = info
                    .PlaybackStatus()
                    .map(|status| {
                        status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                    })
                    .unwrap_or(false);
            }
            Err(error) => {
                snapshot.error = error.to_string();
            }
        }

        match session.GetTimelineProperties() {
            Ok(timeline) => {
                let position = timeline.Position().map(span_seconds).unwrap_or(0.0);
                let end = timeline.EndTime().map(span_seconds).unwrap_or(0.0);
                // EndTime pode ser 0 em alguns players; nesse caso preserva a duracao.
                if end > 0.0 {
                    snapshot.duration = end;
                }
                if snapshot.duration > 0.0 {
                    snapshot.position = position.clamp(0.0, snapshot.duration);
                } else {
                    snapshot.position = position.max(0.0);
                }
            }
            Err(error) => {
                if snapshot.error.is_empty() {
                    snapshot.error = error.to_string();
                }
            }
        }

        if let Err(error) = self.read_properties(&session, force) {
            if snapshot.error.is_empty() {
                snapshot.error = error;
            }
        }

        if !self.properties.title.is_empty() {
            snapshot.title = self.properties.title.clone();
        }
        if !self.properties.artist.is_empty() {
            snapshot.artist = self.properties.artist.clone();
        }
        snapshot.album = self.properties.album.clone();
        snapshot.artwork_key = self.last_thumbnail_key.clone();
        snapshot.artwork = self.last_thumbnail.clone();

        self.last = snapshot.clone();
        snapshot
    }

    fn command(&mut self, command: MediaCommand) -> Result<(), String> {
        if self.session.is_none() {
            let _ = self.ensure_manager();
            self.session = self.pick_session();
        }
        let result = self.send(command);
        // Forca a releitura das propriedades no proximo poll: a faixa muda logo
        // depois do comando.
        self.properties.read_at = None;
        result
    }

    fn set_want_artwork(&mut self, want: bool) {
        if want && !self.want_artwork {
            self.last_thumbnail_key.clear();
        }
        self.want_artwork = want;
    }
}

fn normalize_player(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "spotify" => "spotify",
        "spotifast" => "spotifast",
        _ => "auto",
    }
    .into()
}

fn source_app_id(session: &GlobalSystemMediaTransportControlsSession) -> String {
    session
        .SourceAppUserModelId()
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn span_seconds(span: TimeSpan) -> f64 {
    span.Duration.max(0) as f64 / 10_000_000.0
}

fn read_thumbnail(reference: &IRandomAccessStreamReference) -> Option<String> {
    let stream = reference.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_THUMBNAIL_BYTES {
        return None;
    }
    let mime = stream
        .ContentType()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "image/png".into());
    let mime = if mime.is_empty() {
        "image/png".to_string()
    } else {
        mime
    };

    let input = stream.GetInputStreamAt(0).ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_player() {
        assert_eq!(normalize_player("Spotify"), "spotify");
        assert_eq!(normalize_player(" spotifast "), "spotifast");
        assert_eq!(normalize_player("web"), "auto");
    }

    #[test]
    fn time_span_is_converted_from_100ns() {
        assert!(
            (span_seconds(TimeSpan {
                Duration: 10_000_000
            }) - 1.0)
                .abs()
                < 0.0001
        );
        assert_eq!(span_seconds(TimeSpan { Duration: -5 }), 0.0);
    }
}
