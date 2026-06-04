//! The application's native menu bar, built with `muda`.
//!
//! Menu definition and the mapping from a triggered item (or keyboard
//! shortcut) to a [`MenuAction`] are fully cross-platform and live here. The
//! only platform-divergent code is how the built [`Menu`] is attached to the
//! window, which lives in `window.rs` (`init_for_gtk_window` on Linux,
//! `init_for_hwnd` on Windows, `init_for_nsapp` on macOS).

use muda::accelerator::{Accelerator, Code, CMD_OR_CTRL};
use muda::{Menu, MenuId, MenuItem, Submenu};
use tao::keyboard::Key;

#[cfg(test)]
mod tests;

pub(crate) const MENU_ID_LOAD_ROM: &str = "load_rom";
pub(crate) const MENU_ID_EXIT: &str = "exit";

/// A user-triggerable application action, however it was triggered (menu item,
/// keyboard shortcut, window close, or signal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    LoadRom,
    Exit,
}

/// Maps a triggered `muda` menu item id to its action. Pure.
pub(crate) fn action_for_menu_id(id: &MenuId) -> Option<MenuAction> {
    match id.0.as_str() {
        MENU_ID_LOAD_ROM => Some(MenuAction::LoadRom),
        MENU_ID_EXIT => Some(MenuAction::Exit),
        _ => None,
    }
}

/// Maps a `Ctrl`+`<key>` keyboard shortcut to its action. Pure.
///
/// `ctrl` is whether the control modifier is held; `key` is the logical key of
/// the press. Non-character keys and unmapped characters yield `None`.
pub(crate) fn action_for_shortcut(ctrl: bool, key: &Key) -> Option<MenuAction> {
    if !ctrl {
        return None;
    }
    match key {
        Key::Character("o") => Some(MenuAction::LoadRom),
        Key::Character("q") => Some(MenuAction::Exit),
        _ => None,
    }
}

/// Builds the application's menu bar: a single `File` menu containing
/// `Load ROM...` (Ctrl/Cmd+O) and `Exit` (Ctrl/Cmd+Q).
///
/// Not unit-tested: it constructs native menu objects (GTK/Win32/AppKit) that
/// require a platform UI context.
pub(crate) fn build_menu() -> Result<Menu, muda::Error> {
    let menu = Menu::new();
    let load_rom = MenuItem::with_id(
        MENU_ID_LOAD_ROM,
        "Load ROM...",
        true,
        Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyO)),
    );
    let exit = MenuItem::with_id(
        MENU_ID_EXIT,
        "Exit",
        true,
        Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyQ)),
    );
    let file_menu = Submenu::with_items("File", true, &[&load_rom, &exit])?;
    menu.append(&file_menu)?;
    Ok(menu)
}
