/*
 * Copyright (c) 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::path::PathBuf;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use hints_common::{FROM_EDGE_MIN, FROM_EDGE_PROPORTION, get_offset_from_edge, HEIGHT, Hints, TITLE, WIDTH};

fn main() {
    #[cfg(target_os = "windows")]
        let ansi = false;
    #[cfg(not(target_os = "windows"))]
        let ansi = true;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(ansi)
        .with_thread_names(true);
    let filter_layer = EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("GLFW failed to init");
    glfw.window_hint(glfw::WindowHint::ContextVersion(2, 1));

    let bounds = imgui_support::standalone::get_screen_bounds(&mut glfw);
    let horiz_offset = get_offset_from_edge(bounds.width(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    let vert_offset = get_offset_from_edge(bounds.height(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    {
        let mut system = imgui_support::standalone::init(
            glfw,
            TITLE,
            bounds.width() - horiz_offset - WIDTH,
            vert_offset + FROM_EDGE_MIN as i32 as u32,
            WIDTH,
            HEIGHT,
            Hints::new(get_path()).expect("Unable to create Hints app"),
        );
        system.main_loop();
    }
}

fn get_path() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 2, "Expected exactly one argument: path to config file");
    PathBuf::from(&args[1])
}
