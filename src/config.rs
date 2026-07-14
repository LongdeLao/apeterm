use std::{
    collections::HashMap,
    env, fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{app::ThemeName, i18n::Locale, preferences::UserPreferences};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_ticker_db_path")]
    pub ticker_db_path: PathBuf,
    #[serde(default)]
    pub locale: Locale,
    #[serde(default)]
    pub preferences: UserPreferences,
    #[serde(default)]
    pub onboarding: OnboardingConfig,
    #[serde(default)]
    pub theme: ThemeName,
    #[serde(default)]
    pub watchlist: WatchlistConfig,
    #[serde(default)]
    pub metadata_provider: MetadataProviderConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub backend: BackendConfig,
    #[serde(default)]
    pub news: NewsConfig,
    #[serde(default)]
    pub sec: SecConfig,
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct OnboardingConfig {
    #[serde(default)]
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WatchlistConfig {
    #[serde(default)]
    pub lists: Vec<NamedWatchlist>,
    #[serde(default)]
    pub active: usize,
    #[serde(default = "default_crypto_symbols", skip_serializing)]
    pub crypto_symbols: Vec<String>,
    #[serde(default = "default_stock_symbols", skip_serializing)]
    pub stock_symbols: Vec<String>,
    #[serde(default, skip_serializing)]
    pub display_names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NamedWatchlist {
    #[serde(default = "default_watchlist_name")]
    pub name: String,
    #[serde(default = "default_crypto_symbols")]
    pub crypto_symbols: Vec<String>,
    #[serde(default = "default_stock_symbols")]
    pub stock_symbols: Vec<String>,
    #[serde(default)]
    pub display_names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetadataProviderConfig {
    #[serde(default)]
    pub provider: MetadataProviderKind,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_metadata_requests_per_minute")]
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MetadataProviderKind {
    #[default]
    None,
    SecEdgar,
    Finnhub,
    FinancialModelingPrep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    #[serde(default = "default_auto_check_on_startup")]
    pub auto_check_on_startup: bool,
    #[serde(default = "default_enrich_max_age_hours")]
    pub enrich_max_age_hours: i64,
    #[serde(default = "default_commit_batch_size")]
    pub commit_batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendConfig {
    #[serde(default = "default_backend_enabled")]
    pub enabled: bool,
    #[serde(default = "default_backend_base_url")]
    pub base_url: String,
    #[serde(default = "default_backend_timeout_seconds")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewsConfig {
    #[serde(default = "default_news_feeds")]
    pub feeds: Vec<String>,
    #[serde(default = "default_news_fetch_on_startup")]
    pub fetch_on_startup: bool,
    #[serde(default = "default_enable_rss")]
    pub enable_rss: bool,
    #[serde(default = "default_enable_financial_juice")]
    pub enable_financial_juice: bool,
    #[serde(default = "default_enable_nasdaq")]
    pub enable_nasdaq: bool,
    #[serde(default = "default_news_refresh_interval_seconds")]
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecConfig {
    #[serde(default = "default_sec_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_sec_refresh_interval_seconds")]
    pub refresh_interval_seconds: u64,
    #[serde(default = "default_sec_requests_per_second")]
    pub requests_per_second: u32,
}

impl AppConfig {
    pub fn load() -> io::Result<Self> {
        let mut config = Self::default()?;
        let config_path = config_path()?;

        let mut should_save = false;
        if config_path.exists() {
            let bytes = fs::read(&config_path)?;
            let missing_preferences = serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .is_none_or(|value| value.get("preferences").is_none());
            if let Ok(file_config) = serde_json::from_slice::<Self>(&bytes) {
                config = file_config;
                if missing_preferences {
                    config.preferences.language =
                        crate::preferences::Language::from_locale(&config.locale);
                    should_save = true;
                }
            }
        } else if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
            let bytes = serde_json::to_vec_pretty(&config).map_err(io::Error::other)?;
            write_atomic(&config_path, &bytes)?;
        }

        if matches!(
            config.metadata_provider.provider,
            MetadataProviderKind::None
                | MetadataProviderKind::Finnhub
                | MetadataProviderKind::FinancialModelingPrep
        ) && config.metadata_provider.api_key.is_none()
            && env::var("APETERM_DISABLE_FREE_METADATA").is_err()
        {
            config.metadata_provider.provider = MetadataProviderKind::SecEdgar;
        }

        if let Ok(api_key) = env::var("APETERM_FINNHUB_API_KEY") {
            config.metadata_provider.provider = MetadataProviderKind::Finnhub;
            config.metadata_provider.api_key = Some(api_key);
        } else if let Ok(api_key) = env::var("APETERM_FMP_API_KEY") {
            config.metadata_provider.provider = MetadataProviderKind::FinancialModelingPrep;
            config.metadata_provider.api_key = Some(api_key);
        }

        if env::var("APETERM_DISABLE_FREE_METADATA").is_ok() {
            config.metadata_provider.provider = MetadataProviderKind::None;
        }

        if let Ok(base_url) = env::var("LLM_BASE_URL") {
            config.llm.base_url = base_url;
        }
        if let Ok(model) = env::var("LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(api_key) = env::var("LLM_API_KEY") {
            config.llm.api_key = Some(api_key);
        } else if let Ok(api_key) = env::var("OPENROUTER_API_KEY") {
            config.llm.api_key = Some(api_key);
        }

        if let Ok(enabled) = env::var("APETERM_BACKEND_ENABLED") {
            config.backend.enabled = enabled != "0" && !enabled.eq_ignore_ascii_case("false");
        }
        if let Ok(base_url) = env::var("APETERM_BACKEND_BASE_URL") {
            config.backend.base_url = base_url;
        }

        if should_migrate_legacy_news_feeds(&config.news.feeds) {
            config.news.feeds = default_news_feeds();
            should_save = true;
        }

        config.locale = config.preferences.language.locale();
        config.watchlist.normalize();
        if should_save {
            let _ = config.save();
        }

        Ok(config)
    }

    pub fn default() -> io::Result<Self> {
        Ok(Self {
            ticker_db_path: data_dir()?.join("instruments.sqlite3"),
            ..Default::default()
        })
    }

    pub fn save(&self) -> io::Result<()> {
        let config_path = config_path()?;
        ensure_parent(&config_path)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        write_atomic(&config_path, &bytes)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let preferences = UserPreferences::default();
        Self {
            ticker_db_path: default_ticker_db_path(),
            locale: preferences.language.locale(),
            preferences,
            onboarding: OnboardingConfig::default(),
            theme: ThemeName::default(),
            watchlist: WatchlistConfig::default(),
            metadata_provider: MetadataProviderConfig::default(),
            llm: LlmConfig::default(),
            backend: BackendConfig::default(),
            news: NewsConfig::default(),
            sec: SecConfig::default(),
            update: UpdateConfig::default(),
        }
    }
}

impl Default for WatchlistConfig {
    fn default() -> Self {
        Self {
            lists: default_named_watchlists(),
            active: 0,
            crypto_symbols: default_crypto_symbols(),
            stock_symbols: default_stock_symbols(),
            display_names: HashMap::new(),
        }
    }
}

impl Default for NamedWatchlist {
    fn default() -> Self {
        Self {
            name: default_watchlist_name(),
            stock_symbols: default_stock_symbols(),
            crypto_symbols: Vec::new(),
            display_names: HashMap::new(),
        }
    }
}

impl Default for MetadataProviderConfig {
    fn default() -> Self {
        Self {
            provider: MetadataProviderKind::SecEdgar,
            api_key: None,
            requests_per_minute: default_metadata_requests_per_minute(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check_on_startup: default_auto_check_on_startup(),
            enrich_max_age_hours: default_enrich_max_age_hours(),
            commit_batch_size: default_commit_batch_size(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: default_llm_base_url(),
            model: default_llm_model(),
            api_key: None,
        }
    }
}

impl WatchlistConfig {
    pub fn normalize(&mut self) {
        if self.lists.is_empty() {
            self.lists.push(NamedWatchlist {
                name: default_watchlist_name(),
                stock_symbols: std::mem::take(&mut self.stock_symbols),
                crypto_symbols: Vec::new(),
                display_names: HashMap::new(),
            });
            self.lists.push(NamedWatchlist {
                name: default_crypto_watchlist_name(),
                stock_symbols: Vec::new(),
                crypto_symbols: std::mem::take(&mut self.crypto_symbols),
                display_names: std::mem::take(&mut self.display_names),
            });
        }

        if self.lists.is_empty() {
            self.lists = default_named_watchlists();
        }

        for list in &mut self.lists {
            if list.name.trim().is_empty() {
                list.name = default_watchlist_name();
            }
            list.stock_symbols.sort();
            list.stock_symbols.dedup();
            list.crypto_symbols.sort();
            list.crypto_symbols.dedup();
            list.display_names.retain(|symbol, _| {
                list.stock_symbols.contains(symbol) || list.crypto_symbols.contains(symbol)
            });
        }

        self.active = self.active.min(self.lists.len().saturating_sub(1));
        self.stock_symbols.clear();
        self.crypto_symbols.clear();
        self.display_names.clear();
    }
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            enabled: default_backend_enabled(),
            base_url: default_backend_base_url(),
            timeout_seconds: default_backend_timeout_seconds(),
        }
    }
}

impl Default for NewsConfig {
    fn default() -> Self {
        Self {
            feeds: default_news_feeds(),
            fetch_on_startup: default_news_fetch_on_startup(),
            enable_rss: default_enable_rss(),
            enable_financial_juice: default_enable_financial_juice(),
            enable_nasdaq: default_enable_nasdaq(),
            refresh_interval_seconds: default_news_refresh_interval_seconds(),
        }
    }
}

impl Default for SecConfig {
    fn default() -> Self {
        Self {
            user_agent: default_sec_user_agent(),
            refresh_interval_seconds: default_sec_refresh_interval_seconds(),
            requests_per_second: default_sec_requests_per_second(),
        }
    }
}

pub fn data_dir() -> io::Result<PathBuf> {
    let dirs = project_dirs()?;
    let dir = dirs.data_local_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

pub fn config_path() -> io::Result<PathBuf> {
    let dirs = project_dirs()?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("config.json"))
}

fn project_dirs() -> io::Result<ProjectDirs> {
    ProjectDirs::from("com", "apeterm", "ApeTerm").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve platform config/data directories",
        )
    })
}

pub fn ensure_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_parent(path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("tmp");
    let temp_path = path.with_file_name(format!(".{file_name}.tmp"));
    fs::write(&temp_path, bytes)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn default_ticker_db_path() -> PathBuf {
    data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("instruments.sqlite3")
}

fn default_crypto_symbols() -> Vec<String> {
    ["BTCUSDT", "ETHUSDT", "SOLUSDT"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn default_watchlist_name() -> String {
    "Main".to_string()
}

fn default_crypto_watchlist_name() -> String {
    "Crypto".to_string()
}

fn default_named_watchlists() -> Vec<NamedWatchlist> {
    vec![
        NamedWatchlist {
            name: default_watchlist_name(),
            stock_symbols: default_stock_symbols(),
            crypto_symbols: Vec::new(),
            display_names: HashMap::new(),
        },
        NamedWatchlist {
            name: default_crypto_watchlist_name(),
            stock_symbols: Vec::new(),
            crypto_symbols: default_crypto_symbols(),
            display_names: HashMap::new(),
        },
    ]
}

fn default_stock_symbols() -> Vec<String> {
    [
        "SPY", "QQQ", "NVDA", "AAPL", "MSFT", "AMZN", "META", "GOOGL", "TSLA", "JPM",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn default_metadata_requests_per_minute() -> u32 {
    600
}

fn default_auto_check_on_startup() -> bool {
    true
}

fn default_enrich_max_age_hours() -> i64 {
    24
}

fn default_commit_batch_size() -> usize {
    500
}

fn default_llm_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_llm_model() -> String {
    "openrouter/free".to_string()
}

fn default_backend_enabled() -> bool {
    true
}

fn default_backend_base_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_backend_timeout_seconds() -> u64 {
    8
}

fn default_news_feeds() -> Vec<String> {
    vec![
        "https://news.google.com/rss/search?q=markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=stocks&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=economy&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=earnings&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=%22Federal+Reserve%22&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=ECB&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Abloomberg.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Awsj.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Aft.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
    ]
}

fn default_news_fetch_on_startup() -> bool {
    true
}

fn default_enable_rss() -> bool {
    true
}

fn default_enable_financial_juice() -> bool {
    true
}

fn default_enable_nasdaq() -> bool {
    true
}

fn default_news_refresh_interval_seconds() -> u64 {
    60
}

fn default_sec_user_agent() -> String {
    "ApeTerm/0.1 contact@example.com".to_string()
}

fn default_sec_refresh_interval_seconds() -> u64 {
    60 * 60
}

fn default_sec_requests_per_second() -> u32 {
    8
}

fn should_migrate_legacy_news_feeds(feeds: &[String]) -> bool {
    !feeds.is_empty() && feeds.iter().all(|feed| is_legacy_news_feed(feed.as_str()))
}

fn is_legacy_news_feed(feed: &str) -> bool {
    feed.contains("feeds.content.dowjones.io/public/rss/mw_")
        || feed.contains("feeds.marketwatch.com/marketwatch/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ThemeName;

    #[test]
    fn deserializes_partial_config_with_defaults() {
        let mut config: AppConfig = serde_json::from_str(
            r#"{
                "locale": "de",
                "theme": "light",
                "watchlist": {
                    "stock_symbols": ["SAP", "DTE.DE"]
                }
            }"#,
        )
        .unwrap();
        config.watchlist.normalize();

        assert_eq!(config.locale, Locale::De);
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.watchlist.lists.len(), 2);
        assert_eq!(
            config.watchlist.lists[0].stock_symbols,
            vec!["DTE.DE", "SAP"]
        );
        assert_eq!(
            config.watchlist.lists[1].crypto_symbols,
            vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]
        );
        assert!(!config.onboarding.completed);
        assert_eq!(
            config.preferences.experience,
            crate::preferences::Experience::Pro
        );
        assert_eq!(config.preferences.tone, crate::i18n::Tone::Normal);
        assert_eq!(
            config.preferences.explanations,
            crate::preferences::ExplanationLevel::Off
        );
        assert_eq!(
            config.preferences.agent_style,
            crate::preferences::AgentStyle::Chat
        );
        assert_eq!(config.metadata_provider.requests_per_minute, 600);
        assert_eq!(config.update.enrich_max_age_hours, 24);
        assert!(config.backend.enabled);
        assert_eq!(config.backend.base_url, "http://localhost:8080");
        assert_eq!(config.backend.timeout_seconds, 8);
        assert!(config.news.enable_rss);
        assert!(config.news.enable_financial_juice);
        assert!(config.news.enable_nasdaq);
        assert_eq!(config.news.refresh_interval_seconds, 60);
    }

    #[test]
    fn preferences_round_trip_and_missing_section_defaults() {
        let config = AppConfig {
            preferences: UserPreferences::ape_preset(crate::preferences::Language::German),
            ..<AppConfig as Default>::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let decoded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.preferences, config.preferences);

        let decoded: AppConfig = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(
            decoded.preferences.experience,
            crate::preferences::Experience::Pro
        );
        assert_eq!(decoded.preferences.tone, crate::i18n::Tone::Normal);
        assert_eq!(
            decoded.preferences.explanations,
            crate::preferences::ExplanationLevel::Off
        );
        assert_eq!(
            decoded.preferences.agent_style,
            crate::preferences::AgentStyle::Chat
        );
    }
}
