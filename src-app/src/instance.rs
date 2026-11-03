use single_instance::SingleInstance;

/// Mantem o lock vivo enquanto o app existir. Se ja houver uma instancia,
/// retorna None e o chamador deve sair.
pub fn acquire() -> Option<SingleInstance> {
    let name = if cfg!(target_os = "macos") {
        std::env::temp_dir()
            .join("com.joaovili.SpotifyOverlay.lock")
            .to_string_lossy()
            .to_string()
    } else {
        "SpotifyOverlay-v2".to_string()
    };
    let instance = SingleInstance::new(&name).ok()?;
    if instance.is_single() {
        Some(instance)
    } else {
        None
    }
}
