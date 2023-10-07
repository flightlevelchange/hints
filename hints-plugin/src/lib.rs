/*
 * Copyright © 2023 David Dunwoody.
 *
 * All rights reserved.
 */

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use hints::{init_xplane, ConfigLocation, Hints, HintsEvent};
use imgui_support::xplane::System;
use xplm::command::{CommandHandler, OwnedCommand};
use xplm::menu::{CheckHandler, CheckItem, Menu};
use xplm::plugin::Plugin;
use xplm_ext::logging;

struct HintPlugin {
    _menu: Menu,
    _next_command: OwnedCommand,
    _previous_command: OwnedCommand,
    _toggle_window_command: OwnedCommand,
}

impl Plugin for HintPlugin {
    type Error = Infallible;

    fn start() -> Result<Self, Self::Error> {
        logging::configure("HINTS_PLUGIN_LOG", true);
        let app = Rc::new(RefCell::new(
            Hints::new(ConfigLocation::RelativeToPlugin).expect("Unable to create Hints app"),
        ));
        let system = Rc::new(RefCell::new(init_xplane(Rc::clone(&app))));
        let menu = Menu::new("Hints").expect("Unable to create hints menu");
        let toggle = Rc::new(
            CheckItem::new(
                "Show hints window",
                false,
                ToggleWindowCheckHandler {
                    system: Rc::clone(&system),
                },
            )
            .expect("Unable to crate show hints window menu item"),
        );
        let toggle_command_handler = ToggleWindowCommandHandler {
            system,
            toggle: Rc::clone(&toggle),
        };
        menu.add_child::<Rc<CheckItem>, CheckItem>(toggle);
        menu.add_to_plugins_menu();
        Ok(HintPlugin {
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
                "flc/hints/toggle_window",
                "Toggle window visibility",
                toggle_command_handler,
            ),
        })
    }

    fn info(&self) -> xplm::plugin::PluginInfo {
        xplm::plugin::PluginInfo {
            name: String::from("FLC Hints"),
            signature: String::from("uk.co.flightlevelchange.hints"),
            description: String::from(
                "Pop-up/pop-out window containing hint images for the current aircraft",
            ),
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
