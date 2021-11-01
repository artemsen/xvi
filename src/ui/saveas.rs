// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// Dialog for asking a user for new file name ("Save as").
pub struct SaveAsDlg {
    // Items of the dialog.
    item_path: ItemId,
    item_ok: ItemId,
    item_cancel: ItemId,
}

impl SaveAsDlg {
    /// Show "Save As" dialog.
    ///
    /// # Arguments
    ///
    /// * `default` - default file name
    ///
    /// # Return value
    ///
    /// New file name.
    pub fn show(&mut self, default: String) -> Option<String> {
        let width = 40;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            6,
            DialogType::Normal,
            "Save as",
        );

        dlg.add_next(Text::new("File name:"));
        self.item_path = dlg.add_next(Edit::new(width, default, EditFormat::Any));
        self.item_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        self.item_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        self.on_item_change(&mut dlg, self.item_path);

        // run dialog
        if let Some(id) = dlg.run(self) {
            if id != self.item_cancel {
                if let WidgetData::Text(value) = dlg.get(self.item_path) {
                    return Some(value);
                }
            }
        }
        None
    }
}

impl DialogHandler for SaveAsDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.item_cancel || dialog.is_enabled(self.item_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.item_path {
            let is_ok = if let WidgetData::Text(value) = dialog.get(self.item_path) {
                !value.is_empty()
            } else {
                false
            };
            dialog.set_state(self.item_ok, is_ok);
        }
    }
}

impl Default for SaveAsDlg {
    fn default() -> Self {
        Self {
            item_path: -1,
            item_ok: -1,
            item_cancel: -1,
        }
    }
}
