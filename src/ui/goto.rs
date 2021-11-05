// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::Window;
use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{Button, Edit, EditFormat, StdButton, Text, WidgetData};

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
        let mut dlg = Dialog::new(40 + Dialog::PADDING_X * 2, 9, DialogType::Normal, "Goto");

        dlg.add_next(Text::new("Absolute offset"));

        // absolute offset in dec
        let mut widget = Edit::new(17, format!("{:x}", current), EditFormat::HexUnsigned);
        widget.history = history.iter().map(|o| format!("{:x}", o)).collect();
        dlg.add(
            Window {
                x: Dialog::PADDING_X,
                y: dlg.last_line,
                width: 4,
                height: 1,
            },
            Text::new("hex:"),
        );
        let abs_hex = dlg.add(
            Window {
                x: Dialog::PADDING_X + 4,
                y: dlg.last_line,
                width: 17,
                height: 1,
            },
            widget,
        );

        // absolute offset in dec
        dlg.add(
            Window {
                x: Dialog::PADDING_X + 23,
                y: dlg.last_line,
                width: 4,
                height: 1,
            },
            Text::new("dec:"),
        );
        let abs_dec = dlg.add(
            Window {
                x: Dialog::PADDING_X + 27,
                y: dlg.last_line,
                width: 17,
                height: 1,
            },
            Edit::new(17, String::new(), EditFormat::HexUnsigned),
        );

        dlg.last_line += 1; // skip line

        dlg.add_separator();
        dlg.add_next(Text::new("Relative offset"));

        // relative offset in hex
        dlg.add(
            Window {
                x: Dialog::PADDING_X,
                y: dlg.last_line,
                width: 4,
                height: 1,
            },
            Text::new("hex:"),
        );
        let rel_hex = dlg.add(
            Window {
                x: Dialog::PADDING_X + 4,
                y: dlg.last_line,
                width: 17,
                height: 1,
            },
            Edit::new(17, String::new(), EditFormat::HexSigned),
        );

        // relative offset in dec
        dlg.add(
            Window {
                x: Dialog::PADDING_X + 23,
                y: dlg.last_line,
                width: 4,
                height: 1,
            },
            Text::new("dec:"),
        );
        let rel_dec = dlg.add(
            Window {
                x: Dialog::PADDING_X + 27,
                y: dlg.last_line,
                width: 17,
                height: 1,
            },
            Edit::new(17, String::new(), EditFormat::DecSigned),
        );

        // buttons
        dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        let mut handler = Self {
            current,
            abs_hex,
            abs_dec,
            rel_hex,
            rel_dec,
        };

        handler.on_item_change(&mut dlg, handler.abs_hex);

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != btn_cancel {
                if let WidgetData::Text(value) = dlg.get(handler.abs_hex) {
                    if let Ok(offset) = u64::from_str_radix(&value, 16) {
                        return Some(offset);
                    }
                    return Some(0);
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
        if let WidgetData::Text(value) = dialog.get(source) {
            if source == self.abs_hex {
                offset = u64::from_str_radix(&value, 16).unwrap_or(0);
            } else if source == self.abs_dec {
                offset = value.parse::<u64>().unwrap_or(0);
            } else if source == self.rel_hex || source == self.rel_dec {
                let relative = if source == self.rel_hex {
                    i64::from_str_radix(&value, 16).unwrap_or(0)
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
            dialog.set(self.abs_hex, WidgetData::Text(format!("{:x}", offset)));
        }
        if source != self.abs_dec {
            dialog.set(self.abs_dec, WidgetData::Text(format!("{}", offset)));
        }
        if source != self.rel_hex {
            let offset = offset as i64 - self.current as i64;
            let sign = if offset < 0 { "-" } else { "" };
            let text = format!("{}{:x}", sign, i64::abs(offset));
            dialog.set(self.rel_hex, WidgetData::Text(text));
        }
        if source != self.rel_dec {
            let offset = offset as i64 - self.current as i64;
            dialog.set(self.rel_dec, WidgetData::Text(format!("{}", offset)));
        }
    }
}

impl DialogHandler for GotoDialog {
    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.update(dialog, item);
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if let WidgetData::Text(value) = dialog.get(item) {
            if value.is_empty() {
                dialog.set(item, WidgetData::Text("0".to_string()));
                self.update(dialog, item);
            }
        }
    }
}
