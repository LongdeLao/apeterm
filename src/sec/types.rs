use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Institution,
    Ceo,
}

impl EntityKind {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Institution => "institution",
            Self::Ceo => "ceo",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecEntity {
    pub id: i64,
    pub kind: EntityKind,
    pub name: String,
    pub filer_cik: String,
    pub issuer_ticker: Option<String>,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HoldingRow {
    pub cusip: String,
    pub ticker: Option<String>,
    pub shares: i64,
    pub value_usd: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldingDeltaKind {
    New,
    Increased,
    Decreased,
    Exited,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct HoldingDelta {
    pub cusip: String,
    pub ticker: Option<String>,
    pub current_shares: i64,
    pub previous_shares: i64,
    pub kind: HoldingDeltaKind,
}

#[derive(Debug, Clone)]
pub struct InsiderTx {
    pub ticker: String,
    pub filed_at: String,
    pub transaction_date: String,
    pub code: String,
    pub shares: f64,
    pub price_usd: Option<f64>,
    pub shares_owned_after: Option<f64>,
    pub accession_no: String,
}

#[derive(Debug, Clone)]
pub struct ParsedHolding {
    pub cusip: String,
    pub ticker: Option<String>,
    pub shares: i64,
    pub value_usd: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedInsiderTx {
    pub ticker: String,
    pub filed_at: String,
    pub transaction_date: String,
    pub code: String,
    pub shares: f64,
    pub price_usd: Option<f64>,
    pub shares_owned_after: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SecFiling {
    pub accession_no: String,
    pub filed_at: String,
    pub primary_document: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedEntity {
    pub kind: EntityKind,
    pub name: String,
    pub filer_cik: String,
    pub issuer_ticker: Option<String>,
    pub subtitle: Option<String>,
}
