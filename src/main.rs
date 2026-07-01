use std::io;
use std::time::Duration;

mod app;
mod event;
mod market;
mod pages;
mod quotes;
mod theme;
mod ui;

use app::App;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event as crossterm_event, execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0),
        Hide
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    let market_events = market::spawn_binance_stream();

    let result = run_app(&mut terminal, &mut app, market_events);

    execute!(
        terminal.backend_mut(),
        Show,
        MoveTo(0, 0),
        Clear(ClearType::All),
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    market_events: std::sync::mpsc::Receiver<market::MarketEvent>,
) -> io::Result<()> {
    while !app.should_quit {
        while let Ok(event) = market_events.try_recv() {
            app.handle_market_event(event);
        }

        terminal.draw(|frame| ui::render(frame, &app))?;

        if crossterm_event::poll(Duration::from_millis(100))? {
            let event = crossterm_event::read()?;
            event::handle_event(app, event);
        }
    }

    Ok(())
}
