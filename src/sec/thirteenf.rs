use regex::Regex;

use crate::sec::types::ParsedHolding;

pub fn parse_information_table(xml: &str) -> Result<Vec<ParsedHolding>, String> {
    let row_re = Regex::new(r"(?s)<infoTable\b[^>]*>(.*?)</infoTable>")
        .map_err(|error| error.to_string())?;
    let cusip_re =
        Regex::new(r"(?s)<cusip\b[^>]*>\s*(.*?)\s*</cusip>").map_err(|error| error.to_string())?;
    let value_re =
        Regex::new(r"(?s)<value\b[^>]*>\s*(.*?)\s*</value>").map_err(|error| error.to_string())?;
    let shares_re = Regex::new(r"(?s)<sshPrnamt\b[^>]*>\s*(.*?)\s*</sshPrnamt>")
        .map_err(|error| error.to_string())?;
    let symbol_re =
        Regex::new(r"(?s)<issuerTradingSymbol\b[^>]*>\s*(.*?)\s*</issuerTradingSymbol>")
            .map_err(|error| error.to_string())?;
    let issuer_re = Regex::new(r"(?s)<nameOfIssuer\b[^>]*>\s*(.*?)\s*</nameOfIssuer>")
        .map_err(|error| error.to_string())?;
    let mut holdings = Vec::new();

    for captures in row_re.captures_iter(xml) {
        let Some(row) = captures.get(1).map(|value| value.as_str()) else {
            continue;
        };

        let Some(cusip) = extract_with_regex(row, &cusip_re) else {
            continue;
        };
        let Some(value_usd) =
            extract_with_regex(row, &value_re).and_then(|value| parse_i64(&value))
        else {
            continue;
        };
        let Some(shares) = extract_with_regex(row, &shares_re).and_then(|value| parse_i64(&value))
        else {
            continue;
        };

        let ticker = extract_with_regex(row, &symbol_re)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_uppercase())
            .or_else(|| {
                extract_with_regex(row, &issuer_re).and_then(|value| readable_issuer_label(&value))
            });

        holdings.push(ParsedHolding {
            cusip,
            ticker,
            shares,
            value_usd,
        });
    }

    Ok(holdings)
}

fn extract_with_regex(xml: &str, regex: &Regex) -> Option<String> {
    let value = regex.captures(xml)?.get(1)?.as_str().trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn parse_i64(value: &str) -> Option<i64> {
    value.replace(',', "").parse::<i64>().ok()
}

fn readable_issuer_label(value: &str) -> Option<String> {
    let label = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty() {
        None
    } else {
        Some(label.chars().take(12).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_information_table;

    #[test]
    fn parses_real_13f_fixture() {
        let xml = include_str!("../../tests/fixtures/sec/berkshire_infotable.xml");
        let holdings = parse_information_table(xml).unwrap();
        assert!(!holdings.is_empty());
        assert!(holdings.iter().any(|row| row.cusip == "023135106"));
    }
}
