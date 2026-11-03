//! Plataforma-agnostico: configuracao, estado de reproducao, layout, atalhos e textos.

pub mod config;
pub mod hotkey;
pub mod layout;
pub mod playback;
pub mod presentation;
pub mod spotifast;
pub mod time;

pub use config::Config;
pub use hotkey::{HotkeyError, HotkeySpec, Key, Modifier};
pub use layout::{OverlaySize, ScreenPosition};
pub use playback::{interval_ms, MediaCommand, NowPlaying, PlaybackClock, TrackSnapshot};
pub use presentation::Presentation;
