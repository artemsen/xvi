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
    // Items of the dialog.
    item_hex: ItemId,
    item_ascii: ItemId,
    item_ok: ItemId,
    item_cancel: ItemId,
}

impl SearchDlg {
    /// Character used for non-printable values in the ASCII field.
    const NPCHAR: char = '·';

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
        self.item_hex = dlg.add_next(hex_widget);

        dlg.add_next(Text::new("ASCII:"));
        self.item_ascii = dlg.add_next(Edit::new(width, String::new(), EditFormat::Any));

        dlg.add_separator();
        let backward = dlg.add_next(Checkbox::new("Backward search", self.backward));

        self.item_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        self.item_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        self.on_item_change(&mut dlg, self.item_hex);

        // run dialog
        if let Some(id) = dlg.run(self) {
            if id != self.item_cancel {
                let seq = self.get_hex(&dlg).unwrap();
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

    /// Get current sequence from the hex field.
    fn get_hex(&self, dialog: &Dialog) -> Option<Vec<u8>> {
        if let WidgetData::Text(value) = dialog.get(self.item_hex) {
            Some(
                (0..value.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&value[i..(i + 2).min(value.len())], 16).unwrap())
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl DialogHandler for SearchDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.item_cancel || dialog.is_enabled(self.item_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        let mut is_ok = false;
        if item == self.item_hex {
            if let Some(hex) = self.get_hex(dialog) {
                // set ASCII text from the hex field
                let ascii = hex
                    .iter()
                    .map(|c| {
                        if *c > 0x20 && *c < 0x7f {
                            *c as char
                        } else {
                            SearchDlg::NPCHAR
                        }
                    })
                    .collect();
                dialog.set(self.item_ascii, WidgetData::Text(ascii));
                is_ok = !hex.is_empty();
            }
        } else if item == self.item_ascii {
            // set hex text from the ASCII field
            if let WidgetData::Text(ascii) = dialog.get(self.item_ascii) {
                let hex: String = ascii
                    .chars()
                    .map(|b| format!("{:02x}", if b == SearchDlg::NPCHAR { 0 } else { b as u8 }))
                    .collect();
                is_ok = !hex.is_empty();
                dialog.set(self.item_hex, WidgetData::Text(hex));
            }
        } else {
            is_ok = true;
        }
        dialog.set_state(self.item_ok, is_ok);
    }
}

impl Default for SearchDlg {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            backward: false,
            item_hex: -1,
            item_ascii: -1,
            item_ok: -1,
            item_cancel: -1,
        }
    }
}
