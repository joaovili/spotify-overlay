use std::fmt;

/// Atalho no formato historico do projeto (`Ctrl+Alt+Right`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    /// Cmd no macOS, Win no Windows.
    Meta,
}

impl Modifier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
            Self::Meta => "Cmd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Space,
    Enter,
    Tab,
    Escape,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Backspace,
    ForwardDelete,
    F(u8),
    MediaNext,
    MediaPrev,
    MediaPlayPause,
}

impl Key {
    /// Nome de `Code` usado pelo parser de aceleradores do global-hotkey.
    pub fn code(self) -> String {
        match self {
            Self::Char(c) if c.is_ascii_alphabetic() => format!("Key{}", c.to_ascii_uppercase()),
            Self::Char(c) if c.is_ascii_digit() => format!("Digit{c}"),
            Self::Char(c) => match c {
                '-' => "Minus",
                '=' => "Equal",
                '[' => "BracketLeft",
                ']' => "BracketRight",
                '\\' => "Backslash",
                ';' => "Semicolon",
                '\'' => "Quote",
                ',' => "Comma",
                '.' => "Period",
                '/' => "Slash",
                '`' => "Backquote",
                _ => "Space",
            }
            .into(),
            Self::Left => "ArrowLeft".into(),
            Self::Right => "ArrowRight".into(),
            Self::Up => "ArrowUp".into(),
            Self::Down => "ArrowDown".into(),
            Self::Space => "Space".into(),
            Self::Enter => "Enter".into(),
            Self::Tab => "Tab".into(),
            Self::Escape => "Escape".into(),
            Self::Home => "Home".into(),
            Self::End => "End".into(),
            Self::PageUp => "PageUp".into(),
            Self::PageDown => "PageDown".into(),
            Self::Delete => "Delete".into(),
            Self::Backspace => "Backspace".into(),
            Self::ForwardDelete => "Delete".into(),
            Self::F(n) => format!("F{n}"),
            Self::MediaNext => "MediaTrackNext".into(),
            Self::MediaPrev => "MediaTrackPrevious".into(),
            Self::MediaPlayPause => "MediaPlayPause".into(),
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Char(c) => c.to_ascii_uppercase().to_string(),
            Self::Left => "Left".into(),
            Self::Right => "Right".into(),
            Self::Up => "Up".into(),
            Self::Down => "Down".into(),
            Self::Space => "Space".into(),
            Self::Enter => "Enter".into(),
            Self::Tab => "Tab".into(),
            Self::Escape => "Esc".into(),
            Self::Home => "Home".into(),
            Self::End => "End".into(),
            Self::PageUp => "PgUp".into(),
            Self::PageDown => "PgDn".into(),
            Self::Delete => "Delete".into(),
            Self::Backspace => "Backspace".into(),
            Self::ForwardDelete => "ForwardDelete".into(),
            Self::F(n) => format!("F{n}"),
            Self::MediaNext => "MediaNextTrack".into(),
            Self::MediaPrev => "MediaPrevTrack".into(),
            Self::MediaPlayPause => "MediaPlayPause".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeySpec {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

impl HotkeySpec {
    pub fn accelerator(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|modifier| match modifier {
                Modifier::Ctrl => "Control".to_string(),
                Modifier::Alt => "Alt".to_string(),
                Modifier::Shift => "Shift".to_string(),
                Modifier::Meta => "Super".to_string(),
            })
            .collect();
        parts.push(self.key.code());
        parts.join("+")
    }

    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|modifier| modifier.label().to_string())
            .collect();
        parts.push(self.key.label());
        parts.join("+")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    Empty,
    MissingModifier,
    UnknownKey(String),
}

impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "atalho vazio"),
            Self::MissingModifier => {
                write!(f, "use ao menos um modificador (Ctrl, Alt, Shift ou Cmd)")
            }
            Self::UnknownKey(key) => write!(f, "tecla desconhecida: {key}"),
        }
    }
}

impl std::error::Error for HotkeyError {}

/// `None` para atalho vazio (desligado); `Some(Err)` para atalho invalido.
pub fn parse_optional(spec: &str) -> Option<Result<HotkeySpec, HotkeyError>> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(parse(trimmed))
    }
}

