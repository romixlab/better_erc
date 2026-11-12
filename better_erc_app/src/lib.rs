#![warn(clippy::all, rust_2018_idioms)]

mod context;
mod main_window;
pub mod prelude;
pub mod settings;
mod tabs;
mod windows;

pub use main_window::BetterErcApp;
