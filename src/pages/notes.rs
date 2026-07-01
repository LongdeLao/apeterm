use ratatui::{Frame, layout::Rect};

use crate::{app::App, app::PanelId, i18n::Key, pages::panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    let session = if app.logged_in {
        app.t(Key::NotesSessionLoggedIn)
    } else {
        app.t(Key::NotesSessionLoggedOut)
    };
    let lines = [
        app.t(Key::NotesEmpty).to_string(),
        format!("{}: {}", app.t(Key::NotesLanguage), locale_label(app)),
        format!(
            "{}: {}",
            app.t(Key::NotesTheme),
            app.t(app.theme_name.label_key())
        ),
        session.to_string(),
    ];
    let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    panel::render(
        frame,
        app,
        area,
        panel_id,
        app.t(Key::PanelTitleNotes),
        &line_refs,
    );
}

fn locale_label(app: &App) -> &str {
    app.locale
        .language_key()
        .map(|key| app.t(key))
        .unwrap_or_else(|| app.locale.code())
}
