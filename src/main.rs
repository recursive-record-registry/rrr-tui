use args::Args;
use clap::Parser;
use color_eyre::Result;
use tracing::Instrument;

use crate::app::App;

mod action;
mod app;
mod args;
mod components;
mod config;
mod errors;
mod logging;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    async move {
        let args = Args::parse();
        let mut app = App::new(args)?;
        app.run().await?;
        Ok(())
    }
    .instrument(tracing::info_span!("main"))
    .await
}
