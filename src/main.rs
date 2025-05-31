#![feature(let_chains)]
#![feature(iter_intersperse)]
#![feature(async_trait_bounds)]
#![feature(const_type_name)]
#![feature(try_trait_v2)]

use std::sync::Arc;

use args::Args;
use clap::Parser;
use color_eyre::Result;
use tracing::{Instrument, debug};

use crate::app::App;

mod action;
mod app;
mod args;
mod cbor;
mod color;
mod component;
mod components;
mod env;
mod error;
mod geometry;
mod layout;
mod logging;
mod rect;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::error::init()?;
    let tracing_guard = crate::logging::init()?;

    async move {
        let args = Arc::new(Args::parse());
        let mut app = App::new(&args).await?;
        app.run().await?;
        Ok(()) as Result<()>
    }
    .instrument(tracing::info_span!("main"))
    .await?;

    debug!("Exited successfully.");
    drop(tracing_guard);

    Ok(())
}
