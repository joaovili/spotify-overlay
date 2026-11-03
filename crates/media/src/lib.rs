//! Leitura e controle dos players locais. Cada plataforma tem seu backend, mas a
//! interface e a mesma para o resto do app.

use spotify_overlay_core::{MediaCommand, TrackSnapshot};

pub trait MediaBackend: Send {
    fn snapshot(&mut self) -> TrackSnapshot;
    fn command(&mut self, command: MediaCommand) -> Result<(), String>;

    /// O layout grande e o unico que mostra a capa. No Windows isso evita ler o
    /// thumbnail do SMTC quando ele nao sera exibido.
    fn set_want_artwork(&mut self, _want: bool) {}
}

pub fn create(player: &str) -> Box<dyn MediaBackend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacMedia::new(player))
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsMedia::new(player))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = player;
        Box::new(UnsupportedMedia)
    }
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct UnsupportedMedia;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl MediaBackend for UnsupportedMedia {
    fn snapshot(&mut self) -> TrackSnapshot {
        TrackSnapshot::empty()
    }

    fn command(&mut self, _command: MediaCommand) -> Result<(), String> {
        Err("plataforma nao suportada".into())
    }
}
