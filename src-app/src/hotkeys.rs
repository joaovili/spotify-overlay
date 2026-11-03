use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use spotify_overlay_core::hotkey::parse_optional;
use spotify_overlay_core::{Config, MediaCommand};
use std::collections::HashMap;

pub struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<HotKey>,
    map: HashMap<u32, MediaCommand>,
    capturing: bool,
}

impl Hotkeys {
    pub fn new(config: &Config) -> Self {
        let mut hotkeys = Self {
            manager: GlobalHotKeyManager::new().ok(),
            registered: Vec::new(),
            map: HashMap::new(),
            capturing: false,
        };
        let _ = hotkeys.register(config);
        hotkeys
    }

    pub fn register(&mut self, config: &Config) -> Result<(), String> {
        if self.capturing {
            return Ok(());
        }
        let Some(manager) = &self.manager else {
            return Err("atalhos globais indisponiveis".into());
        };
        if !self.registered.is_empty() {
            let _ = manager.unregister_all(&self.registered);
            self.registered.clear();
        }
        self.map.clear();

        let mut errors = Vec::new();
        for (spec, command) in bindings(config) {
            let trimmed = spec.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed = match parse_optional(trimmed) {
                Some(Ok(parsed)) => parsed,
                Some(Err(error)) => {
                    errors.push(format!("{trimmed}: {error}"));
                    continue;
                }
                None => continue,
            };
            let accelerator = parsed.accelerator();
            let hotkey: HotKey = match accelerator.parse() {
                Ok(hotkey) => hotkey,
                Err(error) => {
                    errors.push(format!("{accelerator}: {error}"));
                    continue;
                }
            };
            match manager.register(hotkey) {
                Ok(()) => {
                    self.map.insert(hotkey.id(), command);
                    self.registered.push(hotkey);
                }
                Err(error) => errors.push(format!("{accelerator}: {error}")),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    pub fn begin_capture(&mut self) {
        self.capturing = true;
        if let Some(manager) = &self.manager {
            let _ = manager.unregister_all(&self.registered);
        }
        self.registered.clear();
        self.map.clear();
    }

    pub fn end_capture(&mut self, config: &Config) {
        self.capturing = false;
        let _ = self.register(config);
    }

    pub fn poll(&mut self) -> Option<MediaCommand> {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() == HotKeyState::Pressed {
                if let Some(command) = self.map.get(&event.id()) {
                    return Some(*command);
                }
            }
        }
        None
    }
}

fn bindings(config: &Config) -> [(String, MediaCommand); 3] {
    [
        (config.hotkey_skip.clone(), MediaCommand::Next),
        (config.hotkey_prev.clone(), MediaCommand::Previous),
        (config.hotkey_toggle.clone(), MediaCommand::Toggle),
    ]
}
