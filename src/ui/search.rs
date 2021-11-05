// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{Button, Checkbox, Edit, EditFormat, StdButton, Text, WidgetData};

/// "Search sequence" dialog.
pub struct SearchDialog {
    hex: ItemId,
    ascii: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl SearchDialog {
    /// Character used for non-printable values in the ASCII field.
    const NPCHAR: char = '·';

    /// Show the "Search" dialog.
    ///
    /// # Arguments
    ///
    /// * `sequences` - sequences history
    /// * `backward` - default search direction
    ///
    /// # Return value
    ///
    /// Search sequence and direction.
    pub fn show(sequences: &[Vec<u8>], backward: bool) -> Option<(Vec<u8>, bool)> {
        // create dialog
        let mut dlg = Dialog::new(40 + Dialog::PADDING_X * 2, 10, DialogType::Normal, "Search");

        // hex sequence
        dlg.add_next(Text::new("Hex sequence to search:"));
        let init = if sequences.is_empty() {
            String::new()
        } else {
            sequences[0].iter().map(|b| format!("{:02x}", b)).collect()
        };
        let mut hex_widget = Edit::new(40, init, EditFormat::HexStream);
        hex_widget.history = sequences
            .iter()
            .map(|s| s.iter().map(|b| format!("{:02x}", b)).collect())
            .collect();
        let hex = dlg.add_next(hex_widget);

        // ascii sequence
        dlg.add_next(Text::new("ASCII:"));
        let ascii = dlg.add_next(Edit::new(40, String::new(), EditFormat::Any));

        // search direction
        dlg.add_separator();
        let bkg_checkbox = dlg.add_next(Checkbox::new("Backward search", backward));

        // buttons
        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        let mut handler = Self {
            hex,
            ascii,
            btn_ok,
            btn_cancel,
        };

        handler.on_item_change(&mut dlg, handler.hex);

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                let seq = handler.get_sequence(&dlg).unwrap();
                debug_assert!(!seq.is_empty());
                let dir = if let WidgetData::Bool(value) = dlg.get(bkg_checkbox) {
                    value
                } else {
                    backward
                };
                return Some((seq, dir));
            }
        }
        None
    }

    /// Get current sequence from the hex field.
    fn get_sequence(&self, dialog: &Dialog) -> Option<Vec<u8>> {
        let mut result = None;
        if let WidgetData::Text(mut value) = dialog.get(self.hex) {
            if !value.is_empty() {
                if value.len() % 2 != 0 {
                    value.push('0');
                }
                result = Some(
                    (0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                        .collect(),
                );
            }
        }
        result
    }
}

impl DialogHandler for SearchDialog {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.btn_cancel || dialog.is_enabled(self.btn_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        let mut is_ok = false;
        if item == self.hex {
            if let Some(hex) = self.get_sequence(dialog) {
                // set ASCII text from the hex field
                let ascii = hex
                    .iter()
                    .map(|c| {
                        if *c > 0x20 && *c < 0x7f {
                            *c as char
                        } else {
                            SearchDialog::NPCHAR
                        }
                    })
                    .collect();
                dialog.set(self.ascii, WidgetData::Text(ascii));
                is_ok = !hex.is_empty();
            }
        } else if item == self.ascii {
            // set hex text from the ASCII field
            if let WidgetData::Text(ascii) = dialog.get(self.ascii) {
                let hex: String = ascii
                    .chars()
                    .map(|b| {
                        format!(
                            "{:02x}",
                            if b == SearchDialog::NPCHAR {
                                0
                            } else {
                                b as u8
                            }
                        )
                    })
                    .collect();
                is_ok = !hex.is_empty();
                dialog.set(self.hex, WidgetData::Text(hex));
            }
        } else {
            is_ok = true;
        }
        dialog.set_state(self.btn_ok, is_ok);
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.hex {
            if let WidgetData::Text(mut value) = dialog.get(self.hex) {
                if !value.is_empty() && value.len() % 2 != 0 {
                    value.push('0');
                    dialog.set(self.hex, WidgetData::Text(value));
                }
            }
        }
    }
}
