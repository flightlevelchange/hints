/*
 * Copyright (c) 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use thiserror::Error;

pub use crate::app::{Hints, HintsEvent};

mod app;
mod concurrent;
mod hints;

pub mod logging;

pub const TITLE: &str = "Hints";
pub const WIDTH: u32 = 400;
pub const HEIGHT: u32 = 300;
pub const FROM_EDGE_PROPORTION: u32 = 20;
pub const FROM_EDGE_MIN: u32 = 50;

pub const LOGGING_ENV_VAR: &str = "HINTS_LOG";

#[derive(Error, Debug)]
#[error("Unable to load hints: {msg}")]
pub struct ConfigError {
    msg: String,
}

impl ConfigError {
    #[must_use]
    pub fn new(msg: String) -> Self {
        ConfigError { msg }
    }
}

#[must_use]
pub fn get_offset_from_edge(size: u32, proportion: u32, min: u32) -> u32 {
    (size / proportion).min(min)
}
