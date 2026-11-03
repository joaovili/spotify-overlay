use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuracao do app. Mantem o mesmo formato e as mesmas chaves das versoes antigas
/// (macOS/Windows), para que o `config.json` continue compativel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub size: String,
    pub position: String,
    pub custom_x: i32,
    pub custom_y: i32,
    pub hotkey_skip: String,
    pub hotkey_prev: String,
    pub hotkey_toggle: String,
    pub always_on_top: bool,
    pub close_hides_to_tray: bool,
    /// No macOS significa iniciar no login; no Windows, iniciar com o Windows.
    pub start_with_windows: bool,
    /// `auto`, `spotify` ou `spotifast`.
    pub player: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: "medium".into(),
            position: "top-right".into(),
            custom_x: 1500,
            custom_y: 24,
            hotkey_skip: "Ctrl+Alt+Right".into(),
            hotkey_prev: "Ctrl+Alt+Left".into(),
            hotkey_toggle: "Ctrl+Alt+H".into(),
            always_on_top: true,
            close_hides_to_tray: true,
            start_with_windows: false,
            player: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidJson,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config: Config =
            serde_json::from_str(text).map_err(|_| ConfigError::InvalidJson)?;
        config.normalize();
        Ok(config)
    }

    /// Le o arquivo; arquivo ausente, vazio ou corrompido volta aos padroes e regrava.
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) if !text.trim().is_empty() => match Config::parse(&text) {
                Ok(config) => config,
                Err(_) => {
                    let config = Config::default();
                    let _ = config.save(path);
                    config
                }
            },
            _ => {
                let config = Config::default();
                let _ = config.save(path);
                config
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        text.push('\n');
        fs::write(path, text)
    }

    pub fn serialized(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Aplica os limites aceitos. Atalhos vazios sao validos (desligam o atalho).
    pub fn normalize(&mut self) {
        self.size = match self.size.trim().to_ascii_lowercase().as_str() {
            "mini" => "mini",
            "large" => "large",
            _ => "medium",
        }
        .into();

        self.position = match self.position.trim().to_ascii_lowercase().as_str() {
            "top-left" => "top-left",
            "bottom-right" => "bottom-right",
            "bottom-left" => "bottom-left",
            "custom" => "custom",
            _ => "top-right",
        }
        .into();

        self.player = match self.player.trim().to_ascii_lowercase().as_str() {
            "spotify" => "spotify",
            "spotifast" => "spotifast",
            _ => "auto",
        }
        .into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_old_apps() {
        let config = Config::default();
        assert_eq!(config.size, "medium");
        assert_eq!(config.position, "top-right");
        assert_eq!(config.hotkey_skip, "Ctrl+Alt+Right");
        assert_eq!(config.hotkey_prev, "Ctrl+Alt+Left");
        assert_eq!(config.hotkey_toggle, "Ctrl+Alt+H");
        assert!(config.always_on_top);
        assert!(config.close_hides_to_tray);
        assert!(!config.start_with_windows);
        assert_eq!(config.player, "auto");
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let config = Config::parse("{}").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn empty_hotkey_disables_it() {
        let config = Config::parse(r#"{"hotkeySkip":""}"#).unwrap();
        assert_eq!(config.hotkey_skip, "");
        assert_eq!(config.hotkey_toggle, "Ctrl+Alt+H");
    }

    #[test]
    fn invalid_values_are_normalized() {
        let config =
            Config::parse(r#"{"size":"huge","position":"middle","player":"web","customX":"nope"}"#);
        // customX com tipo errado invalida o documento inteiro e cai no padrao no load.
        assert!(config.is_err());

        let config =
            Config::parse(r#"{"size":"huge","position":"middle","player":"web"}"#).unwrap();
        assert_eq!(config.size, "medium");
        assert_eq!(config.position, "top-right");
        assert_eq!(config.player, "auto");
    }

    #[test]
    fn serialized_has_same_key_order_as_old_apps() {
        let text = Config::default().serialized();
        let keys = [
            "size",
            "position",
            "customX",
            "customY",
            "hotkeySkip",
            "hotkeyPrev",
            "hotkeyToggle",
            "alwaysOnTop",
            "closeHidesToTray",
            "startWithWindows",
            "player",
        ];
        let mut previous = 0;
        for key in keys {
            let at = text.find(&format!("\"{key}\"")).expect(key);
            assert!(at > previous, "{key} fora de ordem");
            previous = at;
        }
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn roundtrip() {
        let mut config = Config::default();
        config.size = "large".into();
        config.position = "custom".into();
        config.custom_x = 10;
        config.custom_y = 20;
        config.hotkey_skip = String::new();
        let parsed = Config::parse(&config.serialized()).unwrap();
        assert_eq!(parsed, config);
    }
}
