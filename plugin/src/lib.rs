/*
 * Copyright (c) 2023 Flight Level Change Ltd.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

mod utils;

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use imgui_support::geometry::Rect;
use imgui_support_xplane::ui::{PositioningMode, Ref};
use imgui_support_xplane::System;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use xplm::command::{CommandHandler, OwnedCommand};
use xplm::menu::{ActionItem, CheckHandler, CheckItem, Menu, MenuClickHandler};
use xplm::plugin::Plugin;
use xplm_sys::{XPLM_MSG_LIVERY_LOADED, XPLM_MSG_PLANE_UNLOADED};

use crate::utils::{
    get_current_aircraft_filename, get_current_aircraft_icao, get_current_aircraft_path,
    get_prefs_path, XplmWrite,
};
use hints_common::logging::{env_filter, layer};
use hints_common::{
    get_offset_from_edge, ConfigError, Hints, HintsEvent, FROM_EDGE_MIN, FROM_EDGE_PROPORTION,
    HEIGHT, LOGGING_ENV_VAR, TITLE, WIDTH,
};

static LOGGING: OnceLock<()> = OnceLock::new();

struct HintPlugin {
    internals: Option<Internals>,
    aircraft_loaded: bool,
}

struct Internals {
    _menu: Menu,
    _next_command: OwnedCommand,
    _previous_command: OwnedCommand,
    _reload_command: OwnedCommand,
    _toggle_window_command: OwnedCommand,
    _load_command: OwnedCommand,
    _save_command: OwnedCommand,
    _reset_command: OwnedCommand,
}

struct SystemWrapper {
    system: System,
    default_geometry: Rect,
}

impl SystemWrapper {
    fn new(system: System) -> Self {
        let default_geometry = system.window().geometry();
        let mut wrapper = Self {
            system,
            default_geometry,
        };
        wrapper.load(true);
        wrapper
    }

    #[must_use]
    pub fn toggle_hint_window(&mut self) -> bool {
        self.system.window_mut().toggle_visible()
    }

    pub fn set_hint_window_visible(&mut self, visible: bool) {
        self.system.window_mut().set_visible(visible);
    }

    fn save(&self) {
        if let Some(filename) = get_state_path() {
            let state = State::from(self.system.window());
            let toml = toml::to_string_pretty(&state).unwrap();
            match std::fs::write(&filename, toml) {
                Ok(()) => info!("Saved hints window state to {filename:?}"),
                Err(e) => error!("Unable to save hints window state: {e}"),
            }
        }
    }

    fn load(&mut self, quietly: bool) {
        if let Some(filename) = get_state_path() {
            if filename.is_file() {
                match std::fs::read_to_string(&filename) {
                    Ok(toml) => match toml::from_str::<State>(&toml) {
                        Ok(state) => {
                            let window = self.system.window_mut();
                            window.set_positioning_mode(PositioningMode::from(&state.mode));
                            window.set_geometry(&state.position);
                            window.set_visible(state.visible);
                            info!("Loaded hints window state from {filename:?}");
                        }
                        Err(e) => error!("Unable to parse hints window state: {e}"),
                    },
                    Err(e) => error!("Unable to read from {filename:?}: {e}"),
                }
            } else if !quietly {
                warn!("Unable to find any saved window state to load at {filename:?}");
            }
        }
    }

    fn reset(&mut self) {
        let window = self.system.window_mut();
        window.set_positioning_mode(PositioningMode::Free);
        window.set_visible(true);
        window.set_geometry(&self.default_geometry);
    }
}

impl Internals {
    fn new() -> Option<Self> {
        let path = find_path();
        if path.is_none() {
            error!("Unable to find hints directory - plugin will do nothing");
            return None;
        }
        let app = Rc::new(RefCell::new(
            Hints::new(path.unwrap()).expect("Unable to create FLC Hints app"),
        ));
        let wrapper = Rc::new(RefCell::new(SystemWrapper::new(init_xplane(Rc::clone(
            &app,
        )))));

        let (menu, toggle) = create_menu(&wrapper, &app);

        let toggle_command_handler = ToggleWindowCommandHandler {
            wrapper: Rc::clone(&wrapper),
            toggle: Rc::clone(&toggle),
        };

        let save_command_handler = SaveCommandHandler {
            wrapper: Rc::clone(&wrapper),
        };

        let load_command_handler = LoadCommandHandler {
            wrapper: Rc::clone(&wrapper),
        };

        let reset_command_handler = ResetCommandHandler {
            wrapper: Rc::clone(&wrapper),
        };

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
                Rc::clone(&app),
            ),
            _reload_command: create_event_sending_command(
                "flc/hints/reload",
                "Reload hints from disk",
                HintsEvent::Reload,
                app,
            ),
            _toggle_window_command: create_owned_command(
                "flc/hints/window/toggle",
                "Toggle window visibility",
                toggle_command_handler,
            ),
            _load_command: create_owned_command(
                "flc/hints/window/load",
                "Load window position",
                load_command_handler,
            ),
            _save_command: create_owned_command(
                "flc/hints/window/save",
                "Save window position",
                save_command_handler,
            ),
            _reset_command: create_owned_command(
                "flc/hints/window/reset",
                "Reset window position",
                reset_command_handler,
            ),
        })
    }
}

fn create_menu(
    wrapper: &Rc<RefCell<SystemWrapper>>,
    app: &Rc<RefCell<Hints>>,
) -> (Menu, Rc<CheckItem>) {
    let menu = Menu::new("FLC Hints").expect("Unable to create hints menu");
    let toggle = Rc::new(
        CheckItem::new(
            "Show hints",
            false,
            ToggleWindowCheckHandler {
                wrapper: Rc::clone(wrapper),
            },
        )
        .expect("Unable to create show hints window menu item"),
    );
    menu.add_child::<Rc<CheckItem>, CheckItem>(Rc::clone(&toggle));

    let window_menu = Menu::new("Window position").expect("Unable to create window menu");

    window_menu.add_child(
        ActionItem::new(
            "Load",
            LoadMenuClickHandler {
                wrapper: Rc::clone(wrapper),
            },
        )
        .expect("Unable to create load menu item"),
    );

    window_menu.add_child(
        ActionItem::new(
            "Save",
            SaveMenuClickHandler {
                wrapper: Rc::clone(wrapper),
            },
        )
        .expect("Unable to create save menu item"),
    );

    window_menu.add_child(
        ActionItem::new(
            "Reset",
            ResetMenuClickHandler {
                wrapper: Rc::clone(wrapper),
            },
        )
        .expect("Unable to create reset menu item"),
    );
    menu.add_child(window_menu);

    menu.add_child(
        ActionItem::new(
            "Reload hints from disk",
            ReloadMenuClickHandler {
                app: Rc::clone(app),
            },
        )
        .expect("Unable to create reload menu item"),
    );

    // TODO: add scale by 1.25 / 0.8

    menu.add_to_plugins_menu();
    (menu, toggle)
}

impl Plugin for HintPlugin {
    type Error = ConfigError;

    fn start() -> Result<Self, Self::Error> {
        init_logging(LOGGING_ENV_VAR, false);
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
            name: String::from("FLC Hints"),
            signature: String::from("uk.co.flightlevelchange.hints"),
            description: String::from("Displays a set of hint images for the current aircraft"),
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
    wrapper: Rc<RefCell<SystemWrapper>>,
    toggle: Rc<CheckItem>,
}

impl CommandHandler for ToggleWindowCommandHandler {
    fn command_begin(&mut self) {
        let new_visibility = self.wrapper.borrow_mut().toggle_hint_window();
        self.toggle.set_checked(new_visibility);
    }
    fn command_continue(&mut self) {}
    fn command_end(&mut self) {}
}

struct ToggleWindowCheckHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl CheckHandler for ToggleWindowCheckHandler {
    fn item_checked(&mut self, _: &CheckItem, checked: bool) {
        self.wrapper.borrow_mut().set_hint_window_visible(checked);
    }
}

struct ReloadMenuClickHandler {
    app: Rc<RefCell<Hints>>,
}

impl MenuClickHandler for ReloadMenuClickHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        self.app.borrow_mut().reload();
    }
}

struct LoadCommandHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl CommandHandler for LoadCommandHandler {
    fn command_begin(&mut self) {
        self.wrapper.borrow_mut().load(false);
    }
}

struct LoadMenuClickHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl MenuClickHandler for LoadMenuClickHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        self.wrapper.borrow_mut().load(false);
    }
}

struct SaveCommandHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl CommandHandler for SaveCommandHandler {
    fn command_begin(&mut self) {
        self.wrapper.borrow().save();
    }
}

struct SaveMenuClickHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl MenuClickHandler for SaveMenuClickHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        self.wrapper.borrow().save();
    }
}

struct ResetCommandHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl CommandHandler for ResetCommandHandler {
    fn command_begin(&mut self) {
        self.wrapper.borrow_mut().reset();
    }
}

struct ResetMenuClickHandler {
    wrapper: Rc<RefCell<SystemWrapper>>,
}

impl MenuClickHandler for ResetMenuClickHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        self.wrapper.borrow_mut().reset();
    }
}

fn find_path() -> Option<PathBuf> {
    let aircraft_path = get_current_aircraft_path().join("hints");
    info!("Looking for hints in {aircraft_path:?}");
    if aircraft_path.is_dir() {
        Some(aircraft_path)
    } else {
        warn!("No hints found in {aircraft_path:?}");
        None
    }
}

fn init_xplane(app: Rc<RefCell<Hints>>) -> System {
    let bounds = imgui_support_xplane::get_screen_bounds();
    let horiz_offset = get_offset_from_edge(bounds.width(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    let vert_offset = get_offset_from_edge(bounds.height(), FROM_EDGE_PROPORTION, FROM_EDGE_MIN);
    imgui_support_xplane::init(
        TITLE,
        bounds.width() - horiz_offset - WIDTH,
        vert_offset * 2,
        WIDTH,
        HEIGHT,
        app,
    )
}

#[derive(Debug, Serialize, Deserialize)]
enum Mode {
    Free,
    PopOut,
    VR,
}

impl From<&PositioningMode> for Mode {
    fn from(value: &PositioningMode) -> Self {
        match value {
            PositioningMode::PopOut => Mode::PopOut,
            PositioningMode::VR => Mode::VR,
            _ => Mode::Free,
        }
    }
}

impl From<&Mode> for PositioningMode {
    fn from(value: &Mode) -> Self {
        match value {
            Mode::PopOut => PositioningMode::PopOut,
            Mode::VR => PositioningMode::VR,
            Mode::Free => PositioningMode::Free,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct State {
    mode: Mode,
    position: Rect,
    visible: bool,
}

impl From<&Ref> for State {
    fn from(value: &Ref) -> Self {
        let (positioning_mode, position) = value.current_geometry();
        State {
            mode: Mode::from(positioning_mode),
            position,
            visible: value.visible(),
        }
    }
}

fn get_current_aircraft_id() -> String {
    if let Some(icao) = get_current_aircraft_icao() {
        icao
    } else {
        let mut filename = get_current_aircraft_filename();
        filename.set_extension("");
        filename.to_str().unwrap().to_string()
    }
}

fn get_save_directory() -> Option<PathBuf> {
    let path = get_prefs_path().join("hints");
    match std::fs::create_dir_all(&path) {
        Ok(()) => Some(path),
        Err(e) => {
            error!("Could not create hints save directory: {e:?}");
            None
        }
    }
}

fn get_state_path() -> Option<PathBuf> {
    get_save_directory()
        .map(|save_dir| save_dir.join(format!("{}.toml", get_current_aircraft_id())))
}

fn init_logging(var: &str, with_thread_names: bool) {
    LOGGING.get_or_init(|| configure_logging(var, with_thread_names));
}

fn configure_logging(env_var: &str, with_thread_names: bool) {
    let stdout_layer = layer(with_thread_names, None);
    let xp_layer = layer(with_thread_names, Some(false)).with_writer(|| XplmWrite);

    let filter = env_filter(Some(env_var));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(xp_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
}
