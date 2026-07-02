use serde::Deserialize;

use crate::sec::{
    client::SecClient,
    types::{SecEntity, SecFiling},
};

#[derive(Debug, Deserialize)]
struct Submissions {
    filings: FilingWrapper,
}

#[derive(Debug, Deserialize)]
struct FilingWrapper {
    recent: RecentFilings,
}

#[derive(Debug, Deserialize)]
struct RecentFilings {
    #[serde(rename = "accessionNumber")]
    accession_numbers: Vec<String>,
    #[serde(rename = "filingDate")]
    filing_dates: Vec<String>,
    form: Vec<String>,
    #[serde(rename = "primaryDocument")]
    primary_documents: Vec<String>,
}

pub fn new_accessions(
    client: &SecClient,
    entity: &SecEntity,
    last_seen: Option<&str>,
) -> Result<Vec<SecFiling>, String> {
    let url = format!(
        "https://data.sec.gov/submissions/CIK{}.json",
        entity.filer_cik
    );
    let submissions: Submissions = client.get_json(&url)?;
    let recent = submissions.filings.recent;
    let mut filings = Vec::new();
    let bootstrap_limit = match entity.kind.as_db_str() {
        "institution" => 8,
        "ceo" => 20,
        _ => 10,
    };

    for index in 0..recent.accession_numbers.len() {
        let accession_no = recent.accession_numbers[index].clone();
        if last_seen.is_some_and(|value| value == accession_no) {
            break;
        }

        let form = recent.form.get(index).cloned().unwrap_or_default();
        let relevant = match entity.kind.as_db_str() {
            "institution" => form.starts_with("13F-HR"),
            "ceo" => form == "4",
            _ => false,
        };
        if !relevant {
            continue;
        }

        filings.push(SecFiling {
            accession_no,
            filed_at: recent.filing_dates.get(index).cloned().unwrap_or_default(),
            primary_document: recent
                .primary_documents
                .get(index)
                .cloned()
                .unwrap_or_default(),
        });

        if last_seen.is_none() && filings.len() >= bootstrap_limit {
            break;
        }
    }

    filings.reverse();
    Ok(filings)
}
