#![warn(clippy::all, rust_2018_idioms)]

mod context;
mod main_window;
pub mod prelude;
mod tabs;

pub use main_window::BetterErcApp;
