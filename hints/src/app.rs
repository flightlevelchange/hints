/*
 * Copyright © 2023 David Dunwoody.
 *
 * All rights reserved.
 */

use std::cmp::Ordering;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cfg_if::cfg_if;
use imgui::{Image, Key, Ui};
use imgui_support::App;
use serde::Deserialize;
use tracing::{debug, warn};

use dcommon::ui::events::{Action, Event};
use crate::concurrent::thread_loader;

use crate::hints::Hint;

pub struct Hints {
    hints: Arc<Mutex<Vec<Hint>>>,
    current_hint_idx: usize,
}

#[derive(Default, Deserialize)]
struct Config {
    images: Vec<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum ConfigLocation {
    FromArgs,
    RelativeToPlugin,
}

impl Hints {
    /// # Errors
    ///
    /// Returns an error if the config file cannot be found or parsed.
    pub fn new(location: ConfigLocation) -> Result<Self, Box<dyn Error>> {
        let path = get_path(location);
        let config = load_config(&path)?;
        let hints: Arc<Mutex<Vec<Hint>>> = Arc::new(Mutex::new(vec![]));
        let thread_hints = Arc::clone(&hints);
        let (tx, _) = thread_loader(false, move |image_path: String| {
            let p = if let Some(p) = path.parent() {
                p
            } else {
                warn!(path = %path.display(), "Unable to get parent");
                &path
            };
            let p = p.join(image_path);

            let p = match p.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!(error=%e, path=%p.display(), "Unable to canonicalize path");
                    p
                }
            };

            match Hint::new(p) {
                Ok(hint) => match thread_hints.lock() {
                    Ok(mut hints) => hints.push(hint),
                    Err(e) => warn!(error=%e, "Unable to lock hints"),
                },
                Err(e) => warn!(error=%e, "Unable to create hint"),
            };
        });

        for image_path in config.images {
            tx.send(image_path)?;
        }
        drop(tx);

        Ok(Hints {
            hints,
            current_hint_idx: 0,
        })
    }

    fn deallocate_current_texture(&self, hints: &[Hint]) {
        if let Some(current_hint) = hints.get(self.current_hint_idx) {
            current_hint.deallocate_texture();
        }
    }

    pub fn handle_hints_event(&mut self, event: HintsEvent) {
        let hints = self.hints.lock().expect("Could not lock hints");
        if hints.is_empty() {
            warn!("Check log for errors. No hints were loaded.");
            return;
        }
        match event {
            HintsEvent::NextHint => {
                self.deallocate_current_texture(&hints);
                self.current_hint_idx = (self.current_hint_idx + 1) % hints.len();
                debug!(new_idx = self.current_hint_idx, "next_hint()");
            }
            HintsEvent::PreviousHint => {
                self.deallocate_current_texture(&hints);
                self.current_hint_idx = (self.current_hint_idx + hints.len() - 1) % hints.len();
                debug!(new_idx = self.current_hint_idx, "previous_hint()");
            }
        }
    }
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
    debug!(path=%path.as_ref().display(), "Reading configuration");
    match fs::read_to_string(path) {
        Ok(contents) => match toml::from_str::<Config>(&contents) {
            Ok(config) => Ok(config),
            Err(e) => Err(Box::new(e)),
        },
        Err(e) => Err(Box::new(e)),
    }
}

fn get_path(location: ConfigLocation) -> PathBuf {
    match location {
        ConfigLocation::FromArgs => {
            cfg_if! {
                if #[cfg(feature = "standalone")] {
                    use std::env;
                    let args: Vec<String> = env::args().collect();
                    assert!(args.len() == 2, "Expected exactly one argument: path to config file");
                    PathBuf::from(&args[1])
                } else {
                    panic!("ConfigLocation::FromArgs is only supported in standalone mode")
                }
            }
        }
        ConfigLocation::RelativeToPlugin => {
            cfg_if! {
                if #[cfg(feature = "xplane")] {
                    use xplm_ext::plugin::utils::get_plugin_path;
                    get_plugin_path()
                        .parent()
                        .unwrap()
                        .join("../hints/config.toml")
                } else {
                    panic!("ConfigLocation::RelativeToPlugin is only supported in xplane mode")
                }
            }
        }
    }
}

impl App for Hints {
    fn draw_ui(&self, ui: &Ui) {
        let hints = self.hints.lock().unwrap();
        if let Some(hint) = hints.get(self.current_hint_idx) {
            let (width, height) = hint.dimensions();
            let scale_factor = get_scale_factor((width, height), ui.window_size());
            if let Some(texture_id) = hint.texture_id() {
                #[allow(clippy::cast_precision_loss)]
                {
                    Image::new(
                        texture_id,
                        [width as f32 * scale_factor, height as f32 * scale_factor],
                    )
                    .build(ui);
                }
            }
        }
    }

    fn handle_event(&mut self, event: Event) -> bool {
        if let Some(event) = HintsEvent::from(&event) {
            self.handle_hints_event(event);
            true
        } else {
            false
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn get_scale_factor(image_size: (u32, u32), window_size: [f32; 2]) -> f32 {
    let (width, height) = image_size;
    let width_scale = window_size[0] / width as f32;
    let height_scale = window_size[1] / height as f32;
    width_scale.min(height_scale)
}

#[derive(Debug, Clone, Copy)]
pub enum HintsEvent {
    NextHint,
    PreviousHint,
}

impl HintsEvent {
    fn from(event: &Event) -> Option<Self> {
        match *event {
            Event::Scroll(_, y) => match y.cmp(&0) {
                Ordering::Less => Some(Self::PreviousHint),
                Ordering::Equal => None,
                Ordering::Greater => Some(Self::NextHint),
            },
            Event::Key(Some(key), _, action, _) => {
                if action == Action::Press {
                    match key {
                        Key::UpArrow => Some(Self::PreviousHint),
                        Key::DownArrow => Some(Self::NextHint),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
