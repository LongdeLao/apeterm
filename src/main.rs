use std::time::Duration;
use std::{error::Error, io};

mod app;
mod config;
mod db;
mod enrich;
mod event;
mod i18n;
mod import;
mod market;
mod pages;
mod quotes;
mod search;
mod theme;
mod ui;

use app::App;
use config::{AppConfig, MetadataProviderKind};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event as crossterm_event, execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use rusqlite::params;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--check-locales") {
        i18n::validate_embedded_locales().map_err(io::Error::other)?;
        println!("locale files are complete");
        return Ok(());
    }

    let config = AppConfig::load()?;
    if args.first().map(String::as_str) == Some("update") {
        return run_update(config, update_limit(&args));
    }

    let _ = db::open(&config.ticker_db_path)?;
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

    let mut app = App::new(config.clone());
    let market_events = market::spawn_market_streams();

    let result = run_app(&mut terminal, &mut app, market_events);

    execute!(
        terminal.backend_mut(),
        Show,
        MoveTo(0, 0),
        Clear(ClearType::All),
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;

    result?;
    Ok(())
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
        app.poll_live_details();

        terminal.draw(|frame| ui::render(frame, &app))?;

        if crossterm_event::poll(Duration::from_millis(100))? {
            let event = crossterm_event::read()?;
            event::handle_event(app, event);
        }
    }

    Ok(())
}

fn run_update(
    config: AppConfig,
    enrich_limit: Option<usize>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let mut connection = db::open(&config.ticker_db_path)?;
        record_update_start(&connection, "daily_update")?;

        println!("importing NASDAQ symbol directories...");
        let import_summary = import::import_nasdaq_directory(&mut connection).await?;
        println!(
            "imported {} active symbols ({} upserted, {} deactivated)",
            import_summary.fetched, import_summary.upserted, import_summary.deactivated
        );

        if config.metadata_provider.provider == MetadataProviderKind::None {
            println!("metadata enrichment disabled; using NASDAQ directory data only");
        } else {
            println!("enriching stale metadata rows...");
            let enrich_summary = if enrich_limit.is_some() {
                enrich::enrich_stale_instruments_with_limit(
                    &mut connection,
                    &config.metadata_provider,
                    &config.update,
                    enrich_limit,
                )
                .await?
            } else {
                enrich::enrich_stale_instruments(
                    &mut connection,
                    &config.metadata_provider,
                    &config.update,
                )
                .await?
            };
            println!(
                "enrichment attempted {}, updated {}, skipped {}, failed {}",
                enrich_summary.attempted,
                enrich_summary.updated,
                enrich_summary.skipped,
                enrich_summary.failed
            );
        }

        record_update_finish(&connection, "daily_update", "ok")?;
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    })
}

fn update_limit(args: &[String]) -> Option<usize> {
    args.windows(2)
        .find(|window| window[0] == "--limit")
        .and_then(|window| window[1].parse::<usize>().ok())
}

fn record_update_start(connection: &rusqlite::Connection, name: &str) -> rusqlite::Result<()> {
    connection.execute(
        "
        INSERT INTO update_runs(name, last_started, status)
        VALUES (?1, ?2, 'running')
        ON CONFLICT(name) DO UPDATE SET
          last_started = excluded.last_started,
          status = excluded.status
        ",
        params![name, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn record_update_finish(
    connection: &rusqlite::Connection,
    name: &str,
    status: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "
        UPDATE update_runs
        SET last_finished = ?2,
            status = ?3
        WHERE name = ?1
        ",
        params![name, chrono::Utc::now().to_rfc3339(), status],
    )?;
    Ok(())
}
