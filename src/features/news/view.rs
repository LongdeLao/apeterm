use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, NewsFilterTab, NewsListRow, PanelId},
    i18n::Key,
    news::{NewsItem, NewsPriority},
    theme::current_theme,
    ui::panel,
};

const TIME_COL_WIDTH: u16 = 5;
const SOURCE_COL_WIDTH: u16 = 14;

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let inner = panel::content_area(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    panel::render_title(frame, app, chunks[0], panel_id, app.t(Key::PanelTitleNews));
    render_tabs(frame, app, chunks[1]);

    let rows = app.news_visible_rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(app.news_empty_message())
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: true }),
            chunks[2],
        );
    } else if chunks[2].width >= 96 {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(58),
                Constraint::Length(1),
                Constraint::Percentage(42),
            ])
            .split(chunks[2]);
        render_news_list(frame, app, panes[0], &rows);
        render_gap(frame, panes[1]);
        let selected = rows.get(app.news_selection).and_then(|row| match row {
            NewsListRow::Item(index) => app.news_items.get(*index),
            NewsListRow::Group { .. } => None,
        });
        render_side_detail(frame, app, panes[2], selected);
    } else {
        render_news_list(frame, app, chunks[2], &rows);
    }

    if let Some(item) = &app.selected_news {
        render_detail(frame, app, area, item);
    }
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let tabs = [
        (NewsFilterTab::All, "ALL"),
        (NewsFilterTab::Watchlist, "WATCHLIST"),
        (NewsFilterTab::Macro, "MACRO"),
        (NewsFilterTab::Reddit, "REDDIT"),
        (NewsFilterTab::Crypto, "CRYPTO"),
    ];
    let tab_count = tabs.len();
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.muted));
    frame.render_widget(block, area);

    let line = Line::from(
        tabs.into_iter()
            .enumerate()
            .flat_map(|(index, (tab, label))| {
                let selected = app.news_filter_tab == tab;
                let style = if selected {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.muted)
                };

                let mut spans = vec![Span::styled(format!(" {label} "), style)];
                if index + 1 < tab_count {
                    spans.push(Span::raw("  "));
                }
                spans
            })
            .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(line), area);
}

fn render_news_list(frame: &mut Frame, app: &App, area: Rect, rows: &[NewsListRow]) {
    let inner = area;

    if inner.height == 0 {
        return;
    }

    let scroll = app.news_scroll.min(rows.len().saturating_sub(1));
    let table_rows = rows
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .enumerate()
        .map(|(offset, row)| {
            let selected = scroll + offset == app.news_selection;
            render_table_row(app, row, selected, inner.width)
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(TIME_COL_WIDTH),
            Constraint::Length(SOURCE_COL_WIDTH),
            Constraint::Min(12),
        ],
    )
    .column_spacing(1);
    frame.render_widget(table, inner);
}

fn render_table_row(app: &App, row: &NewsListRow, selected: bool, width: u16) -> Row<'static> {
    let theme = current_theme(app.theme_name);
    let selected_style = if selected {
        Style::default().bg(theme.surface)
    } else {
        Style::default()
    };

    match row {
        NewsListRow::Group {
            symbol,
            count,
            expanded,
        } => {
            let marker = if *expanded { "▼" } else { "▶" };
            Row::new(vec![
                Cell::from(Line::from(Span::styled(
                    marker.to_string(),
                    Style::default()
                        .fg(theme.positive)
                        .add_modifier(Modifier::BOLD),
                ))),
                Cell::from(Line::from(Span::styled(
                    "WATCH",
                    Style::default().fg(theme.muted),
                ))),
                Cell::from(Line::from(Span::styled(
                    format!("{symbol} ({count})"),
                    Style::default()
                        .fg(theme.positive)
                        .add_modifier(Modifier::BOLD),
                ))),
            ])
            .style(selected_style)
        }
        NewsListRow::Item(index) => {
            let Some(item) = app.news_items.get(*index) else {
                return Row::new(vec![Cell::from(""), Cell::from(""), Cell::from("")]);
            };

            let time = app.news_timestamp(item.published_at);
            let source = render_source_cell(item, app.theme_name);
            let symbol_prefix = item
                .symbols
                .first()
                .map(|symbol| {
                    Span::styled(
                        format!("{symbol} "),
                        Style::default()
                            .fg(theme.positive)
                            .add_modifier(Modifier::BOLD),
                    )
                })
                .into_iter()
                .collect::<Vec<_>>();
            let headline_width = width
                .saturating_sub(TIME_COL_WIDTH + SOURCE_COL_WIDTH + 4)
                .max(12) as usize;
            let headline = truncate_with_ellipsis(item.title.as_str(), headline_width);
            let mut headline_spans = symbol_prefix;
            headline_spans.push(Span::styled(
                headline,
                if selected {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.foreground)
                },
            ));

            Row::new(vec![
                Cell::from(Line::from(Span::styled(
                    format!("{time:>4}"),
                    Style::default().fg(theme.muted),
                ))),
                Cell::from(source),
                Cell::from(Line::from(headline_spans)),
            ])
            .style(selected_style)
        }
    }
}

