// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// Search properties.
pub struct Search {
    /// Sequence to search.
    pub data: Vec<u8>,
    /// Search direction.
    pub backward: bool,
}

impl Search {
    /// Create new instance.
    pub fn new(default: Vec<u8>) -> Self {
        Self {
            data: default,
            backward: false,
        }
    }

    /// Show "Find" dialog.
    pub fn dialog(&mut self) -> bool {
        let mut init = String::with_capacity(self.data.len() * 2);
        for byte in self.data.iter() {
            init.push_str(&format!("{:02x}", byte));
        }

        let width = 40;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Search",
        );
        dlg.add_next(Text::new("Hex sequence to search:"));
        let hex = dlg.add_next(Edit::new(width, init, EditFormat::HexStream));
        dlg.add_next(Text::new("ASCII:"));
        let ascii = dlg.add_next(Edit::new(width, String::new(), EditFormat::Any));
        dlg.add_separator();
        let backward = dlg.add_next(Checkbox::new("Backward search", self.backward));

        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        dlg.rules
            .push(DialogRule::CopyData(hex, ascii, Box::new(HexToAscii {})));
        dlg.rules
            .push(DialogRule::CopyData(ascii, hex, Box::new(AsciiToHex {})));

        dlg.rules.push(DialogRule::StateChange(
            hex,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));
        dlg.rules.push(DialogRule::StateChange(
            ascii,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));

        dlg.rules
            .push(DialogRule::AllowExit(hex, Box::new(DisableEmpty {})));
        dlg.cancel = btn_cancel;

        dlg.apply(hex);

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Text(value) = dlg.get(hex) {
                    let mut value = value;
                    if value.len() % 2 != 0 {
                        value.push('0');
                    }
                    self.data = (0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                        .collect();
                }
                if let WidgetData::Bool(value) = dlg.get(backward) {
                    self.backward = value;
                }
                return true;
            }
        }
        false
    }

    /// Convert from text to byte array.
    fn get_hex(data: &WidgetData) -> Vec<u8> {
        let mut array = Vec::new();
        if let WidgetData::Text(value) = data {
            for it in (0..value.len()).step_by(2) {
                let hex = &value[it..it + if it + 2 < value.len() { 2 } else { 1 }];
                array.push(u8::from_str_radix(hex, 16).unwrap())
            }
        }
        array
    }
}

/// Dialog rule: set ASCII text from the hex field.
struct HexToAscii;
impl CopyData for HexToAscii {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        let mut ascii = String::new();
        for c in Search::get_hex(data).iter() {
            ascii.push(if *c > 0x20 && *c < 0x7f {
                *c as char
            } else {
                '?'
            });
        }
        Some(WidgetData::Text(ascii))
    }
}

/// Dialog rule: set hex text from the ASCII field.
struct AsciiToHex;
impl CopyData for AsciiToHex {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        if let WidgetData::Text(value) = data {
            Some(WidgetData::Text(
                value.chars().map(|i| format!("{:02x}", i as u8)).collect(),
            ))
        } else {
            None
        }
    }
}
