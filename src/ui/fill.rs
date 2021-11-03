// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::range::RangeControl;
use super::widget::*;
use std::ops::Range;

/// Dialog for setting "fill" parameters.
pub struct FillDlg {
    rctl: RangeControl,
    pattern: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl FillDlg {
    /// Show "Fill" configuration dialog.
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
            RangeControl::DIALOG_WIDTH + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Fill range",
        );

        // place range control on dialog
        let rctl = RangeControl::create(&mut dlg, offset..offset + 1, max);

        // pattern
        dlg.add_separator();
        dlg.add_next(Text::new("Fill pattern:"));
        let text = pattern.iter().map(|b| format!("{:02x}", b)).collect();
        let pattern = dlg.add_next(Edit::new(
            RangeControl::DIALOG_WIDTH,
            text,
            EditFormat::HexStream,
        ));

        // buttons
        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

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
                let pattern = if let WidgetData::Text(val) = dlg.get(handler.pattern) {
                    (0..val.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&val[i..(i + 2).min(val.len())], 16).unwrap())
                        .collect()
                } else {
                    vec![0]
                };
                return Some((range, pattern));
            }
        }
        None
    }
}

impl DialogHandler for FillDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.btn_cancel || dialog.is_enabled(self.btn_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_item_change(dialog, item);
        dialog.set_state(self.btn_ok, self.rctl.get(dialog).is_some());
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.pattern {
            if let WidgetData::Text(mut value) = dialog.get(self.pattern) {
                if value.is_empty() {
                    dialog.set(self.pattern, WidgetData::Text("00".to_string()));
                } else if value.len() % 2 != 0 {
                    value.push('0');
                    dialog.set(self.pattern, WidgetData::Text(value));
                }
            }
        } else {
            self.rctl.on_focus_lost(dialog, item);
        }
    }
}
