/*
 * Copyright (c) 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::Rc;

use imgui_support::xplane::System;
use tracing::{debug, error, info, trace, warn};
use xplm::command::{CommandHandler, OwnedCommand};
use xplm::menu::{CheckHandler, CheckItem, Menu};
use xplm::plugin::Plugin;
use xplm_ext::logging;
use xplm_ext::plugin::utils::get_current_aircraft_path;
use xplm_sys::{XPLM_MSG_LIVERY_LOADED, XPLM_MSG_PLANE_UNLOADED};

use hints_common::{ConfigError, FROM_EDGE_MIN, FROM_EDGE_PROPORTION, get_offset_from_edge, HEIGHT, Hints, HintsEvent, LOGGING_ENV_VAR, TITLE, WIDTH};

struct HintPlugin {
    internals: Option<Internals>,
    aircraft_loaded: bool,
}

struct Internals {
    _menu: Menu,
    _next_command: OwnedCommand,
    _previous_command: OwnedCommand,
    _toggle_window_command: OwnedCommand,
}

impl Internals {
    fn new() -> Option<Self> {
        let path = find_path();
        if path.is_none() {
            error!("Unable to find hints directory - plugin will do nothing");
            return None;
        }
        let app = Rc::new(RefCell::new(
            Hints::new(&path.unwrap()).expect("Unable to create Hints app"),
        ));
        let system = Rc::new(RefCell::new(init_xplane(Rc::clone(&app))));
        let menu = Menu::new("FLChints").expect("Unable to create hints menu");
        let toggle = Rc::new(
            CheckItem::new(
                "Show hints",
                false,
                ToggleWindowCheckHandler {
                    system: Rc::clone(&system),
                },
            )
                .expect("Unable to create show hints window menu item"),
        );
        let toggle_command_handler = ToggleWindowCommandHandler {
            system,
            toggle: Rc::clone(&toggle),
        };
        menu.add_child::<Rc<CheckItem>, CheckItem>(toggle);
        menu.add_to_plugins_menu();
        Some(Internals {
            _menu: menu,
            _next_command: create_event_sending_command(
                "flc/hints/next",
                "Show next hint",
                HintsEvent::NextHint,
                Rc::clone(&app),
            ),
            _previous_command: create_event_sending_command(
                "flc/hints/previous",
                "Show previous hint",
                HintsEvent::PreviousHint,
                app,
            ),
            _toggle_window_command: create_owned_command(
                "flc/hints/toggle",
                "Toggle window visibility",
                toggle_command_handler,
            ),
        })
    }
}

impl Plugin for HintPlugin {
    type Error = ConfigError;

    fn start() -> Result<Self, Self::Error> {
        logging::init(LOGGING_ENV_VAR, false);
        trace!("start()");
        Ok(HintPlugin {
            internals: None,
            aircraft_loaded: false,
        })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        trace!("enable()");
        if self.aircraft_loaded {
            self.internals = Internals::new();
        }
        Ok(())
    }

    fn disable(&mut self) {
        trace!("disable()");
        self.internals.take();
    }

    fn info(&self) -> xplm::plugin::PluginInfo {
        xplm::plugin::PluginInfo {
            name: String::from("FLChints"),
            signature: String::from("uk.co.flightlevelchange.hints"),
            description: String::from(
                "Displays a set of hint images for the current aircraft",
            ),
        }
    }

    fn receive_message(&mut self, _from: i32, message: i32, _param: *mut c_void) {
        trace!("Received message {message}");
        #[allow(clippy::cast_sign_loss)]
        match message as u32 {
            XPLM_MSG_LIVERY_LOADED => {
                debug!("Livery loaded");
                self.aircraft_loaded = true;
                self.internals = Internals::new();
            }
            XPLM_MSG_PLANE_UNLOADED => {
                debug!("Plane unloaded");
                self.aircraft_loaded = false;
                self.internals.take();
            }
            _ => {}
        }
    }
}

xplm::xplane_plugin!(HintPlugin);

fn create_event_sending_command(
    name: &str,
    description: &str,
    event: HintsEvent,
    app: Rc<RefCell<Hints>>,
) -> OwnedCommand {
    create_owned_command(name, description, EventSendingCommandHandler { app, event })
}

fn create_owned_command<T: CommandHandler>(
    name: &str,
    description: &str,
    handler: T,
) -> OwnedCommand {
    OwnedCommand::new(name, description, handler).expect("Unable to create command '{name}'")
}

struct EventSendingCommandHandler {
    app: Rc<RefCell<Hints>>,
    event: HintsEvent,
}

impl CommandHandler for EventSendingCommandHandler {
    fn command_begin(&mut self) {
        self.app.borrow_mut().handle_hints_event(self.event);
    }
    fn command_continue(&mut self) {}
    fn command_end(&mut self) {}
}

struct ToggleWindowCommandHandler {
    system: Rc<RefCell<System>>,
    toggle: Rc<CheckItem>,
}

impl CommandHandler for ToggleWindowCommandHandler {
    fn command_begin(&mut self) {
        let new_visibility = self.system.borrow_mut().toggle_hint_window();
        self.toggle.set_checked(new_visibility);
    }
    fn command_continue(&mut self) {}
    fn command_end(&mut self) {}
}

struct ToggleWindowCheckHandler {
    system: Rc<RefCell<System>>,
}

impl CheckHandler for ToggleWindowCheckHandler {
    fn item_checked(&mut self, _: &CheckItem, checked: bool) {
        self.system.borrow_mut().set_hint_window_visible(checked);
    }
}

fn find_path() -> Option<PathBuf> {
    let aircraft_path = get_current_aircraft_path().join("hints");
    info!("Looking for hints in {aircraft_path:?}");
    if aircraft_path.is_dir() {
        info!("Loading hints from {aircraft_path:?}");
        Some(aircraft_path)
    } else {
        warn!("No hints found in {aircraft_path:?}");
        None
    }
}

fn init_xplane(app: Rc<RefCell<Hints>>) -> System {
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
