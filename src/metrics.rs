use crate::{i18n::Key, preferences::Experience};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricId {
    Price,
    ChangePercent,
    Volume,
    AverageVolume,
    RelativeVolume,
    MarketCap,
    PeRatio,
    ForwardPe,
    DividendYield,
    Beta,
    PreviousClose,
}

const SIMPLE_METRICS: &[MetricId] = &[
    MetricId::Price,
    MetricId::ChangePercent,
    MetricId::MarketCap,
    MetricId::PeRatio,
];

const PRO_METRICS: &[MetricId] = &[
    MetricId::Price,
    MetricId::ChangePercent,
    MetricId::Volume,
    MetricId::AverageVolume,
    MetricId::RelativeVolume,
    MetricId::MarketCap,
    MetricId::PeRatio,
    MetricId::ForwardPe,
    MetricId::DividendYield,
    MetricId::Beta,
    MetricId::PreviousClose,
];

const SIMPLE_KEY_STATS: &[MetricId] = &[MetricId::MarketCap, MetricId::PeRatio];

const PRO_KEY_STATS: &[MetricId] = &[
    MetricId::Volume,
    MetricId::AverageVolume,
    MetricId::RelativeVolume,
    MetricId::MarketCap,
    MetricId::PeRatio,
    MetricId::ForwardPe,
    MetricId::DividendYield,
    MetricId::Beta,
    MetricId::PreviousClose,
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
        MetricId::PeRatio => Some(Key::MetricExplanationPeRatio),
        MetricId::ForwardPe => Some(Key::MetricExplanationForwardPe),
        MetricId::DividendYield => Some(Key::MetricExplanationDividendYield),
        MetricId::Beta => Some(Key::MetricExplanationBeta),
        MetricId::PreviousClose => Some(Key::MetricExplanationPreviousClose),
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
        MetricId::PeRatio => Key::DetailsLabelPeRatio,
        MetricId::ForwardPe => Key::DetailsLabelForwardPe,
        MetricId::DividendYield => Key::DetailsLabelDividendYield,
        MetricId::Beta => Key::DetailsLabelBeta,
        MetricId::PreviousClose => Key::DetailsLabelPreviousClose,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_has_strictly_more_metrics_than_simple() {
        assert!(visible_metrics(Experience::Pro).len() > visible_metrics(Experience::Simple).len());
    }
}
