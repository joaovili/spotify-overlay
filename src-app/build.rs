fn main() {
    println!("cargo:rerun-if-changed=../assets/icons/icon.ico");
    let host = std::env::var("HOST").unwrap_or_default();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && host.contains("windows")
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("../assets/icons/icon.ico");
        resource.set("ProductName", "Spotify Overlay");
        resource.set("FileDescription", "Spotify Overlay");
        resource.set(
            "ProductVersion",
            &std::env::var("CARGO_PKG_VERSION").unwrap_or_default(),
        );
        if let Err(error) = resource.compile() {
            println!("cargo:warning=falha ao embutir recursos do Windows: {error}");
        }
    }
}
