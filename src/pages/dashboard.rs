use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    app::{App, PanelId, WindowKind},
    i18n::Key,
    pages::{calendar, fill::Fill, news, notes, sec, watchlist},
    theme::current_theme,
    ui,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area(), app);
    if let Some(background) = theme.background {
        frame.render_widget(Fill::new(background), area);
    }

    let geometry = dashboard_geometry(area, app);

    render_panel(frame, app, geometry.news, PanelId::News);
    render_panel(frame, app, geometry.watchlist, PanelId::Watchlist);
    render_panel(frame, app, geometry.calendar, PanelId::Calendar);
    render_panel(frame, app, geometry.notes, PanelId::Notes);
    render_dividers(frame, app, &geometry);

    if app.show_help {
        render_help(frame, app);
    }
}

fn render_panel(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    match app.panel_content(panel_id) {
        WindowKind::News => news::render(frame, app, area, panel_id),
        WindowKind::Watchlist => watchlist::render(frame, app, area, panel_id),
        WindowKind::Calendar => calendar::render(frame, app, area, panel_id),
        WindowKind::Notes => notes::render(frame, app, area, panel_id),
        WindowKind::Sec => sec::render(frame, app, area, panel_id),
        WindowKind::Picker => render_picker(frame, app, area, panel_id),
    }
}

fn render_picker(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let menu_area = centered_rect(area, 22, WindowKind::CHOICES.len() as u16);
    let mut lines = Vec::new();

    for (index, window_kind) in WindowKind::CHOICES.iter().enumerate() {
        let marker = if index == app.window_picker_index {
            ">"
        } else {
            " "
        };

        let style = if index == app.window_picker_index {
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), style),
            Span::styled(app.t(window_kind.label_key()), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), menu_area);
}

#[derive(Debug, Clone)]
struct DashboardGeometry {
    news: Rect,
    watchlist: Rect,
    calendar: Rect,
    notes: Rect,
    dividers: Vec<Divider>,
    intersections: Vec<Intersection>,
}

#[derive(Debug, Clone, Copy)]
struct Divider {
    area: Rect,
    glyph: char,
}

#[derive(Debug, Clone, Copy)]
struct Intersection {
    area: Rect,
    glyph: char,
}

fn dashboard_geometry(area: Rect, app: &App) -> DashboardGeometry {
    let news_open = app.is_panel_open(PanelId::News);
    let watchlist_open = app.is_panel_open(PanelId::Watchlist);
    let calendar_open = app.is_panel_open(PanelId::Calendar);
    let notes_open = app.is_panel_open(PanelId::Notes);

    let top_split = app.dashboard_layout.top_divider_column(area.width);
    let bottom_split = app.dashboard_layout.bottom_divider_column(area.width);
    let row_split = app.dashboard_layout.divider_row(area.height);

    let top_height = match (news_open || watchlist_open, calendar_open || notes_open) {
        (true, true) => row_split,
        (true, false) => area.height,
        (false, true) => 0,
        (false, false) => 0,
    };
    let bottom_y = if (news_open || watchlist_open) && (calendar_open || notes_open) {
        area.y.saturating_add(top_height).saturating_add(1)
    } else if news_open || watchlist_open {
        area.y.saturating_add(area.height)
    } else {
        area.y
    };
    let bottom_height = area.height.saturating_sub(bottom_y.saturating_sub(area.y));

    let (news_x, news_width, watchlist_x, watchlist_width) =
        row_widths(area, top_split, news_open, watchlist_open);
    let (calendar_x, calendar_width, notes_x, notes_width) =
        row_widths(area, bottom_split, calendar_open, notes_open);

    let news = Rect::new(news_x, area.y, news_width, top_height);
    let watchlist = Rect::new(watchlist_x, area.y, watchlist_width, top_height);
    let calendar = Rect::new(calendar_x, bottom_y, calendar_width, bottom_height);
    let notes = Rect::new(notes_x, bottom_y, notes_width, bottom_height);

    let mut dividers = Vec::new();
    let mut intersections = Vec::new();
    let has_top_vertical = news_open && watchlist_open;
    let has_bottom_vertical = calendar_open && notes_open;
    let has_horizontal = (news_open || watchlist_open) && (calendar_open || notes_open);

    if has_top_vertical {
        dividers.push(Divider {
            area: Rect::new(area.x.saturating_add(top_split), area.y, 1, top_height),
            glyph: '│',
        });
    }
    if has_bottom_vertical {
        dividers.push(Divider {
            area: Rect::new(
                area.x.saturating_add(bottom_split),
                bottom_y,
                1,
                bottom_height,
            ),
            glyph: '│',
        });
    }
    if has_horizontal {
        dividers.push(Divider {
            area: Rect::new(area.x, area.y.saturating_add(top_height), area.width, 1),
            glyph: '─',
        });
        let top_x = area.x.saturating_add(top_split);
        let bottom_x = area.x.saturating_add(bottom_split);
        let y = area.y.saturating_add(top_height);

        if has_top_vertical && has_bottom_vertical && top_x == bottom_x {
            intersections.push(Intersection {
                area: Rect::new(top_x, y, 1, 1),
                glyph: '┼',
            });
        } else {
            if has_top_vertical {
                intersections.push(Intersection {
                    area: Rect::new(top_x, y, 1, 1),
                    glyph: '┴',
                });
            }
            if has_bottom_vertical {
                intersections.push(Intersection {
                    area: Rect::new(bottom_x, y, 1, 1),
                    glyph: '┬',
                });
            }
        }
    }

    DashboardGeometry {
        news,
        watchlist,
        calendar,
        notes,
        dividers,
        intersections,
    }
}

