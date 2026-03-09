
use libloading::{Library, Symbol};

use std::ffi::{c_ulong, c_char, c_uchar};
use std::path::Path;
use super::error::PluginError;

#[repr(C)]
pub struct PluginInterface<'a> {
    pub process_image: Symbol<'a, extern "C" fn(width: c_ulong, height: c_ulong, rgba_data: *mut c_uchar, params: *const c_char)>,
}

pub struct Plugin {
    plugin: Library,
}

impl Plugin {
    pub fn new(filename: &Path) -> Result<Self, PluginError> {
        Ok(Plugin {
            plugin: unsafe { Library::new(filename) }?,
        })
    }
    pub fn interface(&self) -> Result<PluginInterface<'_>, PluginError> {
        Ok(PluginInterface {
            process_image: unsafe { self.plugin.get("process_image") }?,
        })
    }
} 