//! Optional broker adapters. Broker credentials remain owned by each adapter.

pub mod trade_republic;

use std::{error::Error, io};

use crate::config::AppConfig;

pub fn handle_cli(
    mut config: AppConfig,
    args: &[String],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match args.first().map(String::as_str) {
        Some("connect") => {
            trade_republic::connect()?;
            config.broker.trade_republic_enabled = true;
            config.save()?;
            println!(
                "Trade Republic connected. Run `apeterm broker sync` to import the portfolio."
            );
        }
        Some("sync") => {
            if !config.broker.trade_republic_enabled {
                return Err(io::Error::other(
                    "Trade Republic is disabled; run `apeterm broker connect` first",
                )
                .into());
            }
            let snapshot = trade_republic::sync(&config.broker.portfolio_cache_path)?;
            println!(
                "synced {} Trade Republic positions to {}",
                snapshot.positions.len(),
                config.broker.portfolio_cache_path.display()
            );
        }
        Some("disconnect") => {
            config.broker.trade_republic_enabled = false;
            config.save()?;
            println!("Trade Republic disabled. pytr credentials were left untouched.");
        }
        Some("status") | None => {
            println!(
                "Trade Republic: {}",
                if config.broker.trade_republic_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!(
                "Portfolio cache: {}",
                config.broker.portfolio_cache_path.display()
            );
            println!("pytr available: {}", trade_republic::available());
        }
        Some(other) => {
            return Err(io::Error::other(format!(
                "unknown broker command `{other}`; use connect, sync, status, or disconnect"
            ))
            .into());
        }
    }
    Ok(())
}
