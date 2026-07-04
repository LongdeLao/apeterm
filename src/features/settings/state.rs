use crate::app::*;
use crate::preferences::{PreferencePreset, UserPreferences};

/// UI state owned by the settings page.
#[derive(Debug, Default)]
pub struct SettingsFeature {
    pub selection: usize,
    pub reset_confirmation: Option<TextInput>,
}

impl App {
    pub fn open_settings(&mut self) {
        if self.page != Page::Settings {
            self.return_page = Some(self.page);
        }
        self.mode = AppMode::Normal;
        self.page = Page::Settings;
        self.show_help = false;
        self.pending_split = false;
        self.settings.reset_confirmation = None;
        self.selected_news = None;
    }
    pub fn selected_settings_item(&self) -> SettingsItem {
        SettingsItem::ALL[self.settings.selection.min(SettingsItem::ALL.len() - 1)]
    }
    pub fn active_preference_preset(&self) -> PreferencePreset {
        self.preferences.preset()
    }
    pub fn move_settings_selection(&mut self, direction: SelectionDirection) {
        if self.settings.reset_confirmation.is_some() {
            return;
        }

        self.settings.selection = match direction {
            SelectionDirection::Previous => {
                if self.settings.selection == 0 {
                    SettingsItem::ALL.len() - 1
                } else {
                    self.settings.selection - 1
                }
            }
            SelectionDirection::Next => (self.settings.selection + 1) % SettingsItem::ALL.len(),
        };
    }
    pub fn activate_settings_item(&mut self) {
        if let Some(input) = &self.settings.reset_confirmation {
            if input.input == "reset" {
                self.reset_settings_to_defaults();
            }
            return;
        }

        match self.selected_settings_item() {
            SettingsItem::ApePreset => {
                self.set_preferences(UserPreferences::ape_preset(self.preferences.language))
            }
            SettingsItem::ProPreset => {
                self.set_preferences(UserPreferences::pro_preset(self.preferences.language))
            }
            SettingsItem::CustomPreset => {}
            SettingsItem::Experience => self.set_experience(self.preferences.experience.next()),
            SettingsItem::Tone => self.set_tone(self.preferences.tone.next()),
            SettingsItem::Explanations => {
                self.set_explanations(self.preferences.explanations.next())
            }
            SettingsItem::AgentStyle => self.set_agent_style(self.preferences.agent_style.next()),
            SettingsItem::Language => self.toggle_locale(),
            SettingsItem::Theme => self.set_theme(self.theme_name.next()),
            SettingsItem::Onboarding => self.toggle_onboarding_preference(),
            SettingsItem::Reset => {
                self.settings.reset_confirmation = Some(TextInput {
                    input: String::new(),
                });
                self.begin_text_input(InputTarget::ResetConfirmation);
            }
        }
    }
    pub fn adjust_settings_item(&mut self, direction: SelectionDirection) {
        if self.settings.reset_confirmation.is_some() {
            return;
        }

        match self.selected_settings_item() {
            SettingsItem::ApePreset => {
                self.set_preferences(UserPreferences::ape_preset(self.preferences.language))
            }
            SettingsItem::ProPreset => {
                self.set_preferences(UserPreferences::pro_preset(self.preferences.language))
            }
            SettingsItem::CustomPreset => {}
            SettingsItem::Experience => {
                self.set_experience(self.preferences.experience.move_to(direction))
            }
            SettingsItem::Tone => self.set_tone(self.preferences.tone.move_to(direction)),
            SettingsItem::Explanations => {
                self.set_explanations(self.preferences.explanations.move_to(direction))
            }
            SettingsItem::AgentStyle => {
                self.set_agent_style(self.preferences.agent_style.move_to(direction))
            }
            SettingsItem::Language => {
                self.set_language(self.preferences.language.move_to(direction));
            }
            SettingsItem::Theme => self.set_theme(self.theme_name.move_to(direction)),
            SettingsItem::Onboarding => self.toggle_onboarding_preference(),
            SettingsItem::Reset => {}
        }
    }
    pub(crate) fn toggle_onboarding_preference(&mut self) {
        self.onboarding_complete = !self.onboarding_complete;
        self.config.onboarding.completed = self.onboarding_complete;
        let _ = self.config.save();
    }
    pub(crate) fn reset_settings_to_defaults(&mut self) {
        self.preferences = UserPreferences::default();
        self.theme_name = ThemeName::default();
        self.onboarding_complete = false;
        self.onboarding_step = OnboardingStep::Welcome;
        self.config.preferences = self.preferences;
        self.config.locale = self.preferences.language.locale();
        self.config.theme = self.theme_name;
        self.config.onboarding.completed = false;
        self.settings.reset_confirmation = None;
        self.clear_text_input_mode();
        self.apply_preferences();
        let _ = self.config.save();
    }
}

impl SettingsItem {
    pub const ALL: &'static [SettingsItem] = &[
        Self::ApePreset,
        Self::ProPreset,
        Self::CustomPreset,
        Self::Experience,
        Self::Tone,
        Self::Explanations,
        Self::AgentStyle,
        Self::Language,
        Self::Theme,
        Self::Onboarding,
        Self::Reset,
    ];
}
