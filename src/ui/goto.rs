// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{InputFormat, InputLine, StandardButton, WidgetType};

/// "Goto" dialog.
pub struct GotoDialog {
    // Current offet (cursor position)
    current: u64,
    // Items of the dialog.
    abs_hex: ItemId,
    abs_dec: ItemId,
    rel_hex: ItemId,
    rel_dec: ItemId,
}

impl GotoDialog {
    /// Width of the input fields.
    const INP_WIDTH: usize = 17;

    /// Show the "Goto" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `history` - address history
    /// * `current` - current offset
    ///
    /// # Return value
    ///
    /// Absolute offset to jump.
    pub fn show(history: &[u64], current: u64) -> Option<u64> {
        // create dialog
        let mut dlg = Dialog::new(44, 6, DialogType::Normal, "Goto");

        dlg.add_line(WidgetType::StaticText("Absolute offset".to_string()));

        // absolute offset in dec
        dlg.add_line(WidgetType::StaticText("hex:".to_string()));
        let history: Vec<String> = history.iter().map(|o| format!("{:x}", o)).collect();
        let init = if history.is_empty() {
            String::new()
        } else {
            history[0].clone()
        };
        let widget = InputLine::new(
            init,
            InputFormat::HexUnsigned,
            history,
            GotoDialog::INP_WIDTH,
        );
        let abs_hex = dlg.add(
            Dialog::PADDING_X + 4,
            Dialog::PADDING_Y + 1,
            GotoDialog::INP_WIDTH,
            WidgetType::Edit(widget),
        );

        // absolute offset in dec
        dlg.add(
            Dialog::PADDING_X + 23,
            Dialog::PADDING_Y + 1,
            4,
            WidgetType::StaticText("dec:".to_string()),
        );
        let widget = InputLine::new(
            String::new(),
            InputFormat::DecUnsigned,
            Vec::new(),
            GotoDialog::INP_WIDTH,
        );
        let abs_dec = dlg.add(
            Dialog::PADDING_X + 27,
            Dialog::PADDING_Y + 1,
            GotoDialog::INP_WIDTH,
            WidgetType::Edit(widget),
        );

        dlg.add_separator();
        dlg.add_line(WidgetType::StaticText("Relative offset".to_string()));

        // relative offset in hex
        dlg.add_line(WidgetType::StaticText("hex:".to_string()));
        let widget = InputLine::new(
            String::new(),
            InputFormat::HexSigned,
            Vec::new(),
            GotoDialog::INP_WIDTH,
        );
        let rel_hex = dlg.add(
            Dialog::PADDING_X + 4,
            Dialog::PADDING_Y + 4,
            GotoDialog::INP_WIDTH,
            WidgetType::Edit(widget),
        );

        // relative offset in dec
        dlg.add(
            Dialog::PADDING_X + 23,
            Dialog::PADDING_Y + 4,
            4,
            WidgetType::StaticText("dec:".to_string()),
        );
        let widget = InputLine::new(
            String::new(),
            InputFormat::DecSigned,
            Vec::new(),
            GotoDialog::INP_WIDTH,
        );
        let rel_dec = dlg.add(
            Dialog::PADDING_X + 27,
            Dialog::PADDING_Y + 4,
            GotoDialog::INP_WIDTH,
            WidgetType::Edit(widget),
        );

        // buttons
        dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            current,
            abs_hex,
            abs_dec,
            rel_hex,
            rel_dec,
        };
        handler.on_item_change(&mut dlg, handler.abs_hex);

        // show dialog
        if let Some(id) = dlg.show(&mut handler) {
            if id != btn_cancel {
                if let WidgetType::Edit(widget) = dlg.get_widget(handler.abs_hex) {
                    return Some(u64::from_str_radix(widget.get_value(), 16).unwrap_or(0));
                }
            }
        }
        None
    }

    /// Update fields.
    ///
    /// # Arguments
    ///
    /// * `dialog` - dialog instance
    /// * `source` - id of item that is the source of the changes
    fn update(&mut self, dialog: &mut Dialog, source: ItemId) {
        // calculate offset
        let mut offset = 0;
        if let WidgetType::Edit(widget) = dialog.get_widget(source) {
            let value = widget.get_value();
            if source == self.abs_hex {
                offset = u64::from_str_radix(value, 16).unwrap_or(0);
            } else if source == self.abs_dec {
                offset = value.parse::<u64>().unwrap_or(0);
            } else if source == self.rel_hex || source == self.rel_dec {
                let relative = if source == self.rel_hex {
                    i64::from_str_radix(value, 16).unwrap_or(0)
                } else {
                    value.parse::<i64>().unwrap_or(0)
                };
                if relative >= 0 || relative.unsigned_abs() < self.current {
                    offset = self.current;
                    if relative >= 0 {
                        offset += relative.unsigned_abs();
                    } else {
                        offset -= relative.unsigned_abs();
                    }
                }
            } else {
                unreachable!();
            }
        }
        // update other fields
        if source != self.abs_hex {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.abs_hex) {
                widget.set_value(format!("{:x}", offset));
            }
        }
        if source != self.abs_dec {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.abs_dec) {
                widget.set_value(format!("{}", offset));
            }
        }
        if source != self.rel_hex {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.rel_hex) {
                let offset = offset as i64 - self.current as i64;
                let sign = if offset < 0 { "-" } else { "" };
                let text = format!("{}{:x}", sign, i64::abs(offset));
                widget.set_value(text);
            }
        }
        if source != self.rel_dec {
            let offset = offset as i64 - self.current as i64;
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.rel_dec) {
                widget.set_value(format!("{}", offset));
            }
        }
    }
}

impl DialogHandler for GotoDialog {
    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.update(dialog, item);
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if let WidgetType::Edit(widget) = dialog.get_widget_mut(item) {
            if widget.get_value().is_empty() {
                widget.set_value("0".to_string());
                self.update(dialog, item);
            }
        }
    }
}
