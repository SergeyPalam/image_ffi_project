use thiserror::Error;
use image::ImageError;
use std::ffi::NulError;
use std::io;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("File not found")]
    ConversionError(#[from] ImageError),
    #[error("Load library error")]
    LoadLibrary(#[from] libloading::Error),
    #[error("Out of range")]
    OutOfRange,
    #[error("Null error")]
    Null(#[from] NulError),
    #[error("IO error")]
    IO(#[from] io::Error),
    #[error("Unknown error")]
    Unknown,
}
