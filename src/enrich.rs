use std::{collections::HashMap, env, time::Duration};

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{Connection, Result, params};
use serde::Deserialize;
use tokio::time::sleep;

use crate::config::{MetadataProviderConfig, MetadataProviderKind, UpdateConfig};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstrumentMetadata {
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentSummary {
    pub attempted: usize,
    pub updated: usize,
    pub fetched: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[async_trait]
pub trait MetadataProvider {
    fn name(&self) -> &'static str;
    fn supports_symbol(&self, _symbol: &str) -> bool {
        true
    }
    async fn fetch(&self, symbol: &str) -> std::result::Result<InstrumentMetadata, String>;
}

pub async fn enrich_stale_instruments(
    connection: &mut Connection,
    metadata_config: &MetadataProviderConfig,
    update_config: &UpdateConfig,
) -> Result<EnrichmentSummary> {
    enrich_stale_instruments_with_limit(connection, metadata_config, update_config, None).await
}

pub async fn enrich_stale_instruments_with_limit(
    connection: &mut Connection,
    metadata_config: &MetadataProviderConfig,
    update_config: &UpdateConfig,
    limit: Option<usize>,
) -> Result<EnrichmentSummary> {
    let provider: Box<dyn MetadataProvider + Send + Sync> = match metadata_config.provider {
        MetadataProviderKind::None => return Ok(EnrichmentSummary::default()),
        MetadataProviderKind::SecEdgar => match SecEdgarProvider::new().await {
            Ok(provider) => Box::new(provider),
            Err(error) => {
                eprintln!("SEC EDGAR metadata unavailable: {error}");
                return Ok(EnrichmentSummary::default());
            }
        },
        MetadataProviderKind::Finnhub => {
            let Some(api_key) = metadata_config.api_key.clone() else {
                return Ok(EnrichmentSummary::default());
            };
            Box::new(FinnhubProvider::new(api_key))
        }
        MetadataProviderKind::FinancialModelingPrep => {
            let Some(api_key) = metadata_config.api_key.clone() else {
                return Ok(EnrichmentSummary::default());
            };
            Box::new(FmpProvider::new(api_key))
        }
    };

    println!("using {} metadata provider", provider.name());
    let mut symbols = stale_symbols(connection, update_config.enrich_max_age_hours)?;
    let before_filter = symbols.len();
    symbols.retain(|symbol| provider.supports_symbol(symbol));
    let skipped = before_filter.saturating_sub(symbols.len());
    if let Some(limit) = limit {
        symbols.truncate(limit);
    }
    println!(
        "enriching {} SEC-eligible symbols (skipped {} unsupported symbols)",
        symbols.len(),
        skipped
    );
    let mut summary = enrich_symbols(
        connection,
        &*provider,
        &symbols,
        metadata_config.requests_per_minute,
        update_config.commit_batch_size,
    )
    .await?;
    summary.skipped = skipped;
    Ok(summary)
}

pub async fn enrich_symbols(
    connection: &mut Connection,
    provider: &(dyn MetadataProvider + Send + Sync),
    symbols: &[String],
    requests_per_minute: u32,
    commit_batch_size: usize,
) -> Result<EnrichmentSummary> {
    let mut summary = EnrichmentSummary::default();
    let delay = throttle_delay(requests_per_minute);
    let mut pending = Vec::new();
    let total = symbols.len();

    for symbol in symbols {
        summary.attempted += 1;
        match provider.fetch(symbol).await {
            Ok(metadata) => {
                summary.fetched += 1;
                pending.push((symbol.clone(), metadata));
            }
            Err(_) => {
                summary.failed += 1;
            }
        }

        if pending.len() >= commit_batch_size.max(1) {
            summary.updated += commit_metadata_batch(connection, &pending)?;
            pending.clear();
        }

        print_progress(&summary, total);

        if !delay.is_zero() {
            sleep(delay).await;
        }
    }

    if !pending.is_empty() {
        summary.updated += commit_metadata_batch(connection, &pending)?;
    }

    Ok(summary)
}

fn print_progress(summary: &EnrichmentSummary, total: usize) {
    if total == 0 {
        return;
    }

    if summary.attempted == total || summary.attempted % 25 == 0 {
        println!(
            "[{}/{}] fetched {}, committed {}, failed {}",
            summary.attempted, total, summary.fetched, summary.updated, summary.failed
        );
    }
}

pub fn stale_symbols(connection: &Connection, max_age_hours: i64) -> Result<Vec<String>> {
    let cutoff = (Utc::now() - ChronoDuration::hours(max_age_hours)).to_rfc3339();
    let mut statement = connection.prepare(
        "
        SELECT symbol
        FROM instruments
        WHERE active = 1
          AND (
            sector IS NULL
            OR industry IS NULL
            OR last_updated IS NULL
            OR last_updated < ?1
          )
        ORDER BY symbol
        ",
    )?;
    let rows = statement.query_map(params![cutoff], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn commit_metadata_batch(
    connection: &mut Connection,
    rows: &[(String, InstrumentMetadata)],
) -> Result<usize> {
    let tx = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let mut updated = 0;

    for (symbol, metadata) in rows {
        updated += tx.execute(
            "
            UPDATE instruments
            SET sector = COALESCE(?2, sector),
                industry = COALESCE(?3, industry),
                currency = COALESCE(?4, currency),
                last_updated = ?5
            WHERE symbol = ?1
            ",
            params![
                symbol,
                metadata.sector,
                metadata.industry,
                metadata.currency,
                now
            ],
        )?;
    }

    tx.commit()?;
    Ok(updated)
}

fn throttle_delay(requests_per_minute: u32) -> Duration {
    if requests_per_minute == 0 {
        Duration::from_secs(2)
    } else {
        Duration::from_millis((60_000 / requests_per_minute.max(1)) as u64)
    }
}

#[derive(Debug, Clone)]
struct SecEdgarProvider {
    client: reqwest::Client,
    cik_by_symbol: HashMap<String, u64>,
}

impl SecEdgarProvider {
    async fn new() -> std::result::Result<Self, String> {
        let user_agent = env::var("APETERM_SEC_USER_AGENT").unwrap_or_else(|_| {
            "ApeTerm/0.1 local ticker metadata contact@example.com".to_string()
        });
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .map_err(|error| error.to_string())?;
        let exchange: serde_json::Value = client
            .get("https://www.sec.gov/files/company_tickers_exchange.json")
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;
        let cik_by_symbol = parse_sec_ticker_exchange(&exchange);

        Ok(Self {
            client,
            cik_by_symbol,
        })
    }
}

#[async_trait]
impl MetadataProvider for SecEdgarProvider {
    fn name(&self) -> &'static str {
        "sec_edgar"
    }

    fn supports_symbol(&self, symbol: &str) -> bool {
        self.cik_by_symbol.contains_key(&symbol.to_uppercase())
    }

    async fn fetch(&self, symbol: &str) -> std::result::Result<InstrumentMetadata, String> {
        let Some(cik) = self.cik_by_symbol.get(&symbol.to_uppercase()).copied() else {
            return Err("symbol not found in SEC ticker exchange mapping".to_string());
        };
        let url = format!("https://data.sec.gov/submissions/CIK{cik:010}.json");
        let submission: SecSubmission = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        let sector = submission
            .owner_org
            .as_deref()
            .and_then(sector_from_owner_org)
            .or_else(|| submission.sic.as_deref().and_then(sector_from_sic));

        Ok(InstrumentMetadata {
            sector,
            industry: submission.sic_description,
            currency: Some("USD".to_string()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SecSubmission {
    sic: Option<String>,
    #[serde(rename = "sicDescription")]
    sic_description: Option<String>,
    #[serde(rename = "ownerOrg")]
    owner_org: Option<String>,
}

fn parse_sec_ticker_exchange(value: &serde_json::Value) -> HashMap<String, u64> {
    value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let fields = row.as_array()?;
            let cik = fields.first()?.as_u64()?;
            let ticker = fields.get(2)?.as_str()?.to_uppercase();
            Some((ticker, cik))
        })
        .collect()
}

fn sector_from_owner_org(value: &str) -> Option<String> {
    let sector = value
        .trim()
        .trim_start_matches(|character: char| character.is_ascii_digit())
        .trim();
    if sector.is_empty() {
        None
    } else {
        Some(sector.to_string())
    }
}

fn sector_from_sic(sic: &str) -> Option<String> {
    let code = sic.parse::<u16>().ok()?;
    let sector = match code {
        100..=999 => "Agriculture, Forestry and Fishing",
        1000..=1499 => "Mining",
        1500..=1799 => "Construction",
        2000..=3999 => "Manufacturing",
        4000..=4999 => "Transportation, Communications, Electric, Gas and Sanitary Services",
        5000..=5199 => "Wholesale Trade",
        5200..=5999 => "Retail Trade",
        6000..=6799 => "Finance, Insurance and Real Estate",
        7000..=8999 => "Services",
        9100..=9729 => "Public Administration",
        _ => return None,
    };
    Some(sector.to_string())
}

#[derive(Debug, Clone)]
struct FinnhubProvider {
    client: reqwest::Client,
    api_key: String,
}

impl FinnhubProvider {
    fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl MetadataProvider for FinnhubProvider {
    fn name(&self) -> &'static str {
        "finnhub"
    }

    async fn fetch(&self, symbol: &str) -> std::result::Result<InstrumentMetadata, String> {
        let profile_url = format!(
            "https://finnhub.io/api/v1/stock/profile2?symbol={symbol}&token={}",
            self.api_key
        );

        let profile: FinnhubProfile = self
            .client
            .get(profile_url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        Ok(InstrumentMetadata {
            sector: profile.finnhub_industry,
            industry: None,
            currency: profile.currency,
        })
    }
}

#[derive(Debug, Deserialize)]
struct FinnhubProfile {
    #[serde(rename = "finnhubIndustry")]
    finnhub_industry: Option<String>,
    currency: Option<String>,
}

#[derive(Debug, Clone)]
struct FmpProvider {
    client: reqwest::Client,
    api_key: String,
}

impl FmpProvider {
    fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl MetadataProvider for FmpProvider {
    fn name(&self) -> &'static str {
        "financial_modeling_prep"
    }

    async fn fetch(&self, symbol: &str) -> std::result::Result<InstrumentMetadata, String> {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/profile/{symbol}?apikey={}",
            self.api_key
        );
        let rows: Vec<FmpProfile> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;
        let Some(profile) = rows.into_iter().next() else {
            return Err("empty provider response".to_string());
        };

        Ok(InstrumentMetadata {
            sector: profile.sector,
            industry: profile.industry,
            currency: profile.currency,
        })
    }
}

#[derive(Debug, Deserialize)]
struct FmpProfile {
    sector: Option<String>,
    industry: Option<String>,
    currency: Option<String>,
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{Duration as ChronoDuration, Utc};
    use rusqlite::params;

    use super::*;
    use crate::db;

    struct StaticProvider;

    #[async_trait]
    impl MetadataProvider for StaticProvider {
        fn name(&self) -> &'static str {
            "static"
        }

        async fn fetch(&self, _symbol: &str) -> std::result::Result<InstrumentMetadata, String> {
            Ok(InstrumentMetadata {
                sector: Some("Technology".to_string()),
                industry: Some("Consumer Electronics".to_string()),
                currency: Some("USD".to_string()),
            })
        }
    }

    #[tokio::test]
    async fn resume_logic_skips_recent_rows() {
        let mut connection = db::open_memory().unwrap();
        let recent = Utc::now().to_rfc3339();
        let stale = (Utc::now() - ChronoDuration::hours(48)).to_rfc3339();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, sector, industry, active, last_updated) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                params!["AAPL", "Apple Inc.", "Technology", "Electronic Computers", recent],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, active, last_updated) VALUES (?1, ?2, 1, ?3)",
                params!["MSFT", "Microsoft Corp.", stale],
            )
            .unwrap();

        let symbols = stale_symbols(&connection, 24).unwrap();
        assert_eq!(symbols, vec!["MSFT".to_string()]);

        let summary = enrich_symbols(&mut connection, &StaticProvider, &symbols, 10_000, 500)
            .await
            .unwrap();

        assert_eq!(summary.updated, 1);
        let sector: Option<String> = connection
            .query_row(
                "SELECT sector FROM instruments WHERE symbol = 'MSFT'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sector.as_deref(), Some("Technology"));
    }
}
