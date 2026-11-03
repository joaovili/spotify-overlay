use crate::MediaBackend;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AllocAnyThread;
use objc2_foundation::{
    NSAppleScript, NSAppleScriptErrorMessage, NSAppleScriptErrorNumber, NSDictionary, NSNumber,
    NSString,
};
use spotify_overlay_core::{spotifast, MediaCommand, TrackSnapshot};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

const SPOTIFY_SCRIPT: &str = r#"
if application "Spotify" is running then
    tell application "Spotify"
        set stateText to "stopped"
        try
            set stateText to (player state as string)
        end try
        if stateText is "stopped" then return "running" & tab & stateText
        set titleText to ""
        try
            set titleText to (name of current track)
        end try
        set artistText to ""
        try
            set artistText to (artist of current track)
        end try
        set albumText to ""
        try
            set albumText to (album of current track)
        end try
        set durationText to "0"
        try
            set durationText to ((duration of current track) as string)
        end try
        set positionText to "0"
        try
            set positionText to ((player position) as string)
        end try
        set artText to ""
        try
            set artText to (artwork url of current track)
        end try
        return "running" & tab & stateText & tab & titleText & tab & artistText & tab & albumText & tab & durationText & tab & positionText & tab & artText
    end tell
else
    return "notrunning"
end if
"#;

const SCRIPT_UNAVAILABLE: &str = "script indisponivel";
const SPOTIFAST_FALLBACK: &str = "/Applications/Spotifast.app/Contents/MacOS/Spotifast";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Player {
    Spotify,
    Spotifast,
}

struct Config {
    player: String,
}

/// NSAppleScript executa no proprio processo: sem `osascript`, sem subprocesso por
/// poll e o pedido de Automacao aparece no nome do app. O worker do Tauri cria e usa
/// este objeto sempre no mesmo thread.
struct AppleScript {
    script: Option<Retained<NSAppleScript>>,
}

unsafe impl Send for AppleScript {}

impl AppleScript {
    fn new(source: &str) -> Self {
        let source = NSString::from_str(source);
        Self {
            script: NSAppleScript::initWithSource(NSAppleScript::alloc(), &source),
        }
    }

    fn execute(&self) -> Result<String, String> {
        let Some(script) = &self.script else {
            return Err(SCRIPT_UNAVAILABLE.into());
        };

        let mut error: Option<Retained<NSDictionary<NSString, AnyObject>>> = None;
        let result = unsafe { script.executeAndReturnError(Some(&mut error)) };
        if let Some(error) = error {
            let number = error
                .objectForKey(unsafe { NSAppleScriptErrorNumber })
                .and_then(|value| value.downcast::<NSNumber>().ok())
                .map(|number| number.intValue())
                .unwrap_or(0);
            let message = error
                .objectForKey(unsafe { NSAppleScriptErrorMessage })
                .and_then(|value| value.downcast::<NSString>().ok())
                .map(|message| message.to_string())
                .unwrap_or_else(|| "erro AppleScript".into());
            return Err(format!("{number}: {message}"));
        }

        Ok(result
            .stringValue()
            .map(|value| value.to_string())
            .unwrap_or_default())
    }
}

pub struct MacMedia {
    config: Config,
    selected: Option<Player>,
    last: TrackSnapshot,
    last_spotifast: TrackSnapshot,
    executable: Option<PathBuf>,
    spotify_script: AppleScript,
}

impl MacMedia {
    pub fn new(player: &str) -> Self {
        Self {
            config: Config {
                player: normalize_player(player),
            },
            selected: None,
            last: TrackSnapshot::empty(),
            last_spotifast: TrackSnapshot::empty(),
            executable: None,
            spotify_script: AppleScript::new(SPOTIFY_SCRIPT),
        }
    }

    fn read_spotify(&self, previous: &TrackSnapshot) -> TrackSnapshot {
        match self.spotify_script.execute() {
            Ok(output) => parse_spotify_output(&output, previous),
            Err(error) if error == SCRIPT_UNAVAILABLE => TrackSnapshot {
                available: false,
                playing: false,
                error: String::new(),
                ..previous.clone()
            },
            Err(error) => TrackSnapshot {
                available: true,
                playing: false,
                error,
                ..previous.clone()
            },
        }
    }

    fn command_for(&mut self, command: MediaCommand) -> Result<(), String> {
        match self.selected {
            Some(Player::Spotify) => send_spotify(command),
            Some(Player::Spotifast) => self.send_spotifast(command),
            None => {
                if spotifast_running() {
                    self.send_spotifast(command)
                } else {
                    send_spotify(command)
                }
            }
        }
    }

    fn send_spotifast(&mut self, command: MediaCommand) -> Result<(), String> {
        let executable = spotifast_executable(&mut self.executable)
            .ok_or_else(|| "Spotifast nao encontrado".to_string())?;
        let verb = match command {
            MediaCommand::Next => "next",
            MediaCommand::Previous => "previous",
            MediaCommand::Toggle => "play-pause",
        };
        let output = run_with_timeout(
            &executable.to_string_lossy(),
            &[verb],
            Duration::from_secs(2),
        )
        .ok_or_else(|| "Spotifast nao respondeu".to_string())?;
        if output.status != 0 {
            return Err(stderr_message(&output.stderr, "Spotifast retornou erro"));
        }
        Ok(())
    }
}