fn row_widths(area: Rect, split: u16, left_open: bool, right_open: bool) -> (u16, u16, u16, u16) {
    match (left_open, right_open) {
        (true, true) => (
            area.x,
            split,
            area.x.saturating_add(split).saturating_add(1),
            area.width.saturating_sub(split).saturating_sub(1),
        ),
        (true, false) => (area.x, area.width, area.x.saturating_add(area.width), 0),
        (false, true) => (area.x, 0, area.x, area.width),
        (false, false) => (area.x, 0, area.x, 0),
    }
}

fn render_dividers(frame: &mut Frame, app: &App, geometry: &DashboardGeometry) {
    let theme = current_theme(app.theme_name);
    let divider_style = Style::default().fg(theme.muted).add_modifier(Modifier::DIM);

    for divider in &geometry.dividers {
        let widget = match divider.glyph {
            '│' => Paragraph::new(vertical_line(divider.area.height)).style(divider_style),
            '─' => Paragraph::new("─".repeat(divider.area.width as usize)).style(divider_style),
            _ => Paragraph::new("").style(divider_style),
        };

        frame.render_widget(widget, divider.area);
    }

    for intersection in &geometry.intersections {
        frame.render_widget(
            Paragraph::new(intersection.glyph.to_string()).style(divider_style),
            intersection.area,
        );
    }
}

fn vertical_line(height: u16) -> Vec<Line<'static>> {
    (0..height).map(|_| Line::from("│")).collect()
}

fn render_help(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = centered_rect(frame.area(), 50, 17);
    let text = [
        app.t(Key::DashboardHelpFocusTab),
        app.t(Key::DashboardHelpFocusNp),
        app.t(Key::DashboardHelpMoveFocus),
        app.t(Key::DashboardHelpResize),
        app.t(Key::DashboardHelpChangePane),
        app.t(Key::DashboardHelpSettings),
        app.t(Key::DashboardHelpEditWatchlist),
        app.t(Key::DashboardHelpSplit),
        app.t(Key::DashboardHelpAddPanel),
        app.t(Key::DashboardHelpSearch),
        app.t(Key::DashboardHelpToggleLocale),
        app.t(Key::DashboardHelpDrag),
        app.t(Key::DashboardHelpClick),
        app.t(Key::DashboardHelpClosePanel),
        app.t(Key::DashboardHelpReset),
        app.t(Key::DashboardHelpCloseHelp),
        app.t(Key::DashboardHelpQuit),
    ]
    .join("\n");

    let modal_background = theme.background.unwrap_or(Color::Rgb(0, 0, 0));
    let title = if app.pending_split {
        app.t(Key::DashboardHelpTitleSplit)
    } else {
        app.t(Key::DashboardHelpTitle)
    };
    let help = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(theme.foreground).bg(modal_background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(modal_background))
                .border_style(Style::default().fg(theme.accent))
                .title(title),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(modal_background)),
        area,
    );
    frame.render_widget(help, area);
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
