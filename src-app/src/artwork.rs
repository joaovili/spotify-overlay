use base64::Engine;
use eframe::egui;
use spotify_overlay_core::NowPlaying;
use std::sync::mpsc::{self, Receiver, Sender};

pub struct Artwork {
    key: String,
    texture: Option<egui::TextureHandle>,
    pending: Option<String>,
    tx: Sender<(String, egui::ColorImage)>,
    rx: Receiver<(String, egui::ColorImage)>,
}

impl Artwork {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            key: String::new(),
            texture: None,
            pending: None,
            tx,
            rx,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, snapshot: &NowPlaying) {
        if snapshot.artwork_key != self.key {
            self.key = snapshot.artwork_key.clone();
            self.texture = None;
            self.pending = None;
        }

        if self.texture.is_none()
            && !self.key.is_empty()
            && self.pending.as_deref() != Some(self.key.as_str())
        {
            if let Some(source) = &snapshot.artwork {
                self.pending = Some(self.key.clone());
                let key = self.key.clone();
                let source = source.clone();
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    if let Some(image) = load_image(&source) {
                        tx.send((key, image)).ok();
                    }
                });
            }
        }

        while let Ok((key, image)) = self.rx.try_recv() {
            if key == self.key {
                self.texture = Some(ctx.load_texture(
                    format!("cover:{key}"),
                    image,
                    egui::TextureOptions::LINEAR,
                ));
                self.pending = None;
            }
        }
    }

    pub fn texture(&self) -> Option<&egui::TextureHandle> {
        self.texture.as_ref()
    }
}

fn load_image(source: &str) -> Option<egui::ColorImage> {
    let bytes = if let Some(rest) = source.strip_prefix("data:") {
        let (_, data) = rest.split_once(',')?;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .ok()?
    } else if source.starts_with("https://") || source.starts_with("http://") {
        fetch_remote(source)?
    } else {
        return None;
    };

    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        image.as_raw(),
    ))
}

#[cfg(target_os = "macos")]
fn fetch_remote(source: &str) -> Option<Vec<u8>> {
    use std::io::Read;

    let response = ureq::get(source)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(8 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

#[cfg(not(target_os = "macos"))]
fn fetch_remote(_source: &str) -> Option<Vec<u8>> {
    None
}
