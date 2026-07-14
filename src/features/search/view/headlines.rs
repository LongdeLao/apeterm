//! Detail headlines: merge and dedupe backend + local news.

use super::*;

pub(super) fn push_headlines(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    symbol: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    let rows = detail_headlines(app, symbol);
    if rows.is_empty() {
        let empty = backend_headlines_empty_message(app, symbol)
            .unwrap_or_else(|| app.t(Key::DetailsHeadlinesEmpty).to_string());
        lines.push(Line::from(Span::styled(
            empty,
            Style::default().fg(theme.muted),
        )));
        return;
    }
    for (index, row) in rows.iter().take(3).enumerate() {
        if index > 0 {
            lines.push(Line::from(""));
        }
        let prefix = format!("{}. ", index + 1);
        let title_width = width.saturating_sub(text_width(&prefix));
        let mut title_lines = wrap_words(&row.title, title_width);
        if title_lines.len() > 2 {
            title_lines.truncate(2);
            append_suffix_to_last_line(&mut title_lines, "...", title_width);
        }
        if title_lines.is_empty() {
            title_lines.push(row.title.clone());
        }
        for (line_index, title) in title_lines.into_iter().enumerate() {
            let line_prefix = if line_index == 0 {
                prefix.clone()
            } else {
                " ".repeat(text_width(&prefix))
            };
            lines.push(Line::from(vec![
                Span::styled(line_prefix, Style::default().fg(theme.accent)),
                Span::styled(title, Style::default().fg(theme.foreground)),
            ]));
        }
        let metadata = format!("{} | {}", row.sources.join(", "), row.age);
        lines.push(Line::from(Span::styled(
            truncate_to_width(&metadata, width),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
    }
}

#[derive(Debug, Clone)]
pub(super) struct DetailHeadline {
    title: String,
    sources: Vec<String>,
    age: String,
}

pub(super) fn detail_headlines(app: &App, symbol: &str) -> Vec<DetailHeadline> {
    if let Some(insight) = app.search.backend_insight.as_ref()
        && insight.ticker == symbol
        && let Some(context) = &insight.context
    {
        return dedupe_backend_headlines(app, &context.articles);
    }
    dedupe_local_headlines(app, symbol)
}

pub(super) fn dedupe_backend_headlines(
    app: &App,
    articles: &[InsightArticle],
) -> Vec<DetailHeadline> {
    let mut rows = Vec::new();
    for article in articles {
        merge_headline(
            &mut rows,
            article.title.clone(),
            backend_source_label(app, article),
            backend_age_label(app, article),
        );
    }
    rows
}

pub(super) fn dedupe_local_headlines(app: &App, symbol: &str) -> Vec<DetailHeadline> {
    let mut rows = Vec::new();
    for item in app
        .news
        .items
        .iter()
        .filter(|item| item.symbols.iter().any(|candidate| candidate == symbol))
    {
        let age = app.news_timestamp(item.published_at);
        merge_headline(
            &mut rows,
            item.title.clone(),
            if item.source.trim().is_empty() {
                app.t(Key::DetailsHeadlinesLocalFeed).to_string()
            } else {
                item.source.clone()
            },
            if age.is_empty() {
                app.t(Key::DetailsHeadlinesFresh).to_string()
            } else {
                age
            },
        );
    }
    rows
}

pub(super) fn merge_headline(
    rows: &mut Vec<DetailHeadline>,
    title: String,
    source: String,
    age: String,
) {
    let key = normalize_headline_key(&title);
    if let Some(existing) = rows
        .iter_mut()
        .find(|row| normalize_headline_key(&row.title) == key)
    {
        if !existing
            .sources
            .iter()
            .any(|candidate| candidate == &source)
        {
            existing.sources.push(source);
        }
        return;
    }
    rows.push(DetailHeadline {
        title,
        sources: vec![source],
        age,
    });
}

pub(super) fn backend_headlines_empty_message(app: &App, symbol: &str) -> Option<String> {
    let insight = app.search.backend_insight.as_ref()?;
    if insight.ticker != symbol {
        return None;
    }
    let context = insight.context.as_ref()?;
    if context.stale_context || context.articles.is_empty() {
        Some(app.t(Key::DetailsHeadlinesNoFresh).to_string())
    } else {
        None
    }
}

pub(super) fn backend_source_label(app: &App, article: &InsightArticle) -> String {
    if article.source.trim().is_empty() {
        app.t(Key::DetailsHeadlinesSourceUnknown).to_string()
    } else {
        article.source.clone()
    }
}

pub(super) fn backend_age_label(app: &App, article: &InsightArticle) -> String {
    if let Some(age_hours) = article.age_hours {
        format!("{age_hours:.0}h")
    } else if let Some(published_at) = &article.published_at {
        published_at.clone()
    } else {
        app.t(Key::DetailsHeadlinesFresh).to_string()
    }
}

pub(super) fn normalize_headline_key(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| character.to_lowercase())
        .filter(|character| character.is_ascii_alphanumeric() || character.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
