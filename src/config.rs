use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::i18n::Locale;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ticker_db_path: PathBuf,
    #[serde(default)]
    pub locale: Locale,
    pub metadata_provider: MetadataProviderConfig,
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataProviderConfig {
    pub provider: MetadataProviderKind,
    pub api_key: Option<String>,
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetadataProviderKind {
    None,
    SecEdgar,
    Finnhub,
    FinancialModelingPrep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub auto_check_on_startup: bool,
    pub enrich_max_age_hours: i64,
    pub commit_batch_size: usize,
}

impl AppConfig {
    pub fn load() -> io::Result<Self> {
        let mut config = Self::default()?;
        let config_path = config_path()?;

        if config_path.exists() {
            let bytes = fs::read(&config_path)?;
            if let Ok(file_config) = serde_json::from_slice::<Self>(&bytes) {
                config = file_config;
            }
        } else if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
            let bytes = serde_json::to_vec_pretty(&config).map_err(io::Error::other)?;
            fs::write(&config_path, bytes)?;
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

        Ok(config)
    }

    pub fn default() -> io::Result<Self> {
        Ok(Self {
            ticker_db_path: data_dir()?.join("instruments.sqlite3"),
            locale: Locale::En,
            metadata_provider: MetadataProviderConfig {
                provider: MetadataProviderKind::SecEdgar,
                api_key: None,
                requests_per_minute: 600,
            },
            update: UpdateConfig {
                auto_check_on_startup: true,
                enrich_max_age_hours: 24,
                commit_batch_size: 500,
            },
        })
    }

    pub fn save(&self) -> io::Result<()> {
        let config_path = config_path()?;
        ensure_parent(&config_path)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        fs::write(config_path, bytes)
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
