use eframe::egui;
use spotify_overlay_core::{interval_ms, Config, MediaCommand, NowPlaying, PlaybackClock};
use spotify_overlay_media::{create, MediaBackend};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum WorkerMsg {
    Control(MediaCommand),
    Refresh,
    SetPlayer(String),
    SetWantArtwork(bool),
}

pub struct Worker {
    pub tx: Sender<WorkerMsg>,
    pub latest: Arc<Mutex<Option<NowPlaying>>>,
    pub visible: Arc<AtomicBool>,
}

pub fn spawn(config: &Config, ctx: egui::Context) -> Worker {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    let latest = Arc::new(Mutex::new(None));
    let visible = Arc::new(AtomicBool::new(true));

    let player = config.player.clone();
    let want_artwork = config.size == "large";
    let latest_clone = Arc::clone(&latest);
    let visible_clone = Arc::clone(&visible);

    std::thread::Builder::new()
        .name("spotify-overlay-media".into())
        .spawn(move || run(rx, ctx, latest_clone, visible_clone, player, want_artwork))
        .expect("falha ao iniciar a thread de midia");

    Worker {
        tx,
        latest,
        visible,
    }
}

fn run(
    rx: mpsc::Receiver<WorkerMsg>,
    ctx: egui::Context,
    latest: Arc<Mutex<Option<NowPlaying>>>,
    visible: Arc<AtomicBool>,
    initial_player: String,
    initial_artwork: bool,
) {
    let mut player = initial_player;
    let mut backend: Box<dyn MediaBackend> = create(&player);
    backend.set_want_artwork(initial_artwork);
    let mut clock = PlaybackClock::new();

    loop {
        let snapshot = backend.snapshot();
        let now = NowPlaying::from_snapshot(&snapshot, &mut clock, Instant::now());
        *latest.lock().unwrap() = Some(now);
        ctx.request_repaint();

        let visible = visible.load(Ordering::Relaxed);
        let interval = Duration::from_millis(interval_ms(visible));

        match rx.recv_timeout(interval) {
            Ok(WorkerMsg::Control(command)) => {
                let _ = backend.command(command);
                std::thread::sleep(Duration::from_millis(150));
            }
            Ok(WorkerMsg::Refresh) => {}
            Ok(WorkerMsg::SetPlayer(next)) => {
                if next != player {
                    player = next;
                    backend = create(&player);
                    clock.reset();
                }
            }
            Ok(WorkerMsg::SetWantArtwork(want)) => backend.set_want_artwork(want),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}
