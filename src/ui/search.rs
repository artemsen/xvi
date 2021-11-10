// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{CheckBox, InputFormat, InputLine, StandardButton, WidgetType};

/// "Search sequence" dialog.
pub struct SearchDialog {
    hex: ItemId,
    ascii: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl SearchDialog {
    /// Width of the dialog.
    const WIDTH: usize = 40;
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
        let mut dlg = Dialog::new(SearchDialog::WIDTH, 6, DialogType::Normal, "Search");

        // hex sequence
        dlg.add_line(WidgetType::StaticText(
            "Hex sequence to search:".to_string(),
        ));
        let history: Vec<String> = sequences
            .iter()
            .map(|s| s.iter().map(|b| format!("{:02x}", b)).collect())
            .collect();
        let init = if history.is_empty() {
            String::new()
        } else {
            history[0].clone()
        };
        let widget = InputLine::new(init, InputFormat::HexStream, history, SearchDialog::WIDTH);
        let hex = dlg.add_line(WidgetType::Edit(widget));

        // ascii sequence
        dlg.add_line(WidgetType::StaticText("ASCII:".to_string()));
        let widget = InputLine::new(
            String::new(),
            InputFormat::Any,
            Vec::new(),
            SearchDialog::WIDTH,
        );
        let ascii = dlg.add_line(WidgetType::Edit(widget));

        // search direction
        dlg.add_separator();
        let widget = CheckBox {
            state: backward,
            title: "Backward search".to_string(),
        };
        let bkg = dlg.add_line(WidgetType::CheckBox(widget));

        // buttons
        let btn_ok = dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            hex,
            ascii,
            btn_ok,
            btn_cancel,
        };
        handler.on_item_change(&mut dlg, handler.hex);

        // show dialog
        if let Some(id) = dlg.show(&mut handler) {
            if id != handler.btn_cancel {
                let seq = handler.get_sequence(&dlg).unwrap();
                debug_assert!(!seq.is_empty());
                let dir = if let WidgetType::CheckBox(widget) = dlg.get_widget(bkg) {
                    widget.state
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

        if let WidgetType::Edit(widget) = dialog.get_widget(self.hex) {
            let mut value = widget.get_value().to_string();
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
    fn on_close(&mut self, dialog: &mut Dialog, item: ItemId) -> bool {
        item == self.btn_cancel || dialog.get_context(self.btn_ok).enabled
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
                if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.ascii) {
                    widget.set_value(ascii);
                }
                is_ok = !hex.is_empty();
            }
        } else if item == self.ascii {
            // set hex text from the ASCII field
            if let WidgetType::Edit(widget) = dialog.get_widget(self.ascii) {
                let hex: String = widget
                    .get_value()
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
                if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.hex) {
                    widget.set_value(hex);
                }
            }
        } else {
            is_ok = true;
        }

        dialog.set_enabled(self.btn_ok, is_ok);
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.hex {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.hex) {
                let mut value = widget.get_value().to_string();
                if !value.is_empty() && value.len() % 2 != 0 {
                    value.push('0');
                    widget.set_value(value);
                }
            }
        }
    }
}
