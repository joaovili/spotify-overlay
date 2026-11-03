use auto_launch::{AutoLaunch, AutoLaunchBuilder};

pub struct Autostart {
    launcher: Option<AutoLaunch>,
}

impl Autostart {
    pub fn new() -> Self {
        let launcher = (|| {
            let executable = std::env::current_exe().ok()?;
            let path = executable.to_str()?;
            let mut builder = AutoLaunchBuilder::new();
            builder.set_app_name("Spotify Overlay").set_app_path(path);
            #[cfg(target_os = "macos")]
            builder.set_macos_launch_mode(auto_launch::MacOSLaunchMode::SMAppService);
            #[cfg(target_os = "windows")]
            builder.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
            builder.build().ok()
        })();
        Self { launcher }
    }

    pub fn apply(&self, enabled: bool) {
        let Some(launcher) = &self.launcher else {
            return;
        };
        let _ = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
    }
}
