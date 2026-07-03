use std::env;

use serde::{Deserialize, Serialize};

use crate::i18n::Locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UserPreferences {
    #[serde(default)]
    pub experience: Experience,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub explanations: ExplanationLevel,
    #[serde(default)]
    pub agent_style: AgentStyle,
    #[serde(default = "default_language")]
    pub language: Language,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Experience {
    Simple,
    #[default]
    Pro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    #[default]
    Normal,
    Ape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExplanationLevel {
    #[default]
    Off,
    Beginner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentStyle {
    #[default]
    Chat,
    Analyst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    English,
    German,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencePreset {
    Ape,
    Pro,
    Custom,
}

impl UserPreferences {
    pub fn ape_preset(language: Language) -> Self {
        Self {
            experience: Experience::Simple,
            tone: Tone::Ape,
            explanations: ExplanationLevel::Beginner,
            agent_style: AgentStyle::Chat,
            language,
        }
    }

    pub fn pro_preset(language: Language) -> Self {
        Self {
            experience: Experience::Pro,
            tone: Tone::Normal,
            explanations: ExplanationLevel::Off,
            agent_style: AgentStyle::Analyst,
            language,
        }
    }

    pub fn preset(self) -> PreferencePreset {
        let language = self.language;
        if self == Self::ape_preset(language) {
            PreferencePreset::Ape
        } else if self == Self::pro_preset(language) {
            PreferencePreset::Pro
        } else {
            PreferencePreset::Custom
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        default_language()
    }
}

impl Language {
    pub fn from_locale(locale: &Locale) -> Self {
        match locale {
            Locale::De => Self::German,
            Locale::En | Locale::Other(_) => Self::English,
        }
    }

    pub fn locale(self) -> Locale {
        match self {
            Self::English => Locale::En,
            Self::German => Locale::De,
        }
    }

    pub fn move_to(self, direction: crate::app::SelectionDirection) -> Self {
        match direction {
            crate::app::SelectionDirection::Previous | crate::app::SelectionDirection::Next => {
                self.next()
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::English => Self::German,
            Self::German => Self::English,
        }
    }
}

impl Experience {
    pub fn move_to(self, direction: crate::app::SelectionDirection) -> Self {
        match direction {
            crate::app::SelectionDirection::Previous | crate::app::SelectionDirection::Next => {
                self.next()
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Simple => Self::Pro,
            Self::Pro => Self::Simple,
        }
    }
}

impl Tone {
    pub fn move_to(self, direction: crate::app::SelectionDirection) -> Self {
        match direction {
            crate::app::SelectionDirection::Previous | crate::app::SelectionDirection::Next => {
                self.next()
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Ape,
            Self::Ape => Self::Normal,
        }
    }
}

impl ExplanationLevel {
    pub fn move_to(self, direction: crate::app::SelectionDirection) -> Self {
        match direction {
            crate::app::SelectionDirection::Previous | crate::app::SelectionDirection::Next => {
                self.next()
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Beginner,
            Self::Beginner => Self::Off,
        }
    }
}

impl AgentStyle {
    pub fn move_to(self, direction: crate::app::SelectionDirection) -> Self {
        match direction {
            crate::app::SelectionDirection::Previous | crate::app::SelectionDirection::Next => {
                self.next()
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Chat => Self::Analyst,
            Self::Analyst => Self::Chat,
        }
    }
}

fn default_language() -> Language {
    let locale = env::var("LC_ALL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var("LANG").ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if locale.starts_with("de") || locale.contains(".de") || locale.contains("_de") {
        Language::German
    } else {
        Language::English
    }
}
