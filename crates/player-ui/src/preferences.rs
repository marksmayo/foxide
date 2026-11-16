//! Validated user preferences. No user-supplied text becomes an engine command.
use std::collections::BTreeMap;

pub const SHORTCUT_ACTIONS: &[(&str, &str, f64)] = &[
    ("pause", "pause", 0.0),
    ("seek-backward", "seek-relative", -5.0),
    ("seek-forward", "seek-relative", 5.0),
    ("seek-minute-backward", "seek-relative", -60.0),
    ("seek-minute-forward", "seek-relative", 60.0),
    ("fullscreen", "fullscreen", 0.0),
    ("playlist", "playlist", 0.0),
    ("mute", "mute", 0.0),
    ("quit", "quit", 0.0),
    ("next", "next", 0.0),
    ("previous", "previous", 0.0),
    ("frame-step", "frame-step", 0.0),
    ("frame-back-step", "frame-back-step", 0.0),
    ("open", "open", 0.0),
    ("open-url", "url", 0.0),
    ("preferences", "preferences", 0.0),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Preferences {
    bindings: BTreeMap<String, String>,
    light: bool,
    accent: u32,
    compact: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        let bindings = [
            ("Space", "pause"),
            ("p", "pause"),
            ("ArrowLeft", "seek-backward"),
            ("ArrowRight", "seek-forward"),
            ("ArrowUp", "seek-minute-forward"),
            ("ArrowDown", "seek-minute-backward"),
            ("f", "fullscreen"),
            ("F11", "fullscreen"),
            ("q", "quit"),
            (">", "next"),
            ("<", "previous"),
            (".", "frame-step"),
            (",", "frame-back-step"),
            ("m", "mute"),
            ("Ctrl+O", "open"),
            ("Meta+O", "open"),
            ("Ctrl+L", "open-url"),
            ("Meta+L", "open-url"),
            ("Ctrl+P", "playlist"),
            ("Meta+P", "playlist"),
            ("Ctrl+Q", "quit"),
            ("Meta+Q", "quit"),
        ]
        .into_iter()
        .map(|(key, action)| (normalize_chord(key).unwrap(), action.into()))
        .collect();
        Self {
            bindings,
            light: false,
            accent: 0xef955f,
            compact: false,
        }
    }
}

fn normalize_chord(chord: &str) -> Result<String, String> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let parts: Vec<_> = chord.trim().split('+').collect();
    let (key, modifiers) = parts.split_last().ok_or("Enter a key")?;
    for modifier in modifiers {
        let flag = match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => &mut ctrl,
            "alt" => &mut alt,
            "shift" => &mut shift,
            "meta" | "cmd" | "super" => &mut meta,
            _ => return Err("Unknown key modifier".into()),
        };
        if *flag {
            return Err("Duplicate key modifier".into());
        }
        *flag = true;
    }
    let key = match *key {
        "Space" => "Space".to_owned(),
        "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Enter" | "Tab" => (*key).to_owned(),
        "Escape" => return Err("Escape is reserved for closing dialogs and fullscreen".into()),
        value
            if value.starts_with('F')
                && value[1..]
                    .parse::<u8>()
                    .is_ok_and(|n| (1..=24).contains(&n)) =>
        {
            value.to_owned()
        }
        value
            if value.chars().count() == 1
                && !value.chars().any(char::is_control)
                && !value.trim().is_empty() =>
        {
            value.to_lowercase()
        }
        _ => return Err("Use a character, Space, an arrow, Enter, Tab or F1–F24".into()),
    };
    let mut normalized = String::new();
    for (enabled, prefix) in [
        (ctrl, "Ctrl+"),
        (alt, "Alt+"),
        (shift, "Shift+"),
        (meta, "Meta+"),
    ] {
        if enabled {
            normalized.push_str(prefix);
        }
    }
    normalized.push_str(&key);
    Ok(normalized)
}

impl Preferences {
    pub fn compact(&self) -> bool {
        self.compact
    }
    pub fn set_compact(&mut self, compact: bool) {
        self.compact = compact;
    }

