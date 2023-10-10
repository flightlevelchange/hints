/*
 * Copyright (c) 2023 Flight Level Change Ltd.
 *
 * All rights reserved.
 */

use std::ffi::{c_char, CStr, CString};
use std::io::Write;
use std::path::PathBuf;

use xplm::data::borrowed::DataRef;
use xplm::data::StringRead;
use xplm_sys::{
    XPLMDebugString, XPLMExtractFileAndPath, XPLMGetNthAircraftModel, XPLMGetPrefsPath,
};

#[must_use]
pub fn get_prefs_path() -> PathBuf {
    PathBuf::from(read_to_buffer(|buffer| unsafe {
        XPLMGetPrefsPath(buffer);
        XPLMExtractFileAndPath(buffer);
    }))
}

#[must_use]
pub fn get_current_aircraft_path() -> PathBuf {
    PathBuf::from(read_to_buffer(|path| {
        read_to_buffer(|filename| unsafe {
            XPLMGetNthAircraftModel(0, filename, path);
            XPLMExtractFileAndPath(path);
        });
    }))
}

#[must_use]
pub fn get_current_aircraft_filename() -> PathBuf {
    PathBuf::from(read_to_buffer(|filename| {
        read_to_buffer(|path| unsafe {
            XPLMGetNthAircraftModel(0, filename, path);
        });
    }))
}

#[must_use]
pub fn get_current_aircraft_icao() -> Option<String> {
    let icao = DataRef::find("sim/aircraft/view/acf_ICAO")
        .expect("Could not find ICAO dataref")
        .get_as_string()
        .expect("Could not convert to UTF8");
    if icao.is_empty() {
        None
    } else {
        Some(icao)
    }
}

// from xplm
fn read_to_buffer<F: Fn(*mut c_char)>(read_callback: F) -> String {
    let mut buffer = [0 as c_char; 512];
    read_callback(buffer.as_mut_ptr());
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    cstr.to_string_lossy().into_owned()
}

pub struct XplmWrite;

impl Write for XplmWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match CString::new(buf) {
            Ok(c_str) => unsafe {
                XPLMDebugString(c_str.as_ptr());
            },
            Err(_) => unsafe {
                XPLMDebugString("Invalid message\n\0".as_ptr().cast());
            },
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
