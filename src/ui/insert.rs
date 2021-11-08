// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{InputFormat, InputLine, StandardButton, WidgetType};

/// "Insert bytes" dialog.
pub struct InsertDialog {
    length: ItemId,
    offset: ItemId,
    pattern: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl InsertDialog {
    /// Width of the dialog.
    const WIDTH: usize = 42;

    /// Show the "Insert bytes" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `offset` - defualt start offset (current position)
    /// * `pattern` - default pattern
    ///
    /// # Return value
    ///
    /// Start offset, number of bytes to insert and pattern to fill.
    pub fn show(offset: u64, pattern: &[u8]) -> Option<(u64, u64, Vec<u8>)> {
        // create dialog
        let mut dlg = Dialog::new(InsertDialog::WIDTH, 7, DialogType::Normal, "Insert bytes");

        dlg.add_line(WidgetType::StaticText(
            "Insert        bytes at offset".to_string(),
        ));

        // length
        let widget = InputLine::new("1".to_string(), InputFormat::DecUnsigned, Vec::new(), 6);
        let length = dlg.add(
            Dialog::PADDING_X + 7,
            Dialog::PADDING_Y,
            6,
            WidgetType::Edit(widget),
        );

        // start offset
        let widget = InputLine::new(
            format!("{:x}", offset),
            InputFormat::HexUnsigned,
            Vec::new(),
            12,
        );
        let offset = dlg.add(
            Dialog::PADDING_X + 30,
            Dialog::PADDING_Y,
            12,
            WidgetType::Edit(widget),
        );

        // pattern
        dlg.add_separator();
        dlg.add_line(WidgetType::StaticText("Fill with pattern:".to_string()));
        let text = pattern.iter().map(|b| format!("{:02x}", b)).collect();
        let widget = InputLine::new(
            text,
            InputFormat::HexStream,
            Vec::new(),
            InsertDialog::WIDTH,
        );
        let pattern = dlg.add_line(WidgetType::Edit(widget));

        // warning message
        dlg.add_separator();
        dlg.add_center("WARNING!".to_string());
        dlg.add_center("This operation cannot be undone!".to_string());

        // buttons
        let btn_ok = dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            length,
            offset,
            pattern,
            btn_ok,
            btn_cancel,
        };

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                let length = if let WidgetType::Edit(widget) = dlg.get_widget(handler.length) {
                    widget.get_value().parse::<u64>().unwrap_or(0)
                } else {
                    0
                };
                debug_assert_ne!(length, 0);
                let offset = if let WidgetType::Edit(widget) = dlg.get_widget(handler.offset) {
                    u64::from_str_radix(widget.get_value(), 16).unwrap_or(0)
                } else {
                    0
                };
                let pattern = handler.get_pattern(&dlg);
                return Some((offset, length, pattern));
            }
        }
        None
    }

    /// Get current sequence from the pattern field.
    fn get_pattern(&self, dialog: &Dialog) -> Vec<u8> {
        if let WidgetType::Edit(widget) = dialog.get_widget(self.pattern) {
            let mut value = widget.get_value().to_string();
            if !value.is_empty() {
                if value.len() % 2 != 0 {
                    value.push('0');
                }
                return (0..value.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                    .collect();
            }
        }
        vec![0]
    }
}

impl DialogHandler for InsertDialog {
    fn on_close(&mut self, dialog: &mut Dialog, item: ItemId) -> bool {
        item == self.btn_cancel || dialog.get_context(self.btn_ok).enabled
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.length {
            if let WidgetType::Edit(widget) = dialog.get_widget(self.length) {
                let is_valid = widget.get_value().parse::<u64>().unwrap_or(0) != 0;
                dialog.set_enabled(self.btn_ok, is_valid);
            }
        }
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.length || item == self.offset {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(item) {
                if widget.get_value().is_empty() {
                    widget.set_value("0".to_string());
                }
            }
        } else if item == self.pattern {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.pattern) {
                let mut value = widget.get_value().to_string();
                if value.is_empty() {
                    widget.set_value("00".to_string());
                } else if value.len() % 2 != 0 {
                    value.push('0');
                    widget.set_value(value);
                }
            }
        }
    }
}
