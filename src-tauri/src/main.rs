#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod hotkeys;
mod menu;
mod platform;
mod state;
mod tray;
mod window;
mod worker;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--version") {
        println!("Spotify Overlay {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|arg| arg == "--selftest-core") {
        std::process::exit(selftest_core());
    }

    if args.iter().any(|arg| arg == "--dump-player") {
        let mut backend = spotify_overlay_media::create("auto");
        let snapshot = backend.snapshot();
        println!(
            "available={} playing={} title={:?} artist={:?} album={:?} position={:.2} duration={:.2} artwork_key={:?} artwork={} error={:?}",
            snapshot.available,
            snapshot.playing,
            snapshot.title,
            snapshot.artist,
            snapshot.album,
            snapshot.position,
            snapshot.duration,
            snapshot.artwork_key,
            snapshot.artwork.is_some(),
            snapshot.error
        );
        return;
    }

    app::run();
}

fn selftest_core() -> i32 {
    use spotify_overlay_core::{hotkey, layout, spotifast, Config};

    let mut failures: Vec<String> = Vec::new();

    let config = Config::default();
    if config.size != "medium" || config.player != "auto" {
        failures.push("defaults".into());
    }
    match Config::parse(&config.serialized()) {
        Ok(parsed) if parsed == config => {}
        _ => failures.push("config roundtrip".into()),
    }

    if layout::OverlaySize::Large.dimensions() != (440.0, 148.0) {
        failures.push("layout large".into());
    }

    match hotkey::parse("Ctrl+Alt+Right") {
        Ok(spec) if spec.accelerator() == "Control+Alt+ArrowRight" => {}
        _ => failures.push("hotkey parse".into()),
    }

    if spotifast::parse("playing\tTitle\tArtist\tAlbum\t1000\t2000\t50\toff\toff\thttps://i.scdn.co/x\tfalse\tDesktop")
        .map(|track| track.playing && track.title == "Title")
        != Some(true)
    {
        failures.push("spotifast parse".into());
    }

    if failures.is_empty() {
        println!("selftest-core: ok");
        0
    } else {
        eprintln!("selftest-core: falhou ({})", failures.join(", "));
        1
    }
}
