use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version = VERSION_MESSAGE, about)]
pub struct Args {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    /// The path to a registry directory containing a `registry.cbor` file.
    /// By default, the current working directory is used.
    #[arg(short('d'), long, default_value = ".")]
    pub registry_directory: PathBuf,

    /// Enforce a maximum width of the user interface.
    #[arg(short('W'), long)]
    pub force_max_width: Option<u16>,

    /// Enforce a maximum height of the user interface.
    #[arg(short('H'), long)]
    pub force_max_height: Option<u16>,
}

pub const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);
