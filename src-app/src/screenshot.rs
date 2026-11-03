use eframe::egui;

pub fn save_png(path: &std::path::Path, capture: &egui::ColorImage) {
    let mut bytes = Vec::with_capacity(capture.pixels.len() * 4);
    for pixel in &capture.pixels {
        bytes.extend_from_slice(&pixel.to_array());
    }
    let _ = image::save_buffer(
        path,
        &bytes,
        capture.size[0] as u32,
        capture.size[1] as u32,
        image::ColorType::Rgba8,
    );
}
