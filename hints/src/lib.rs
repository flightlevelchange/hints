/*
 * Copyright © 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

#[cfg(feature = "xplane")]
use std::cell::RefCell;
#[cfg(feature = "xplane")]
use std::rc::Rc;

pub use crate::app::{ConfigLocation, Hints, HintsEvent};

#[cfg(not(any(feature = "standalone", feature = "xplane")))]
compile_error!("One of the features ['standalone', 'xplane'] must be enabled");

mod app;
mod hints;

const TITLE: &str = "Hints";
const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;
const FROM_EDGE_PROPORTION: u32 = 20;
const FROM_EDGE_MIN: u32 = 50;

#[cfg(feature = "standalone")]
pub fn run_standalone() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

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
            Hints::new(ConfigLocation::FromArgs).expect("Unable to create Hints app"),
        );
        system.main_loop();
    }
}

#[cfg(feature = "xplane")]
pub fn init_xplane(app: Rc<RefCell<Hints>>) -> imgui_support::xplane::System {
    let bounds = imgui_support::xplane::get_screen_bounds();
    let horiz_offset = get_offset_from_edge(bounds.width(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    let vert_offset = get_offset_from_edge(bounds.height(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    imgui_support::xplane::init(
        TITLE,
        bounds.width() - horiz_offset - WIDTH,
        vert_offset * 2,
        WIDTH,
        HEIGHT,
        app,
    )
}

fn get_offset_from_edge(size: u32, proportion: u32, min: u32) -> u32 {
    (size / proportion).min(min)
}
