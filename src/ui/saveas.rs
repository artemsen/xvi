// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{Button, Edit, EditFormat, StdButton, Text, WidgetData};

/// "Save as" dialog.
pub struct SaveAsDialog {
    path: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl SaveAsDialog {
    /// Show the "Save As" dialog.
    ///
    /// # Arguments
    ///
    /// * `default` - default file name
    ///
    /// # Return value
    ///
    /// New file name.
    pub fn show(default: String) -> Option<String> {
        // create dialog
        let mut dlg = Dialog::new(40 + Dialog::PADDING_X * 2, 6, DialogType::Normal, "Save as");

        dlg.add_next(Text::new("File name:"));
        let path = dlg.add_next(Edit::new(40, default, EditFormat::Any));

        // buttons
        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        let mut handler = Self {
            path,
            btn_ok,
            btn_cancel,
        };

        handler.on_item_change(&mut dlg, handler.path);

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                if let WidgetData::Text(value) = dlg.get(handler.path) {
                    return Some(value);
                }
            }
        }
        None
    }
}

impl DialogHandler for SaveAsDialog {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.btn_cancel || dialog.is_enabled(self.btn_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.path {
            let is_ok = if let WidgetData::Text(value) = dialog.get(self.path) {
                !value.is_empty()
            } else {
                false
            };
            dialog.set_state(self.btn_ok, is_ok);
        }
    }
}
