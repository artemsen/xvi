// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::widget::{InputFormat, InputLine, StandardButton, WidgetType};

/// "Save as" dialog.
pub struct SaveAsDialog {
    path: ItemId,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl SaveAsDialog {
    /// Width of the dialog.
    const WIDTH: usize = 40;

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
        let mut dlg = Dialog::new(SaveAsDialog::WIDTH, 2, DialogType::Normal, "Save as");
        // file path input
        dlg.add_line(WidgetType::StaticText("File name:".to_string()));
        let edit = InputLine::new(default, InputFormat::Any, Vec::new(), SaveAsDialog::WIDTH);
        let path = dlg.add_line(WidgetType::Edit(edit));
        // buttons
        let btn_ok = dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            path,
            btn_ok,
            btn_cancel,
        };
        handler.on_item_change(&mut dlg, handler.path);

        // run dialog
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                if let WidgetType::Edit(widget) = dlg.get_widget(handler.path) {
                    return Some(widget.get_value().to_string());
                }
            }
        }
        None
    }
}

impl DialogHandler for SaveAsDialog {
    fn on_close(&mut self, dialog: &mut Dialog, item: ItemId) -> bool {
        item == self.btn_cancel || dialog.get_context(self.btn_ok).enabled
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.path {
            let is_ok = match dialog.get_widget(self.path) {
                WidgetType::Edit(widget) => !widget.get_value().is_empty(),
                _ => true,
            };
            dialog.set_enabled(self.btn_ok, is_ok);
        }
    }
}
