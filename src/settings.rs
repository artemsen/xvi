// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

pub struct Settings {
    pub colors: ColorScheme,
}
impl Settings {
    pub fn default() -> Self {
        Self {
            colors: ColorScheme::default16(),
        }
    }
}

pub struct ColorScheme {
    pub active_fg: u8,
    pub passive_fg: u8,
    pub modified_fg: u8,
    pub common_bg: u8,
    pub highlight_bg: u8,
    pub statusbar_fg: u8,
    pub statusbar_bg: u8,
    pub keybarid_fg: u8,
    pub keybarid_bg: u8,
    pub keybartitle_fg: u8,
    pub keybartitle_bg: u8,
    pub dialog_fg: u8,
    pub dialog_bg: u8,
    pub shadow_fg: u8,
    pub shadow_bg: u8,
    pub button_fg: u8,
    pub button_disabled_fg: u8,
    pub button_bg: u8,
    pub buttonfocused_fg: u8,
    pub buttonfocused_bg: u8,
    pub edit_fg: u8,
    pub edit_bg: u8,
    pub editfocused_fg: u8,
}

impl ColorScheme {
    const BLACK: u8 = 0;
    //const RED: u8 = 1;
    //const GREEN: u8 = 2;
    const YELLOW: u8 = 3;
    const BLUE: u8 = 4;
    //const MAGENTA: u8 = 5;
    const CYAN: u8 = 6;
    const WHITE: u8 = 7;
    const LIGHT: u8 = 8;

    // 16 colors scheme
    pub fn default16() -> Self {
        Self {
            active_fg: ColorScheme::WHITE,
            passive_fg: ColorScheme::LIGHT + ColorScheme::BLUE,
            modified_fg: ColorScheme::LIGHT + ColorScheme::YELLOW,
            common_bg: ColorScheme::BLUE,
            highlight_bg: ColorScheme::LIGHT + ColorScheme::BLACK,
            statusbar_fg: ColorScheme::BLACK,
            statusbar_bg: ColorScheme::CYAN,
            keybarid_fg: ColorScheme::WHITE,
            keybarid_bg: ColorScheme::BLACK,
            keybartitle_fg: ColorScheme::BLACK,
            keybartitle_bg: ColorScheme::CYAN,
            dialog_fg: ColorScheme::BLACK,
            dialog_bg: ColorScheme::WHITE,
            shadow_fg: ColorScheme::LIGHT + ColorScheme::BLACK,
            shadow_bg: ColorScheme::BLACK,
            button_fg: ColorScheme::BLACK,
            button_disabled_fg: ColorScheme::LIGHT + ColorScheme::BLACK,
            button_bg: ColorScheme::WHITE,
            buttonfocused_fg: ColorScheme::BLACK,
            buttonfocused_bg: ColorScheme::CYAN,
            edit_fg: ColorScheme::LIGHT + ColorScheme::BLACK,
            edit_bg: ColorScheme::CYAN,
            editfocused_fg: ColorScheme::BLACK,
        }
    }
}
