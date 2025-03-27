#![allow(dead_code)] // Remove this once you start using the code

use std::{collections::HashMap, env, path::PathBuf};

use color_eyre::{owo_colors::OwoColorize, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_deref::{Deref, DerefMut};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use ratatui::style::{Color, Modifier, Style};
use serde::{de::Deserializer, Deserialize};
use tracing::error;

use crate::action::{Action, FocusChange, FocusChangeDirection, FocusChangeScope};

lazy_static! {
    pub static ref PKG_NAME: String = env!("CARGO_PKG_NAME").to_string();
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_string();
    pub static ref PROJECT_VERSION: String = env!("CARGO_PKG_VERSION").to_string();
}
