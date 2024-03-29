/*
 * Copyright (c) 2023 Flight Level Change Ltd.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use std::path::PathBuf;

use glfw::fail_on_errors;
use tracing_subscriber::layer::SubscriberExt;

use hints_common::logging::{env_filter, layer};
use hints_common::{
    get_offset_from_edge, Hints, FROM_EDGE_MIN, FROM_EDGE_PROPORTION, HEIGHT, LOGGING_ENV_VAR,
    TITLE, WIDTH,
};

fn main() {
    let stdout_layer = layer(false, None);
    let filter = env_filter(Some(LOGGING_ENV_VAR));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    let mut glfw = glfw::init(fail_on_errors!()).expect("GLFW failed to init");
    glfw.window_hint(glfw::WindowHint::ContextVersion(2, 1));

    let bounds = imgui_support_standalone::get_screen_bounds(&mut glfw);
    let horiz_offset = get_offset_from_edge(bounds.width(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    let vert_offset = get_offset_from_edge(bounds.height(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    {
        let mut system = imgui_support_standalone::init(
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
    assert_eq!(
        args.len(),
        2,
        "Expected exactly one argument: path to config file"
    );
    PathBuf::from(&args[1])
}
