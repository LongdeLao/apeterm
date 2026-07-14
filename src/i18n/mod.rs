use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::IntoEnumIterator;
use unicode_width::UnicodeWidthStr;

pub mod keys;

/// Copy voice for localized strings; `Ape` selects `@ape` key variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    #[default]
    Normal,
    Ape,
}

pub use keys::Key;

include!(concat!(env!("OUT_DIR"), "/embedded_locales.rs"));

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    #[default]
    En,
    De,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct I18n {
    active: Locale,
    locales: HashMap<Locale, HashMap<String, String>>,
}

impl I18n {
    pub fn new(active: Locale) -> Self {
        let locales = load_locales().expect("embedded locale JSON must parse");
        let i18n = Self { active, locales };

        #[cfg(debug_assertions)]
        i18n.assert_complete();

        i18n
    }

    pub fn set_active(&mut self, active: Locale) {
        self.active = active;
    }

    pub fn available_locales(&self) -> Vec<Locale> {
        let mut locales = self.locales.keys().cloned().collect::<Vec<_>>();
        locales.sort_by(|left, right| left.code().cmp(right.code()));
        locales
    }

    pub fn t(&self, key: Key) -> &str {
        self.t_with_tone(key, Tone::Normal)
    }

    pub fn t_with_tone(&self, key: Key, tone: Tone) -> &str {
        let raw_key: &'static str = key.into();
        let tone_key = match tone {
            Tone::Normal => None,
            Tone::Ape => Some(format!("{raw_key}@ape")),
        };
        if let Some(tone_key) = tone_key.as_deref()
            && let Some(value) = self
                .locales
                .get(&self.active)
                .and_then(|locale| locale.get(tone_key))
        {
            return value.as_str();
        }

        self.locales
            .get(&self.active)
            .and_then(|locale| locale.get(raw_key))
            .or_else(|| {
                self.locales
                    .get(&Locale::En)
                    .and_then(|locale| locale.get(raw_key))
            })
            .map(String::as_str)
            .unwrap_or(raw_key)
    }

    pub fn width(&self, key: Key) -> usize {
        UnicodeWidthStr::width(self.t(key))
    }

    pub fn metric_explanation(&self, key: Key) -> Option<&str> {
        let raw_key: &'static str = key.into();
        self.locales
            .get(&self.active)
            .and_then(|locale| locale.get(raw_key))
            .or_else(|| {
                self.locales
                    .get(&Locale::En)
                    .and_then(|locale| locale.get(raw_key))
            })
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
    }

    #[cfg(debug_assertions)]
    pub fn assert_complete(&self) {
        let missing = missing_locale_keys(&self.locales);
        debug_assert!(
            missing.is_empty(),
            "missing locale keys:\n{}",
            missing.join("\n")
        );
    }
}

impl Locale {
    pub fn code(&self) -> &str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Other(code) => code.as_str(),
        }
    }

    pub fn language_key(&self) -> Option<Key> {
        match self {
            Self::En => Some(Key::AppLanguageEnglish),
            Self::De => Some(Key::AppLanguageGerman),
            Self::Other(_) => None,
        }
    }

    fn from_code(code: &str) -> Self {
        match code {
            "en" => Self::En,
            "de" => Self::De,
            other => Self::Other(other.to_string()),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl FromStr for Locale {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_code(value))
    }
}

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_code(&value))
    }
}

pub fn validate_embedded_locales() -> Result<(), String> {
    let locales = load_locales().map_err(|error| error.to_string())?;
    let missing = missing_locale_keys(&locales);
    let empty = empty_locale_values(&locales);
    if missing.is_empty() && empty.is_empty() {
        Ok(())
    } else {
        let mut lines = Vec::new();
        if !missing.is_empty() {
            lines.push(format!("missing locale keys:\n{}", missing.join("\n")));
        }
        if !empty.is_empty() {
            lines.push(format!("empty locale values:\n{}", empty.join("\n")));
        }
        Err(lines.join("\n"))
    }
}

fn load_locales() -> serde_json::Result<HashMap<Locale, HashMap<String, String>>> {
    let mut locales = HashMap::new();
    for (code, json) in EMBEDDED_LOCALES {
        locales.insert(Locale::from_code(code), serde_json::from_str(json)?);
    }
    Ok(locales)
}

fn missing_locale_keys(locales: &HashMap<Locale, HashMap<String, String>>) -> Vec<String> {
    let mut missing = Vec::new();

    for locale in locales.keys() {
        let Some(entries) = locales.get(locale) else {
            missing.push(format!("{locale:?}: <locale file missing>"));
            continue;
        };

        for key in Key::iter() {
            let raw_key: &'static str = key.into();
            if !entries.contains_key(raw_key) {
                missing.push(format!("{locale:?}: {raw_key}"));
            }
        }
    }

    missing
}

fn empty_locale_values(locales: &HashMap<Locale, HashMap<String, String>>) -> Vec<String> {
    let mut empty = Vec::new();

    for (locale, entries) in locales {
        for key in Key::iter() {
            let raw_key: &'static str = key.into();
            if entries
                .get(raw_key)
                .is_some_and(|value| value.trim().is_empty())
            {
                empty.push(format!("{locale:?}: {raw_key}"));
            }
        }
    }

    empty
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn every_key_resolves_for_every_locale_and_tone() {
        for locale in [Locale::En, Locale::De] {
            let i18n = I18n::new(locale.clone());
            for tone in [Tone::Normal, Tone::Ape] {
                for key in Key::iter() {
                    let resolved = i18n.t_with_tone(key, tone);
                    assert!(
                        !resolved.trim().is_empty(),
                        "empty copy for {locale:?} {tone:?} {key:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn german_ape_empty_state_uses_override_only_for_ape() {
        let i18n = I18n::new(Locale::De);

        assert_eq!(
            i18n.t_with_tone(Key::SearchEmpty, Tone::Normal),
            "Keine Daten gefunden."
        );
        assert_eq!(
            i18n.t_with_tone(Key::SearchEmpty, Tone::Ape),
            "Nix gefunden."
        );
    }
}
