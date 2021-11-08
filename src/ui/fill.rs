// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::range::RangeControl;
use super::widget::{InputFormat, InputLine, StandardButton, WidgetType};
use std::ops::Range;

/// "Fill fange" dialog.
pub struct FillDialog {
    rctl: RangeControl,
    pattern: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl FillDialog {
    /// Show the "Fill range" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `offset` - defualt start offset (current position)
    /// * `max` - max offset (file size)
    /// * `pattern` - default pattern
    ///
    /// # Return value
    ///
    /// Range and pattern to fill.
    pub fn show(offset: u64, max: u64, pattern: &[u8]) -> Option<(Range<u64>, Vec<u8>)> {
        // create dialog
        let mut dlg = Dialog::new(
            RangeControl::DIALOG_WIDTH,
            6,
            DialogType::Normal,
            "Fill range",
        );

        // place range control on dialog
        let rctl = RangeControl::create(&mut dlg, offset..offset + 1, max);

        // pattern
        dlg.add_separator();
        dlg.add_line(WidgetType::StaticText("Fill with pattern:".to_string()));
        let text = pattern.iter().map(|b| format!("{:02x}", b)).collect();
        let widget = InputLine::new(
            text,
            InputFormat::HexStream,
            Vec::new(),
            RangeControl::DIALOG_WIDTH,
        );
        let pattern = dlg.add_line(WidgetType::Edit(widget));

        // buttons
        let btn_ok = dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            rctl,
            pattern,
            btn_ok,
            btn_cancel,
        };

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                let range = handler.rctl.get(&dlg).unwrap();
                let pattern = handler.get_pattern(&dlg);
                return Some((range, pattern));
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

impl DialogHandler for FillDialog {
    fn on_close(&mut self, dialog: &mut Dialog, item: ItemId) -> bool {
        item == self.btn_cancel || dialog.get_context(self.btn_ok).enabled
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_item_change(dialog, item);
        dialog.set_enabled(self.btn_ok, self.rctl.get(dialog).is_some());
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.pattern {
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.pattern) {
                let mut value = widget.get_value().to_string();
                if value.is_empty() {
                    widget.set_value("00".to_string());
                } else if value.len() % 2 != 0 {
                    value.push('0');
                    widget.set_value(value);
                }
            }
        } else {
            self.rctl.on_focus_lost(dialog, item);
        }
    }
}
