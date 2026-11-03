use spotify_overlay_core::{
    interval_ms, Config, MediaCommand, NowPlaying, PlaybackClock, TrackSnapshot,
};
use spotify_overlay_media::{create, MediaBackend};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

#[derive(Debug, Clone)]
pub enum WorkerMsg {
    Control(MediaCommand),
    Refresh,
    SetPlayer(String),
    SetWantArtwork(bool),
}

/// Le o player numa thread propria. Comandos chegam por canal para que o backend
/// (COM/SMTC no Windows) sempre rode no mesmo thread em que foi criado.
pub fn spawn(app: AppHandle, config: Config) -> Sender<WorkerMsg> {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    let initial_player = config.player.clone();
    let initial_artwork = config.size == "large";

    std::thread::Builder::new()
        .name("spotify-overlay-media".into())
        .spawn(move || run(app, rx, initial_player, initial_artwork))
        .expect("falha ao iniciar a thread de midia");

    tx
}

fn run(
    app: AppHandle,
    rx: mpsc::Receiver<WorkerMsg>,
    initial_player: String,
    initial_artwork: bool,
) {
    let mut player = initial_player;
    let mut backend: Box<dyn MediaBackend> = create(&player);
    backend.set_want_artwork(initial_artwork);
    let mut clock = PlaybackClock::new();
    let mut last_artwork_key = String::new();

    loop {
        let snapshot = backend.snapshot();
        publish(&app, &snapshot, &mut clock, &mut last_artwork_key);

        let visible = app.state::<AppState>().is_visible();
        let interval = Duration::from_millis(interval_ms(visible));

        match rx.recv_timeout(interval) {
            Ok(WorkerMsg::Control(command)) => {
                let _ = backend.command(command);
                // O player leva alguns milissegundos para publicar a nova faixa.
                std::thread::sleep(Duration::from_millis(150));
            }
            Ok(WorkerMsg::Refresh) => {}
            Ok(WorkerMsg::SetPlayer(next)) => {
                if next != player {
                    player = next;
                    backend = create(&player);
                    clock.reset();
                    last_artwork_key.clear();
                }
            }
            Ok(WorkerMsg::SetWantArtwork(want)) => backend.set_want_artwork(want),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn publish(
    app: &AppHandle,
    snapshot: &TrackSnapshot,
    clock: &mut PlaybackClock,
    last_artwork_key: &mut String,
) {
    let full = NowPlaying::from_snapshot(snapshot, clock, Instant::now());
    let mut outgoing = full.clone();

    // Capa pode ser um data URL grande (Windows/SMTC). Reenvia so quando muda;
    // a interface mantem a ultima imagem recebida.
    if !full.artwork_key.is_empty() && full.artwork_key == *last_artwork_key {
        outgoing.artwork = None;
    } else if !full.artwork_key.is_empty() {
        *last_artwork_key = full.artwork_key.clone();
    }

    if let Some(state) = app.try_state::<AppState>() {
        *state.last.lock().unwrap() = Some(full);
    }
    let _ = app.emit("state", &outgoing);
}
