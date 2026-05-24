use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SkinTone {
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl SkinTone {
    pub fn all() -> &'static [SkinTone] {
        &[Self::Light, Self::MediumLight, Self::Medium, Self::MediumDark, Self::Dark]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Light       => "Light",
            Self::MediumLight => "Medium Light",
            Self::Medium      => "Medium",
            Self::MediumDark  => "Medium Dark",
            Self::Dark        => "Dark",
        }
    }
    pub fn rgb(&self) -> [f32; 3] {
        match self {
            Self::Light       => [1.00, 0.87, 0.77],
            Self::MediumLight => [0.95, 0.75, 0.60],
            Self::Medium      => [0.85, 0.60, 0.40],
            Self::MediumDark  => [0.60, 0.40, 0.25],
            Self::Dark        => [0.35, 0.22, 0.14],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterConfig {
    pub name:        String,
    pub skin_tone:   SkinTone,
    pub hair_color:  [f32; 3],
    pub shirt_color: [f32; 3],
    pub pants_color: [f32; 3],
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            name:        String::from("Blarg Thompson"),
            skin_tone:   SkinTone::Light,
            hair_color:  [0.25, 0.15, 0.08],
            shirt_color: [0.20, 0.25, 0.35],
            pants_color: [0.15, 0.15, 0.20],
        }
    }
}

impl CharacterConfig {
    const PATH: &'static str = "character.json";

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
