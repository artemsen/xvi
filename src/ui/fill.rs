// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::Window;
use super::dialog::*;
use super::widget::*;
use std::ops::Range;

/// Dialog for setting "fill" parameters.
pub struct FillDlg {
    // Last used pattern.
    pattern: Vec<u8>,
    // Length of the last used range.
    length: u64,
    // Items of the dialog.
    item_start: ItemId,
    item_end: ItemId,
    item_length: ItemId,
    item_pattern: ItemId,
    item_ok: ItemId,
    item_cancel: ItemId,
}

impl FillDlg {
    /// Show "Fill" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `start` - start address (current offset)
    ///
    /// # Return value
    ///
    /// Range and pattern to fill.
    pub fn show(&mut self, start: u64) -> Option<(Range<u64>, &[u8])> {
        let width = 39;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Fill range",
        );

        // start offset
        dlg.add_next(Text::new("Start offset (hex):"));
        self.item_start = dlg.add(
            Window {
                x: 25,
                y: dlg.last_line - 1,
                width: 16,
                height: 1,
            },
            Edit::new(16, format!("{:x}", start), EditFormat::HexUnsigned),
        );

        // end offset
        dlg.add_next(Text::new("End offset (hex):"));
        self.item_end = dlg.add(
            Window {
                x: 25,
                y: dlg.last_line - 1,
                width: 16,
                height: 1,
            },
            Edit::new(
                16,
                format!("{:x}", start + self.length - 1),
                EditFormat::HexUnsigned,
            ),
        );

        // number of bytes to fill
        dlg.add_next(Text::new("Number of bytes (int):"));
        self.item_length = dlg.add(
            Window {
                x: 25,
                y: dlg.last_line - 1,
                width: 16,
                height: 1,
            },
            Edit::new(16, format!("{}", self.length), EditFormat::DecUnsigned),
        );

        // fill pattern
        dlg.add_separator();
        dlg.add_next(Text::new("Pattern (hex):"));
        let pattern = self.pattern.iter().map(|b| format!("{:02x}", b)).collect();
        self.item_pattern = dlg.add_next(Edit::new(width, pattern, EditFormat::HexStream));

        // buttons
        self.item_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        self.item_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        // run dialog
        if let Some(id) = dlg.run(self) {
            if id != self.item_cancel {
                self.pattern = if let WidgetData::Text(value) = dlg.get(self.item_pattern) {
                    (0..value.len())
                        .step_by(2)
                        .map(|i| {
                            u8::from_str_radix(&value[i..(i + 2).min(value.len())], 16).unwrap()
                        })
                        .collect()
                } else {
                    vec![0]
                };
                let start = self.get_start(&dlg);
                let end = self.get_end(&dlg);
                debug_assert!(start < end);
                self.length = end - start + 1;
                return Some(((start..end), &self.pattern));
            }
        }
        None
    }

    /// Get start offset value.
    fn get_start(&self, dialog: &Dialog) -> u64 {
        if let WidgetData::Text(value) = dialog.get(self.item_start) {
            u64::from_str_radix(&value, 16).unwrap_or(0)
        } else {
            0
        }
    }

    /// Get end offset value.
    fn get_end(&self, dialog: &Dialog) -> u64 {
        if let WidgetData::Text(value) = dialog.get(self.item_end) {
            u64::from_str_radix(&value, 16).unwrap_or(0)
        } else {
            0
        }
    }
}

impl DialogHandler for FillDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.item_cancel || dialog.is_enabled(self.item_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.item_start || item == self.item_end || item == self.item_length {
            let mut length = 0;
            let start = self.get_start(dialog);
            if item == self.item_length {
                if let WidgetData::Text(val) = dialog.get(self.item_length) {
                    length = val.parse::<u64>().unwrap_or(1);
                    let end = format!("{:x}", start + length - 1);
                    dialog.set(self.item_end, WidgetData::Text(end));
                }
            } else {
                let end = self.get_end(dialog);
                if end >= start {
                    length = end - start + 1;
                }
                dialog.set(self.item_length, WidgetData::Text(format!("{}", length)));
            }

            dialog.set_state(self.item_ok, length != 0);
        }
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if let WidgetData::Text(value) = dialog.get(item) {
            if value.is_empty() {
                if item == self.item_start || item == self.item_end || item == self.item_length {
                    dialog.set(item, WidgetData::Text("0".to_string()));
                } else if item == self.item_pattern {
                    dialog.set(item, WidgetData::Text("00".to_string()));
                }
            }
        }
    }
}

impl Default for FillDlg {
    fn default() -> Self {
        Self {
            pattern: vec![0],
            length: 1,
            item_start: -1,
            item_end: -1,
            item_length: -1,
            item_pattern: -1,
            item_ok: -1,
            item_cancel: -1,
        }
    }
}