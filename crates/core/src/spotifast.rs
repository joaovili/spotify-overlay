/// Saida de `Spotifast now-playing --raw`: uma linha separada por tabs, na ordem
/// state, title, artists, album, position_ms, duration_ms, volume, shuffle,
/// repeat, art_url, saved, device.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotifastNowPlaying {
    pub state: String,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub position: f64,
    pub duration: f64,
    pub artwork_url: Option<String>,
    pub device: String,
}

impl SpotifastNowPlaying {
    pub fn has_track(&self) -> bool {
        !self.title.is_empty()
    }
}

/// `None` quando a saida nao parece uma linha de now-playing valida.
pub fn parse(output: &str) -> Option<SpotifastNowPlaying> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    let fields: Vec<&str> = first_line.split('\t').collect();
    if fields.len() < 6 {
        return None;
    }

    let state = fields[0].trim().to_ascii_lowercase();
    if state.is_empty() {
        return None;
    }

    let position_ms = fields[4].trim().parse::<f64>().unwrap_or(0.0);
    let duration_ms = fields[5].trim().parse::<f64>().unwrap_or(0.0);
    let art = fields.get(9).map(|value| value.trim()).unwrap_or_default();

    Some(SpotifastNowPlaying {
        playing: state == "playing",
        state,
        title: fields[1].to_string(),
        artist: fields[2].to_string(),
        album: fields[3].to_string(),
        position: (position_ms / 1000.0).max(0.0),
        duration: (duration_ms / 1000.0).max(0.0),
        artwork_url: http_url(art),
        device: fields
            .get(11)
            .map(|value| value.to_string())
            .unwrap_or_default(),
    })
}

/// So aceita http(s): caminho local nao serve como capa.
fn http_url(text: &str) -> Option<String> {
    let lower = text.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(text.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "playing\tSong\tArtist\tAlbum\t12345\t180000\t80\tshuffle\toff\thttps://i.scdn.co/image/abc\ttrue\tDesktop";

    #[test]
    fn parses_full_line() {
        let now = parse(SAMPLE).unwrap();
        assert!(now.playing);
        assert_eq!(now.title, "Song");
        assert_eq!(now.artist, "Artist");
        assert_eq!(now.album, "Album");
        assert!((now.position - 12.345).abs() < 0.001);
        assert!((now.duration - 180.0).abs() < 0.001);
        assert_eq!(
            now.artwork_url.as_deref(),
            Some("https://i.scdn.co/image/abc")
        );
        assert_eq!(now.device, "Desktop");
    }

    #[test]
    fn rejects_noise() {
        assert!(parse("").is_none());
        assert!(parse("error: not found").is_none());
        assert!(parse("playing\tSong").is_none());
    }

    #[test]
    fn rejects_local_artwork_path() {
        let line = "paused\tSong\tArtist\tAlbum\t0\t1000\t0\toff\toff\t/tmp/a.png\tfalse\t";
        assert_eq!(parse(line).unwrap().artwork_url, None);
    }

    #[test]
    fn ignores_lines_after_the_first() {
        let line = format!("{SAMPLE}\nlog noise");
        assert_eq!(parse(&line).unwrap().title, "Song");
    }
}