impl MediaBackend for MacMedia {
    fn snapshot(&mut self) -> TrackSnapshot {
        let snapshot = match self.config.player.as_str() {
            "spotify" => {
                let snapshot = self.read_spotify(&self.last);
                self.selected = if snapshot.available {
                    Some(Player::Spotify)
                } else {
                    None
                };
                snapshot
            }
            "spotifast" => {
                self.selected = if spotifast_running() {
                    Some(Player::Spotifast)
                } else {
                    None
                };
                let snapshot = read_spotifast(&mut self.executable, &self.last_spotifast);
                self.last_spotifast = snapshot.clone();
                snapshot
            }
            _ => self.auto_snapshot(),
        };

        let mut merged = self.last.clone();
        merged.available = snapshot.available;
        merged.error = snapshot.error;
        merged.playing = snapshot.playing;
        if snapshot.available {
            merged.position = snapshot.position;
            merged.position_stamp = snapshot.position_stamp;
        }
        if !snapshot.title.is_empty() {
            merged.title = snapshot.title;
        }
        if !snapshot.artist.is_empty() {
            merged.artist = snapshot.artist;
        }
        merged.album = snapshot.album;
        if snapshot.duration > 0.0 {
            merged.duration = snapshot.duration;
        }
        merged.artwork_key = snapshot.artwork_key;
        merged.artwork = snapshot.artwork;

        self.last = merged.clone();
        merged
    }

    fn command(&mut self, command: MediaCommand) -> Result<(), String> {
        self.command_for(command)
    }
}

impl MacMedia {
    fn auto_snapshot(&mut self) -> TrackSnapshot {
        // O proprio script detecta se o Spotify esta aberto, entao podemos
        // consulta-lo sem abrir o app (o `tell` so roda dentro do `if running`).
        let official = self.read_spotify(&self.last);
        let fast = if spotifast_running() {
            let snapshot = read_spotifast(&mut self.executable, &self.last_spotifast);
            self.last_spotifast = snapshot.clone();
            snapshot
        } else {
            TrackSnapshot {
                available: false,
                ..TrackSnapshot::empty()
            }
        };

        if fast.playing {
            self.selected = Some(Player::Spotifast);
            return fast;
        }
        if official.playing {
            self.selected = Some(Player::Spotify);
            return official;
        }
        if fast.available {
            self.selected = Some(Player::Spotifast);
            return fast;
        }
        if official.available {
            self.selected = Some(Player::Spotify);
            return official;
        }

        self.selected = None;
        TrackSnapshot {
            available: false,
            playing: false,
            error: String::new(),
            ..TrackSnapshot::empty()
        }
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

fn spotifast_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "-i", "Spotifast"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn parse_spotify_output(output: &str, previous: &TrackSnapshot) -> TrackSnapshot {
    let line = output.trim();
    if line.is_empty() || line == "notrunning" {
        return TrackSnapshot {
            available: false,
            playing: false,
            error: String::new(),
            ..previous.clone()
        };
    }

    let fields: Vec<&str> = line.split('\t').collect();
    if fields.first().copied() != Some("running") {
        return TrackSnapshot {
            available: false,
            playing: false,
            error: String::new(),
            ..previous.clone()
        };
    }

    let state = fields.get(1).map(|value| value.trim()).unwrap_or("stopped");
    let playing = state == "playing" || state == "kPSP";
    let mut snapshot = previous.clone();
    snapshot.available = true;
    snapshot.playing = playing;
    snapshot.error = String::new();
    snapshot.position_stamp = Some(Instant::now());

    if fields.len() >= 8 {
        snapshot.title = fields[2].to_string();
        snapshot.artist = fields[3].to_string();
        snapshot.album = fields[4].to_string();
        snapshot.duration = fields[5].trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
        snapshot.position = fields[6].trim().parse::<f64>().unwrap_or(0.0);
        snapshot.artwork_key = fields[7].trim().to_string();
        snapshot.artwork = allowed_artwork_url(fields[7].trim());
        if snapshot.duration <= 0.0 {
            snapshot.duration = previous.duration;
        }
    }
    snapshot
}

fn read_spotifast(executable: &mut Option<PathBuf>, previous: &TrackSnapshot) -> TrackSnapshot {
    if !spotifast_running() {
        return TrackSnapshot {
            available: false,
            playing: false,
            error: String::new(),
            ..previous.clone()
        };
    }

    let Some(binary) = spotifast_executable(executable) else {
        return TrackSnapshot {
            available: true,
            playing: false,
            error: "Spotifast nao encontrado".into(),
            ..previous.clone()
        };
    };

    let output = match run_with_timeout(
        &binary.to_string_lossy(),
        &["now-playing", "--raw"],
        Duration::from_secs(2),
    ) {
        Some(output) => output,
        None => {
            return TrackSnapshot {
                available: true,
                playing: false,
                error: "Spotifast nao respondeu".into(),
                ..previous.clone()
            }
        }
    };

    if output.status != 0 {
        return TrackSnapshot {
            available: true,
            playing: false,
            error: stderr_message(&output.stderr, "Spotifast retornou erro"),
            ..previous.clone()
        };
    }

    let Some(track) = spotifast::parse(&output.stdout) else {
        return TrackSnapshot {
            available: true,
            playing: false,
            error: "resposta inesperada do Spotifast".into(),
            ..previous.clone()
        };
    };

    TrackSnapshot {
        title: track.title,
        artist: track.artist,
        album: track.album,
        playing: track.playing,
        available: true,
        position: track.position,
        duration: track.duration,
        position_stamp: Some(Instant::now()),
        error: String::new(),
        artwork_key: track.artwork_url.clone().unwrap_or_default(),
        artwork: track.artwork_url.and_then(|url| allowed_artwork_url(&url)),
    }
}

fn spotifast_executable(cache: &mut Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = cache {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // O binario dentro do .app e o mesmo que a CLI instala. Preferir o caminho
    // fixo evita depender do PATH de um app aberto pelo Finder.
    let fallback = PathBuf::from(SPOTIFAST_FALLBACK);
    if fallback.is_file() {
        *cache = Some(fallback.clone());
        return Some(fallback);
    }

    if let Ok(path) = which("Spotifast").or_else(|_| which("spotifast")) {
        *cache = Some(path.clone());
        return Some(path);
    }

    None
}

fn which(name: &str) -> Result<PathBuf, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(())
}

fn send_spotify(command: MediaCommand) -> Result<(), String> {
    let verb = match command {
        MediaCommand::Next => "next track",
        MediaCommand::Previous => "previous track",
        MediaCommand::Toggle => "playpause",
    };
    let source = format!(
        "if application \"Spotify\" is running then\n tell application \"Spotify\" to {verb}\nelse\n return \"notrunning\"\nend if"
    );
    let script = AppleScript::new(&source);
    match script.execute() {
        Ok(output) if output.trim() == "notrunning" => Err("Spotify nao esta aberto".into()),
        Ok(_) => Ok(()),
        Err(error) if error == SCRIPT_UNAVAILABLE => Err("Spotify nao esta aberto".into()),
        Err(error) => Err(error),
    }
}

fn allowed_artwork_url(url: &str) -> Option<String> {
    let lower = url.trim().to_ascii_lowercase();
    if !lower.starts_with("https://") {
        return None;
    }
    let host = lower
        .trim_start_matches("https://")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if host == "i.scdn.co" {
        Some(url.trim().to_string())
    } else {
        None
    }
}

#[derive(Debug)]
struct ProcessOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<ProcessOutput> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let status = match child.wait_timeout(timeout).ok()? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }

