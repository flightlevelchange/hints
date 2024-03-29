/*
 * Copyright (c) 2023 Flight Level Change Ltd.
 *
 * All rights reserved.
 */

use std::cmp::Ordering;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use imgui::{Image, Key, Ui};
use imgui_support::events::{Action, Event};
use imgui_support::App;
use tracing::{info, trace, warn};

use crate::concurrent::thread_loader;
use crate::hints::Hint;
use crate::ConfigError;

pub struct Hints {
    path: PathBuf,
    hints: Arc<Mutex<Vec<Hint>>>,
    current_hint_idx: usize,
}

impl Hints {
    /// # Errors
    ///
    /// Returns an error if the config file cannot be found or parsed.
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        if !path.is_dir() {
            return Err(Box::new(ConfigError::new(format!(
                "{} is not a directory",
                path.display()
            ))));
        }
        let mut hints = Hints {
            path,
            hints: Arc::new(Mutex::new(vec![])),
            current_hint_idx: 0,
        };
        hints.reload();
        Ok(hints)
    }

    pub fn reload(&mut self) {
        info!("Loading hints from {:?}", self.path);
        self.current_hint_idx = 0;
        self.hints.lock().unwrap().clear();
        let thread_hints = Arc::clone(&self.hints);
        let (tx, _) = thread_loader(false, move |image_path: PathBuf| {
            match Hint::new(&image_path) {
                Ok(hint) => match thread_hints.lock() {
                    Ok(mut hints) => hints.push(hint),
                    Err(e) => warn!(error=%e, "Unable to lock hints"),
                },
                Err(e) => warn!("Unable to create hint from {image_path:?}: {e}"),
            };
        });

        let mut files = std::fs::read_dir(&self.path)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();
        files.sort();
        if files.is_empty() {
            warn!("No files found in {:?}", self.path);
        }
        for f in files {
            tx.send(f).unwrap();
        }
        drop(tx);
    }

    fn deallocate_current_texture(&self, hints: &[Hint]) {
        if let Some(current_hint) = hints.get(self.current_hint_idx) {
            current_hint.deallocate_texture();
        }
    }

    pub fn handle_hints_event(&mut self, event: HintsEvent) {
        match event {
            HintsEvent::NextHint => {
                if self.have_hints() {
                    let hints = self.hints.lock().expect("Could not lock hints");
                    self.deallocate_current_texture(&hints);
                    self.current_hint_idx = (self.current_hint_idx + 1) % hints.len();
                    trace!(new_idx = self.current_hint_idx, "HintsEvent::NextHint");
                }
            }
            HintsEvent::PreviousHint => {
                if self.have_hints() {
                    let hints = self.hints.lock().expect("Could not lock hints");
                    self.deallocate_current_texture(&hints);
                    self.current_hint_idx = (self.current_hint_idx + hints.len() - 1) % hints.len();
                    trace!(new_idx = self.current_hint_idx, "HintsEvent::PreviousHint");
                }
            }
            HintsEvent::Reload => {
                self.reload();
                trace!("HintsEvent::Reload");
            }
        }
    }

    fn have_hints(&self) -> bool {
        let hints = self.hints.lock().expect("Could not lock hints");
        if hints.is_empty() {
            warn!("Check log for errors. No hints are loaded");
            false
        } else {
            true
        }
    }
}

impl App for Hints {
    fn draw_ui(&self, ui: &Ui) {
        let hints = self.hints.lock().unwrap();
        if let Some(hint) = hints.get(self.current_hint_idx) {
            let (width, height) = hint.dimensions();
            let scale_factor = get_scale_factor((width, height), ui.content_region_max());
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
    Reload,
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
                        Key::R => Some(Self::Reload),
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
