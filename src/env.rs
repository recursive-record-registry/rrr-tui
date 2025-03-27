#![allow(dead_code)] // Remove this once you start using the code

use std::env;

use lazy_static::lazy_static;


lazy_static! {
    pub static ref PKG_NAME: String = env!("CARGO_PKG_NAME").to_string();
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_string();
    pub static ref PROJECT_VERSION: String = env!("CARGO_PKG_VERSION").to_string();
}
