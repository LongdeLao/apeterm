use quick_xml::de::from_str;
use serde::Deserialize;

use crate::sec::types::ParsedInsiderTx;

#[derive(Debug, Deserialize)]
struct OwnershipDocument {
    #[serde(rename = "periodOfReport")]
    _period_of_report: String,
    issuer: Issuer,
    #[serde(rename = "nonDerivativeTable")]
    non_derivative_table: Option<NonDerivativeTable>,
}

#[derive(Debug, Deserialize)]
struct Issuer {
    #[serde(rename = "issuerTradingSymbol")]
    ticker: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NonDerivativeTable {
    #[serde(rename = "nonDerivativeTransaction", default)]
    transactions: Vec<NonDerivativeTransaction>,
}

#[derive(Debug, Deserialize)]
struct NonDerivativeTransaction {
    #[serde(rename = "transactionDate")]
    transaction_date: ValueHolder,
    #[serde(rename = "transactionCoding")]
    coding: TransactionCoding,
    #[serde(rename = "transactionAmounts")]
    amounts: TransactionAmounts,
    #[serde(rename = "postTransactionAmounts")]
    post_transaction_amounts: Option<PostTransactionAmounts>,
}

#[derive(Debug, Deserialize)]
struct TransactionCoding {
    #[serde(rename = "transactionCode")]
    code: String,
}

#[derive(Debug, Deserialize)]
struct TransactionAmounts {
    #[serde(rename = "transactionShares")]
    shares: ValueHolder,
    #[serde(rename = "transactionPricePerShare")]
    price_per_share: Option<OptionalValueHolder>,
}

#[derive(Debug, Deserialize)]
struct PostTransactionAmounts {
    #[serde(rename = "sharesOwnedFollowingTransaction")]
    shares_owned_after: Option<ValueHolder>,
}

#[derive(Debug, Deserialize)]
struct ValueHolder {
    value: String,
}

#[derive(Debug, Deserialize)]
struct OptionalValueHolder {
    value: Option<String>,
}

pub fn parse_form4(xml: &str, filed_at: &str) -> Result<Vec<ParsedInsiderTx>, String> {
    let parsed: OwnershipDocument = from_str(xml).map_err(|error| error.to_string())?;
    let ticker = parsed
        .issuer
        .ticker
        .map(|value| value.trim().to_ascii_uppercase())
        .unwrap_or_default();
    let mut transactions = Vec::new();

    for tx in parsed
        .non_derivative_table
        .map(|table| table.transactions)
        .unwrap_or_default()
    {
        let Ok(shares) = parse_decimal(&tx.amounts.shares.value) else {
            continue;
        };
        let price_usd = tx
            .amounts
            .price_per_share
            .as_ref()
            .and_then(|value| value.value.as_deref())
            .map(parse_decimal)
            .transpose()
            .ok()
            .flatten();
        let shares_owned_after = tx
            .post_transaction_amounts
            .as_ref()
            .and_then(|value| value.shares_owned_after.as_ref())
            .map(|value| parse_decimal(&value.value))
            .transpose()
            .ok()
            .flatten();

        transactions.push(ParsedInsiderTx {
            ticker: ticker.clone(),
            filed_at: filed_at.to_string(),
            transaction_date: tx.transaction_date.value,
            code: tx.coding.code.trim().to_string(),
            shares,
            price_usd,
            shares_owned_after,
        });
    }

    Ok(transactions)
}

fn parse_decimal(value: &str) -> Result<f64, String> {
    value
        .replace(',', "")
        .parse::<f64>()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_form4;

    #[test]
    fn parses_real_form4_fixture() {
        let xml = include_str!("../../tests/fixtures/sec/tim_cook_form4.xml");
        let transactions = parse_form4(xml, "2023-10-03").unwrap();
        assert!(!transactions.is_empty());
        assert!(transactions.iter().any(|tx| tx.code == "M"));
    }
}
