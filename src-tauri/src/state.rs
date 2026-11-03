use spotify_overlay_core::{Config, NowPlaying};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Mutex};

use crate::worker::WorkerMsg;

pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub visible: AtomicBool,
    pub last: Mutex<Option<NowPlaying>>,
    pub worker: Mutex<Option<Sender<WorkerMsg>>>,
}

impl AppState {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        Self {
            config: Mutex::new(config),
            config_path,
            visible: AtomicBool::new(true),
            last: Mutex::new(None),
            worker: Mutex::new(None),
        }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn set_config(&self, config: Config) {
        *self.config.lock().unwrap() = config;
    }

    pub fn send(&self, message: WorkerMsg) -> bool {
        match self.worker.lock().unwrap().as_ref() {
            Some(sender) => sender.send(message).is_ok(),
            None => false,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }

    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Relaxed);
    }

    pub fn last_snapshot(&self) -> Option<NowPlaying> {
        self.last.lock().unwrap().clone()
    }
}
