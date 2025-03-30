#![feature(let_chains)]

use args::Args;
use clap::Parser;
use color_eyre::Result;
use tracing::{debug, Instrument};

use crate::app::App;

mod action;
mod app;
mod args;
mod cbor;
mod components;
mod env;
mod errors;
mod logging;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    let tracing_guard = crate::logging::init()?;

    async move {
        let args = Args::parse();
        let mut app = App::new(args).await?;
        app.run().await?;
        Ok(()) as Result<()>
    }
    .instrument(tracing::info_span!("main"))
    .await?;

    debug!("Exited successfully.");
    drop(tracing_guard);

    Ok(())
}