fn render_source_cell(item: &NewsItem, theme_name: crate::app::ThemeName) -> Line<'static> {
    let theme = current_theme(theme_name);
    let badge = source_badge(&item.source);
    let mut spans = vec![Span::styled(
        format!("{badge:<6}"),
        Style::default().fg(theme.muted),
    )];
    match item.priority {
        NewsPriority::Critical => spans.push(Span::styled(
            " !",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        NewsPriority::High => spans.push(Span::styled(
            " +",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        NewsPriority::Medium | NewsPriority::Low => {}
    }
    Line::from(spans)
}

fn render_gap(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(" ").style(Style::default().bg(Color::Reset)),
        area,
    );
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect, item: &NewsItem) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(theme.surface);
    let modal = centered_rect(area, 78, 20);
    let published = app.news_absolute_timestamp(item.published_at);
    let symbols = app
        .news_symbols_label(&item.symbols)
        .unwrap_or_else(|| "-".to_string());

    let mut lines = vec![
        Line::from(Span::styled(
            item.title.as_str(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line(app.t(Key::NewsDetailSource), item.source.as_str(), theme),
        detail_line(app.t(Key::NewsDetailPublished), published.as_str(), theme),
        detail_line(
            app.t(Key::NewsDetailPriority),
            app.news_priority_label(item.priority),
            theme,
        ),
        detail_line(app.t(Key::NewsDetailSymbols), symbols.as_str(), theme),
    ];

    if let Some(author) = &item.author {
        lines.push(detail_line(app.t(Key::NewsDetailAuthor), author, theme));
    }

    lines.push(detail_line(
        app.t(Key::NewsDetailLink),
        item.url.as_str(),
        theme,
    ));

    if let Some(description) = &item.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            app.t(Key::NewsDetailSummary),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            description.as_str(),
            Style::default().fg(theme.foreground),
        )));
    }

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.foreground).bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" article ")
                .style(Style::default().bg(background))
                .border_style(Style::default().fg(theme.accent)),
        );

    frame.render_widget(Clear, modal);
    frame.render_widget(panel, modal);
}

fn render_side_detail(frame: &mut Frame, app: &App, area: Rect, item: Option<&NewsItem>) {
    let theme = current_theme(app.theme_name);
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y,
        width: area.width.saturating_sub(1),
        height: area.height,
    };

    let lines = if let Some(item) = item {
        let published = app.news_absolute_timestamp(item.published_at);
        let symbols = app
            .news_symbols_label(&item.symbols)
            .unwrap_or_else(|| "-".to_string());
        vec![
            Line::from(Span::styled(
                item.title.as_str(),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            owned_detail_line(app.t(Key::NewsDetailSource), item.source.as_str(), theme),
            owned_detail_line(app.t(Key::NewsDetailPublished), &published, theme),
            owned_detail_line(
                app.t(Key::NewsDetailPriority),
                app.news_priority_label(item.priority),
                theme,
            ),
            owned_detail_line(app.t(Key::NewsDetailSymbols), &symbols, theme),
            Line::from(""),
            Line::from(Span::styled(
                item.description
                    .as_deref()
                    .unwrap_or("No summary available."),
                Style::default().fg(theme.foreground),
            )),
            Line::from(""),
            Line::from(Span::styled(
                item.url.as_str(),
                Style::default().fg(theme.muted),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Select a headline",
            Style::default().fg(theme.muted),
        ))]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.foreground)),
        inner,
    );
}

fn detail_line<'a>(label: &'a str, value: &'a str, theme: crate::theme::Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(theme.muted)),
        Span::styled(value, Style::default().fg(theme.foreground)),
    ])
}

fn owned_detail_line(label: &str, value: &str, theme: crate::theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(theme.muted)),
        Span::styled(value.to_string(), Style::default().fg(theme.foreground)),
    ])
}

fn source_badge(source: &str) -> String {
    match source {
        "FinancialJuice" => "FJ".to_string(),
        "Bloomberg" => "BBG".to_string(),
        "Reuters" => "RTRS".to_string(),
        "NASDAQ" => "NDAQ".to_string(),
        "Yahoo Finance" => "YF".to_string(),
        "Wall Street Journal" => "WSJ".to_string(),
        "Financial Times" => "FT".to_string(),
        "MarketWatch" => "MW".to_string(),
        "CNBC" => "CNBC".to_string(),
        other => other
            .chars()
            .take(6)
            .collect::<String>()
            .to_ascii_uppercase(),
    }
}

fn truncate_with_ellipsis(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(value) <= width {
        return value.to_string();
    }

    let mut result = String::new();
    for character in value.chars() {
        let next = format!("{result}{character}");
        if UnicodeWidthStr::width(next.as_str()) + 1 > width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height: height.min(area.height),
    }
}