    Some(ProcessOutput {
        status: status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

fn stderr_message(stderr: &str, fallback: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Usado apenas para testes e diagnostico.
pub fn parse_output_for_tests(output: &str) -> TrackSnapshot {
    parse_spotify_output(output, &TrackSnapshot::empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spotify_overlay_core::TrackSnapshot;

    #[test]
    fn parses_running_playing_track() {
        let line =
            "running\tplaying\tSong\tArtist\tAlbum\t180000\t12.5\thttps://i.scdn.co/image/abc";
        let snapshot = parse_output_for_tests(line);
        assert!(snapshot.available);
        assert!(snapshot.playing);
        assert_eq!(snapshot.title, "Song");
        assert_eq!(snapshot.artist, "Artist");
        assert_eq!(snapshot.album, "Album");
        assert!((snapshot.duration - 180.0).abs() < 0.001);
        assert!((snapshot.position - 12.5).abs() < 0.001);
        assert_eq!(snapshot.artwork_key, "https://i.scdn.co/image/abc");
    }

    #[test]
    fn not_running_is_not_available_and_not_an_error() {
        let snapshot = parse_output_for_tests("notrunning");
        assert!(!snapshot.available);
        assert!(snapshot.error.is_empty());
    }

    #[test]
    fn stopped_keeps_available() {
        let snapshot = parse_output_for_tests("running\tstopped");
        assert!(snapshot.available);
        assert!(!snapshot.playing);
        assert!(snapshot.error.is_empty());
    }

    #[test]
    fn keeps_previous_title_when_stopped() {
        let previous = TrackSnapshot {
            title: "Old".into(),
            artist: "Artist".into(),
            duration: 90.0,
            ..TrackSnapshot::empty()
        };
        let snapshot = parse_spotify_output("running\tstopped", &previous);
        assert_eq!(snapshot.title, "Old");
        assert_eq!(snapshot.artist, "Artist");
    }

    #[test]
    fn only_accepts_the_spotify_cdn() {
        assert_eq!(
            allowed_artwork_url("https://i.scdn.co/image/abc").as_deref(),
            Some("https://i.scdn.co/image/abc")
        );
        assert_eq!(allowed_artwork_url("https://evil.example/a.png"), None);
        assert_eq!(allowed_artwork_url("http://i.scdn.co/a.png"), None);
    }
}
