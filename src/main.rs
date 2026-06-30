use std::io;

mod app;
mod event;
mod pages;
mod theme;
mod ui;

use app::App;
use crossterm::event as crossterm_event;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;

        let event = crossterm_event::read()?;
        event::handle_event(&mut app, event);
    }

    ratatui::restore();
    Ok(())
}
