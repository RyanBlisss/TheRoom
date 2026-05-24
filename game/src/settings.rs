use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub mouse_sensitivity: f32,
    pub fov:               f32,
    pub master_volume:     f32,
    pub fullscreen:        bool,
    pub show_fps:          bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.0015,
            fov:               70.0,
            master_volume:     0.8,
            fullscreen:        false,
            show_fps:          false,
        }
    }
}

impl Settings {
    const PATH: &'static str = "settings.json";

    pub fn load() -> Self {
        std::fs::read_to_string(Self::PATH)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::PATH, json);
        }
    }
}