pub fn parse(spec: &str) -> Result<HotkeySpec, HotkeyError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(HotkeyError::Empty);
    }

    let parts: Vec<&str> = trimmed.split('+').map(str::trim).collect();
    let last = parts.last().copied().unwrap_or_default();
    if last.is_empty() {
        return Err(HotkeyError::UnknownKey(trimmed.into()));
    }

    let mut modifiers: Vec<Modifier> = Vec::new();
    for token in &parts[..parts.len() - 1] {
        let modifier = match token.to_ascii_uppercase().as_str() {
            "CMD" | "COMMAND" | "WIN" | "SUPER" | "META" => Modifier::Meta,
            "CTRL" | "CONTROL" => Modifier::Ctrl,
            "ALT" | "OPTION" | "OPT" => Modifier::Alt,
            "SHIFT" => Modifier::Shift,
            _ => continue,
        };
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }

    if modifiers.is_empty() {
        return Err(HotkeyError::MissingModifier);
    }

    let key = parse_key(last)?;
    Ok(HotkeySpec { modifiers, key })
}

fn parse_key(raw: &str) -> Result<Key, HotkeyError> {
    let name = raw.trim().to_ascii_uppercase();
    let key = match name.as_str() {
        "LEFT" => Key::Left,
        "RIGHT" => Key::Right,
        "UP" => Key::Up,
        "DOWN" => Key::Down,
        "SPACE" | "SPACEBAR" => Key::Space,
        "ENTER" | "RETURN" => Key::Enter,
        "TAB" => Key::Tab,
        "ESC" | "ESCAPE" => Key::Escape,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PGUP" | "PAGEUP" => Key::PageUp,
        "PGDN" | "PAGEDOWN" => Key::PageDown,
        "DEL" | "DELETE" => Key::Delete,
        "BACKSPACE" => Key::Backspace,
        "FORWARDDELETE" | "DELFORWARD" => Key::ForwardDelete,
        "MEDIANEXTTRACK" | "MEDIANEXT" | "NEXTTRACK" => Key::MediaNext,
        "MEDIAPREVTRACK" | "MEDIAPREV" | "MEDIAPREVIOUS" | "PREVTRACK" => Key::MediaPrev,
        "MEDIAPLAYPAUSE" | "PLAYPAUSE" => Key::MediaPlayPause,
        _ => {
            if let Some(number) = name.strip_prefix('F') {
                if let Ok(number) = number.parse::<u8>() {
                    if (1..=24).contains(&number) {
                        return Ok(Key::F(number));
                    }
                }
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => Key::Char(c),
                (Some(c), None) if "=-[]\\;',./`".contains(c) => Key::Char(c),
                _ => return Err(HotkeyError::UnknownKey(raw.into())),
            }
        }
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_shortcuts() {
        let spec = parse("Ctrl+Alt+Right").unwrap();
        assert_eq!(spec.key, Key::Right);
        assert_eq!(spec.modifiers, vec![Modifier::Ctrl, Modifier::Alt]);
        assert_eq!(spec.accelerator(), "Control+Alt+ArrowRight");
        assert_eq!(spec.describe(), "Ctrl+Alt+Right");

        let toggle = parse("Ctrl+Alt+H").unwrap();
        assert_eq!(toggle.key, Key::Char('H'));
        assert_eq!(toggle.accelerator(), "Control+Alt+KeyH");
    }

    #[test]
    fn accepts_aliases_and_case() {
        let spec = parse("command + option + f5").unwrap();
        assert_eq!(spec.modifiers, vec![Modifier::Meta, Modifier::Alt]);
        assert_eq!(spec.key, Key::F(5));
        assert_eq!(spec.accelerator(), "Super+Alt+F5");
    }

    #[test]
    fn requires_a_modifier() {
        assert_eq!(parse("Right"), Err(HotkeyError::MissingModifier));
    }

    #[test]
    fn empty_is_an_error_but_optional_is_off() {
        assert_eq!(parse(""), Err(HotkeyError::Empty));
        assert_eq!(parse_optional(""), None);
        assert_eq!(parse_optional("   "), None);
    }

    #[test]
    fn unknown_key_is_reported() {
        assert_eq!(
            parse("Ctrl+Alt+Banana"),
            Err(HotkeyError::UnknownKey("Banana".into()))
        );
    }

    #[test]
    fn media_keys_parse() {
        let spec = parse("Ctrl+Alt+MediaNextTrack").unwrap();
        assert_eq!(spec.key, Key::MediaNext);
        assert_eq!(spec.accelerator(), "Control+Alt+MediaTrackNext");
    }

    #[test]
    fn digits_and_punctuation() {
        assert_eq!(parse("Ctrl+1").unwrap().accelerator(), "Control+Digit1");
        assert_eq!(parse("Ctrl+-").unwrap().accelerator(), "Control+Minus");
    }
}
