// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// Dialog for configuring search parameters.
pub struct SearchDlg {
    /// Search history.
    pub history: Vec<Vec<u8>>,
    /// Search direction.
    pub backward: bool,
}

impl SearchDlg {
    /// Show configuration dialog.
    pub fn show(&mut self) -> bool {
        let width = 40;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Search",
        );

        dlg.add_next(Text::new("Hex sequence to search:"));
        let init = if let Some(seq) = self.get_sequence() {
            seq.iter().map(|b| format!("{:02x}", b)).collect()
        } else {
            String::new()
        };
        let hex_history = self
            .history
            .iter()
            .map(|s| s.iter().map(|b| format!("{:02x}", b)).collect())
            .collect();
        let mut hex_widget = Edit::new(width, init, EditFormat::HexStream);
        hex_widget.history = hex_history;
        let hex = dlg.add_next(hex_widget);

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

        dlg.apply(hex);

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                let seq = SearchDlg::get_hex(&dlg.get(hex));
                self.history.retain(|s| s != &seq);
                self.history.insert(0, seq);
                if let WidgetData::Bool(value) = dlg.get(backward) {
                    self.backward = value;
                }
                return true;
            }
        }
        false
    }

    /// Get current search sequence.
    pub fn get_sequence(&self) -> Option<Vec<u8>> {
        if !self.history.is_empty() {
            Some(self.history[0].clone())
        } else {
            None
        }
    }

    /// Convert from text to byte array.
    fn get_hex(data: &WidgetData) -> Vec<u8> {
        if let WidgetData::Text(value) = data {
            (0..value.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&value[i..(i + 2).min(value.len())], 16).unwrap())
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Dialog rule: set ASCII text from the hex field.
struct HexToAscii;
impl CopyData for HexToAscii {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        let seq = SearchDlg::get_hex(data);
        let ascii = seq
            .iter()
            .map(|c| {
                if *c > 0x20 && *c < 0x7f {
                    *c as char
                } else {
                    '?'
                }
            })
            .collect();
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
