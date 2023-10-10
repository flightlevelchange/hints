/*
 * Copyright (c) 2023 Flight Level Change Ltd.
 *
 * All rights reserved.
 */

use std::cell::Cell;
use std::path::Path;

use image::{ImageError, RgbaImage};
use imgui::TextureId;
use imgui_support::deallocate_texture;
#[cfg(feature = "standalone")]
use imgui_support_standalone::create_texture;
#[cfg(feature = "xplane")]
use imgui_support_xplane::create_texture;
use tracing::{error, info};

#[cfg(not(any(feature = "standalone", feature = "xplane")))]
compile_error!("At least one of the following features must be enabled: standalone, xplane");

#[derive(Debug)]
pub struct Hint {
    image: RgbaImage,
    texture_id: Cell<Option<TextureId>>,
}

impl Hint {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ImageError> {
        info!(path = %path.as_ref().display(), "Loading hint");
        let image = image::open(path)?.into_rgba8();
        Ok(Hint {
            image,
            texture_id: Cell::new(None),
        })
    }

    pub fn texture_id(&self) -> Option<TextureId> {
        if let Some(texture_id) = self.texture_id.get() {
            Some(texture_id)
        } else {
            let texture_id = match create_texture(&self.image) {
                Ok(texture_id) => Some(texture_id),
                Err(e) => {
                    error!(error = %e, "Unable to create texture");
                    None
                }
            };
            self.texture_id.replace(texture_id);
            texture_id
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    pub fn deallocate_texture(&self) {
        if let Some(texture_id) = self.texture_id.take() {
            deallocate_texture(texture_id);
        }
    }
}

impl Drop for Hint {
    fn drop(&mut self) {
        self.deallocate_texture();
    }
}
