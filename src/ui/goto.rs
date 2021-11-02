// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::Window;
use super::dialog::*;
use super::widget::*;

/// Dialog for setting "goto" parameters.
pub struct GotoDlg {
    /// Address history.
    pub history: Vec<u64>,
    // Current offet (cursor position)
    current: u64,
    // Items of the dialog.
    item_abshex: ItemId,
    item_absdec: ItemId,
    item_relhex: ItemId,
    item_reldec: ItemId,
    item_cancel: ItemId,
}

impl GotoDlg {
    /// Show "goto" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `current` - current offset
    ///
    /// # Return value
    ///
    /// Absolute offset to jump.
    pub fn show(&mut self, current: u64) -> Option<u64> {
        self.current = current;

        let width = 44;
        let mut dlg = Dialog::new(width + Dialog::PADDING_X * 2, 9, DialogType::Normal, "Goto");

        dlg.add_next(Text::new("Absolute offset"));

        // absolute offset in dec
        let mut widget = Edit::new(17, format!("{:x}", self.current), EditFormat::HexUnsigned);
        widget.history = self.history.iter().map(|o| format!("{:x}", o)).collect();
        dlg.add(
            Window {
                x: Dialog::PADDING_X,
                y: dlg.last_line,
                width,
                height: 1,
            },
            Text::new("hex:"),
        );
        self.item_abshex = dlg.add(
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
                width,
                height: 1,
            },
            Text::new("dec:"),
        );
        self.item_absdec = dlg.add(
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
                width,
                height: 1,
            },
            Text::new("hex:"),
        );
        self.item_relhex = dlg.add(
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
                width,
                height: 1,
            },
            Text::new("dec:"),
        );
        self.item_reldec = dlg.add(
            Window {
                x: Dialog::PADDING_X + 27,
                y: dlg.last_line,
                width: 17,
                height: 1,
            },
            Edit::new(17, String::new(), EditFormat::DecSigned),
        );

        dlg.add_button(Button::std(StdButton::Ok, true));
        self.item_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        self.on_item_change(&mut dlg, self.item_abshex);

        // run dialog
        if let Some(id) = dlg.run(self) {
            if id != self.item_cancel {
                if let WidgetData::Text(value) = dlg.get(self.item_abshex) {
                    return match u64::from_str_radix(&value, 16) {
                        Ok(offset) => {
                            self.history.retain(|o| o != &offset);
                            self.history.insert(0, offset);
                            Some(offset)
                        }
                        Err(_) => Some(0),
                    };
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
            if source == self.item_abshex {
                offset = u64::from_str_radix(&value, 16).unwrap_or(0);
            } else if source == self.item_absdec {
                offset = value.parse::<u64>().unwrap_or(0);
            } else if source == self.item_relhex {
                let relative = i64::from_str_radix(&value, 16).unwrap_or(0);
                if relative >= 0 || -relative < self.current as i64 {
                    offset = (self.current as i64 + relative) as u64;
                }
            } else if source == self.item_reldec {
                let relative = value.parse::<i64>().unwrap_or(0);
                if relative >= 0 || -relative < self.current as i64 {
                    offset = (self.current as i64 + relative) as u64;
                }
            } else {
                unreachable!();
            }
        }
        // update other fields
        if source != self.item_abshex {
            dialog.set(self.item_abshex, WidgetData::Text(format!("{:x}", offset)));
        }
        if source != self.item_absdec {
            dialog.set(self.item_absdec, WidgetData::Text(format!("{}", offset)));
        }
        if source != self.item_relhex {
            let offset = offset as i64 - self.current as i64;
            let sign = if offset < 0 { "-" } else { "" };
            let text = format!("{}{:x}", sign, i64::abs(offset));
            dialog.set(self.item_relhex, WidgetData::Text(text));
        }
        if source != self.item_reldec {
            let offset = offset as i64 - self.current as i64;
            dialog.set(self.item_reldec, WidgetData::Text(format!("{}", offset)));
        }
    }
}

impl DialogHandler for GotoDlg {
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

impl Default for GotoDlg {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            current: 0,
            item_abshex: -1,
            item_absdec: -1,
            item_relhex: -1,
            item_reldec: -1,
            item_cancel: -1,
        }
    }
}
