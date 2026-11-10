//! Mado's core: everything without a window. What each shortcut does,
//! where it puts a window, and the settings are here, as pure code with
//! tests; the app crate reads the Mac and carries out what this decides.
//! The CLI (`mado-core`) runs each part from a terminal.

pub mod action;
pub mod config;
pub mod layout;
pub mod paths;
pub mod process;
pub mod shortcut;
pub mod snap;
pub mod update;