    pub fn light_theme(&self) -> bool {
        self.light
    }
    pub fn accent_rgb(&self) -> u32 {
        self.accent
    }
    pub fn set_appearance(&mut self, light: bool, color: &str) -> Result<(), String> {
        let hex = color
            .strip_prefix('#')
            .ok_or("Use a six-digit hex color, such as #ef955f")?;
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Use a six-digit hex color, such as #ef955f".into());
        }
        let accent = u32::from_str_radix(hex, 16).map_err(|error| error.to_string())?;
        self.light = light;
        self.accent = accent;
        Ok(())
    }

    /// Missing files use defaults; malformed files remain untouched and return an error.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.to_string()),
        };
        let mut text = String::new();
        file.take(64 * 1024 + 1)
            .read_to_string(&mut text)
            .map_err(|error| error.to_string())?;
        Self::from_json(&text)
    }

    /// Write a sibling file and rename it only after the complete JSON is synced.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut name = path.as_os_str().to_owned();
        name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let temporary = std::path::PathBuf::from(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let written = file
            .write_all(self.to_json().as_bytes())
            .and_then(|_| file.sync_all());
        drop(file);
        let result = written.and_then(|_| std::fs::rename(&temporary, path));
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(|error| error.to_string())
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&str, &str)> {
        self.bindings
            .iter()
            .map(|(key, action)| (key.as_str(), action.as_str()))
    }

    pub fn bind(&mut self, chord: &str, action: &str) -> Result<(), String> {
        let key = normalize_chord(chord)?;
        if !SHORTCUT_ACTIONS.iter().any(|entry| entry.0 == action) {
            return Err("Unknown shortcut action".into());
        }
        if self
            .bindings
            .get(&key)
            .is_some_and(|existing| existing != action)
        {
            return Err("This key is already assigned. Remove that binding first.".into());
        }
        if self.bindings.len() >= 128 && !self.bindings.contains_key(&key) {
            return Err("Too many bindings".into());
        }
        self.bindings.insert(key, action.into());
        Ok(())
    }

    pub fn unbind(&mut self, chord: &str) -> Result<(), String> {
        self.bindings.remove(&normalize_chord(chord)?);
        Ok(())
    }

    pub fn resolve_key(
        &self,
        key: &str,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    ) -> Option<(&'static str, f64)> {
        let mut chord = String::new();
        for (enabled, prefix) in [
            (ctrl, "Ctrl+"),
            (alt, "Alt+"),
            (shift, "Shift+"),
            (meta, "Meta+"),
        ] {
            if enabled {
                chord.push_str(prefix);
            }
        }
        chord.push_str(if key == " " { "Space" } else { key });
        let action = self.bindings.get(&normalize_chord(&chord).ok()?)?;
        SHORTCUT_ACTIONS
            .iter()
            .find(|entry| entry.0 == action)
            .map(|entry| (entry.1, entry.2))
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({"version": 1, "bindings": self.bindings, "compact": self.compact, "appearance": {"light": self.light, "accent": format!("#{:06x}", self.accent)}}).to_string()
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > 64 * 1024 {
            return Err("Preferences exceed 64 KiB".into());
        }
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| error.to_string())?;
        if value["version"].as_u64() != Some(1) {
            return Err("Unsupported preferences version".into());
        }
        let bindings = value["bindings"]
            .as_object()
            .ok_or("Bindings must be an object")?;
        let mut preferences = Self {
            bindings: BTreeMap::new(),
            ..Self::default()
        };
        for (key, action) in bindings {
            preferences.bind(
                key,
                action.as_str().ok_or("Shortcut actions must be strings")?,
            )?;
        }
        if let Some(appearance) = value.get("appearance") {
            let light = appearance["light"]
                .as_bool()
                .ok_or("Theme must be a boolean")?;
            let accent = appearance["accent"]
                .as_str()
                .ok_or("Accent must be a hex color")?;
            preferences.set_appearance(light, accent)?;
        }
        if let Some(compact) = value.get("compact") {
            preferences.compact = compact.as_bool().ok_or("Compact view must be a boolean")?;
        }
        Ok(preferences)
    }
}
