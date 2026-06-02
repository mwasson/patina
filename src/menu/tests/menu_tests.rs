use crate::menu::{
    action_for_menu_id, action_for_shortcut, MenuAction, MENU_ID_EXIT, MENU_ID_LOAD_ROM,
};
use muda::MenuId;
use tao::keyboard::Key;

#[test]
fn menu_id_load_rom_maps_to_load_rom_action() {
    let id = MenuId(MENU_ID_LOAD_ROM.to_string());
    assert_eq!(action_for_menu_id(&id), Some(MenuAction::LoadRom));
}

#[test]
fn menu_id_exit_maps_to_exit_action() {
    let id = MenuId(MENU_ID_EXIT.to_string());
    assert_eq!(action_for_menu_id(&id), Some(MenuAction::Exit));
}

#[test]
fn unknown_menu_id_maps_to_no_action() {
    let id = MenuId("something_else".to_string());
    assert_eq!(action_for_menu_id(&id), None);
}

#[test]
fn ctrl_o_maps_to_load_rom() {
    assert_eq!(
        action_for_shortcut(true, &Key::Character("o")),
        Some(MenuAction::LoadRom)
    );
}

#[test]
fn ctrl_q_maps_to_exit() {
    assert_eq!(
        action_for_shortcut(true, &Key::Character("q")),
        Some(MenuAction::Exit)
    );
}

#[test]
fn shortcut_without_ctrl_is_ignored() {
    assert_eq!(action_for_shortcut(false, &Key::Character("o")), None);
    assert_eq!(action_for_shortcut(false, &Key::Character("q")), None);
}

#[test]
fn ctrl_with_unmapped_char_is_ignored() {
    assert_eq!(action_for_shortcut(true, &Key::Character("a")), None);
}

#[test]
fn ctrl_with_non_character_key_is_ignored() {
    assert_eq!(action_for_shortcut(true, &Key::Enter), None);
    assert_eq!(action_for_shortcut(true, &Key::ArrowUp), None);
}
