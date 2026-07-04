use crate::{i18n::Key, preferences::Experience};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricId {
    Price,
    ChangePercent,
    Volume,
    AverageVolume,
    RelativeVolume,
    MarketCap,
    PreviousClose,
    Open,
    DayRange,
    Week52High,
    Week52Low,
    PeRatio,
    ForwardPe,
    DividendYield,
    Beta,
}

const SIMPLE_METRICS: &[MetricId] = &[
    MetricId::Price,
    MetricId::ChangePercent,
    MetricId::MarketCap,
    MetricId::PeRatio,
    MetricId::DividendYield,
    MetricId::Beta,
];

const PRO_METRICS: &[MetricId] = &[
    MetricId::Price,
    MetricId::ChangePercent,
    MetricId::Volume,
    MetricId::AverageVolume,
    MetricId::RelativeVolume,
    MetricId::MarketCap,
    MetricId::PreviousClose,
    MetricId::Open,
    MetricId::DayRange,
    MetricId::Week52High,
    MetricId::Week52Low,
    MetricId::PeRatio,
    MetricId::ForwardPe,
    MetricId::DividendYield,
    MetricId::Beta,
];

// Revenue growth, margins, ROE, ROIC, debt/equity, and EV/EBITDA stay out
// until LiveInstrumentDetails has populated fields for them.
const SIMPLE_KEY_STATS: &[MetricId] = &[
    MetricId::MarketCap,
    MetricId::PeRatio,
    MetricId::DividendYield,
    MetricId::Beta,
];

const PRO_KEY_STATS: &[MetricId] = &[
    MetricId::Volume,
    MetricId::AverageVolume,
    MetricId::RelativeVolume,
    MetricId::MarketCap,
    MetricId::PreviousClose,
    MetricId::Open,
    MetricId::DayRange,
    MetricId::Week52High,
    MetricId::Week52Low,
    MetricId::PeRatio,
    MetricId::ForwardPe,
    MetricId::DividendYield,
    MetricId::Beta,
];

pub fn visible_metrics(experience: Experience) -> &'static [MetricId] {
    match experience {
        Experience::Simple => SIMPLE_METRICS,
        Experience::Pro => PRO_METRICS,
    }
}

pub fn visible_key_stats(experience: Experience) -> &'static [MetricId] {
    match experience {
        Experience::Simple => SIMPLE_KEY_STATS,
        Experience::Pro => PRO_KEY_STATS,
    }
}

pub fn metric_explanation_key(metric: MetricId) -> Option<Key> {
    match metric {
        MetricId::Volume => Some(Key::MetricExplanationVolume),
        MetricId::AverageVolume => Some(Key::MetricExplanationAvgVolume),
        MetricId::RelativeVolume => Some(Key::MetricExplanationRelativeVolume),
        MetricId::MarketCap => Some(Key::MetricExplanationMarketCap),
        MetricId::PreviousClose => Some(Key::MetricExplanationPreviousClose),
        MetricId::Open => Some(Key::MetricExplanationOpen),
        MetricId::DayRange => Some(Key::MetricExplanationDayRange),
        MetricId::Week52High => Some(Key::MetricExplanationWeek52High),
        MetricId::Week52Low => Some(Key::MetricExplanationWeek52Low),
        MetricId::PeRatio => Some(Key::MetricExplanationPeRatio),
        MetricId::ForwardPe => Some(Key::MetricExplanationForwardPe),
        MetricId::DividendYield => Some(Key::MetricExplanationDividendYield),
        MetricId::Beta => Some(Key::MetricExplanationBeta),
        MetricId::Price | MetricId::ChangePercent => None,
    }
}

pub fn metric_label_key(metric: MetricId) -> Key {
    match metric {
        MetricId::Price => Key::DetailsLabelCurrentPrice,
        MetricId::ChangePercent => Key::DetailsLabelChange,
        MetricId::Volume => Key::DetailsLabelVolume,
        MetricId::AverageVolume => Key::DetailsLabelAvgVolume,
        MetricId::RelativeVolume => Key::DetailsLabelRvol,
        MetricId::MarketCap => Key::DetailsLabelMarketCap,
        MetricId::PreviousClose => Key::DetailsLabelPreviousClose,
        MetricId::Open => Key::DetailsLabelOpen,
        MetricId::DayRange => Key::DetailsLabelDayRange,
        MetricId::Week52High => Key::DetailsLabelWeekHigh,
        MetricId::Week52Low => Key::DetailsLabelWeekLow,
        MetricId::PeRatio => Key::DetailsLabelPeRatio,
        MetricId::ForwardPe => Key::DetailsLabelForwardPe,
        MetricId::DividendYield => Key::DetailsLabelDividendYield,
        MetricId::Beta => Key::DetailsLabelBeta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_has_strictly_more_metrics_than_simple() {
        assert!(visible_metrics(Experience::Pro).len() > visible_metrics(Experience::Simple).len());
        assert!(visible_metrics(Experience::Pro).contains(&MetricId::Open));
        assert!(visible_metrics(Experience::Pro).contains(&MetricId::DayRange));
        assert!(visible_metrics(Experience::Pro).contains(&MetricId::Week52High));
        assert!(visible_metrics(Experience::Pro).contains(&MetricId::Week52Low));
        assert!(visible_metrics(Experience::Simple).contains(&MetricId::DividendYield));
        assert!(visible_metrics(Experience::Simple).contains(&MetricId::Beta));
    }
}
