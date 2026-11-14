#![warn(clippy::all, rust_2018_idioms)]

mod context;
mod main_window;
pub mod prelude;
pub mod settings;
mod tabs;
mod windows;

pub use main_window::BetterErcApp;

/// The app's id: the Wayland app id (the window's class), the launcher's `StartupWMClass` and icon name
/// (`deploy/better_erc.desktop`, installed by `just install`)
pub const APP_ID: &str = "better_erc";

/// The name eframe kept the saved state under before the app had an id (`~/.local/share/bettererc`), still used so
/// the saved layout stays
pub const STORAGE_NAME: &str = "Better ERC";

#[cfg(test)]
mod desktop_tests {
    /// The launcher names the app as its window does, so the dock groups them and the icon shows
    #[test]
    fn the_desktop_file_matches_the_app_id() {
        let entry = include_str!("../deploy/better_erc.desktop");
        for key in ["StartupWMClass", "Icon", "Exec"] {
            assert!(
                entry
                    .lines()
                    .any(|l| l == format!("{key}={}", super::APP_ID)),
                "{key} is not {}",
                super::APP_ID
            );
        }
    }
}
