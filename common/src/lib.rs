/*
 * Copyright (c) 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

pub use crate::app::{Hints, HintsEvent};

mod app;
mod concurrent;
mod hints;

pub const TITLE: &str = "Hints";
pub const WIDTH: u32 = 400;
pub const HEIGHT: u32 = 300;
pub const FROM_EDGE_PROPORTION: u32 = 20;
pub const FROM_EDGE_MIN: u32 = 50;

pub const LOGGING_ENV_VAR: &str = "HINTS_LOG";

#[must_use]
pub fn get_offset_from_edge(size: u32, proportion: u32, min: u32) -> u32 {
    (size / proportion).min(min)
}
