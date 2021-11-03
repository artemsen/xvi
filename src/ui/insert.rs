// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::Window;
use super::dialog::*;
use super::widget::*;

/// Dialog for setting "insert" parameters.
pub struct InsertDlg {
    length: ItemId,
    offset: ItemId,
    pattern: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl InsertDlg {
    /// Show "Insert bytes" configuration dialog.
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
        let mut dlg = Dialog::new(46, 10, DialogType::Normal, "Insert bytes");

        dlg.add_next(Text::new("Insert        bytes at offset"));

        // length
        let wnd = Window {
            x: 9,
            y: dlg.last_line - 1,
            width: 6,
            height: 1,
        };
        let widget = Edit::new(wnd.width, "1".to_string(), EditFormat::DecUnsigned);
        let length = dlg.add(wnd, widget);

        // start offset
        let wnd = Window {
            x: 32,
            y: dlg.last_line - 1,
            width: 12,
            height: 1,
        };
        let widget = Edit::new(wnd.width, format!("{:x}", offset), EditFormat::DecUnsigned);
        let offset = dlg.add(wnd, widget);

        dlg.add_separator();

        // pattern
        dlg.add_next(Text::new("Fill with pattern:"));
        let text = pattern.iter().map(|b| format!("{:02x}", b)).collect();
        let wnd = Window {
            x: 21,
            y: dlg.last_line - 1,
            width: 23,
            height: 1,
        };
        let widget = Edit::new(wnd.width, text, EditFormat::HexStream);
        let pattern = dlg.add(wnd, widget);

        // warning message
        dlg.add_separator();
        let msg_title = "WARNING!";
        dlg.add_center(msg_title.len(), Text::new(msg_title));
        let msg_text = "This operation cannot be undone!";
        dlg.add_center(msg_text.len(), Text::new(msg_text));

        // buttons
        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

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
                let length = if let WidgetData::Text(value) = dlg.get(handler.length) {
                    value.parse::<u64>().unwrap_or(0)
                } else {
                    0
                };
                debug_assert_ne!(length, 0);
                let offset = if let WidgetData::Text(value) = dlg.get(handler.offset) {
                    u64::from_str_radix(&value, 16).unwrap_or(0)
                } else {
                    0
                };
                let pattern = if let WidgetData::Text(val) = dlg.get(handler.pattern) {
                    (0..val.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&val[i..(i + 2).min(val.len())], 16).unwrap())
                        .collect()
                } else {
                    vec![0]
                };
                return Some((offset, length, pattern));
            }
        }
        None
    }
}

impl DialogHandler for InsertDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.btn_cancel || dialog.is_enabled(self.btn_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.length {
            if let WidgetData::Text(value) = dialog.get(self.length) {
                let is_valid = value.parse::<u64>().unwrap_or(0) != 0;
                dialog.set_state(self.btn_ok, is_valid);
            }
        }
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.length || item == self.offset {
            if let WidgetData::Text(value) = dialog.get(item) {
                if value.is_empty() {
                    dialog.set(item, WidgetData::Text("0".to_string()));
                }
            }
        } else if item == self.pattern {
            if let WidgetData::Text(mut value) = dialog.get(self.pattern) {
                if value.is_empty() {
                    dialog.set(self.pattern, WidgetData::Text("00".to_string()));
                } else if value.len() % 2 != 0 {
                    value.push('0');
                    dialog.set(self.pattern, WidgetData::Text(value));
                }
            }
        }
    }
}
