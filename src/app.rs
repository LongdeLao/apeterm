#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Onboarding,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    Language,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    German,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Dark,
    Light,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDirection {
    Previous,
    Next,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub page: Page,
    pub onboarding_step: OnboardingStep,
    pub onboarding_complete: bool,
    pub logged_in: bool,
    pub language: Language,
    pub theme_name: ThemeName,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            page: Page::Onboarding,
            onboarding_step: OnboardingStep::Welcome,
            onboarding_complete: false,
            logged_in: false,
            language: Language::English,
            theme_name: ThemeName::Dark,
        }
    }

    pub fn advance_onboarding(&mut self) {
        self.onboarding_step = match self.onboarding_step {
            OnboardingStep::Welcome => OnboardingStep::Language,
            OnboardingStep::Language => OnboardingStep::Theme,
            OnboardingStep::Theme => {
                self.page = Page::Dashboard;
                self.onboarding_complete = true;
                OnboardingStep::Theme
            }
        };
    }

    pub fn move_selection(&mut self, direction: SelectionDirection) {
        match self.onboarding_step {
            OnboardingStep::Welcome => {}
            OnboardingStep::Language => self.language = self.language.move_to(direction),
            OnboardingStep::Theme => self.theme_name = self.theme_name.move_to(direction),
        }
    }
}

impl Language {
    fn move_to(self, direction: SelectionDirection) -> Self {
        match self {
            Self::English => match direction {
                SelectionDirection::Previous => Self::German,
                SelectionDirection::Next => Self::German,
            },
            Self::German => match direction {
                SelectionDirection::Previous => Self::English,
                SelectionDirection::Next => Self::English,
            },
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
        }
    }
}

impl ThemeName {
    fn move_to(self, direction: SelectionDirection) -> Self {
        match direction {
            SelectionDirection::Previous => self.previous(),
            SelectionDirection::Next => self.next(),
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Dark => Self::Transparent,
            Self::Light => Self::Dark,
            Self::Transparent => Self::Light,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Transparent,
            Self::Transparent => Self::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Transparent => "Transparent",
        }
    }
}
