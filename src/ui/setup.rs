// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::ascii::TABLES;
use super::super::view::View;
use super::dialog::{Dialog, DialogType};
use super::widget::{Button, Checkbox, Listbox, StdButton, Text, WidgetData};

/// Dialog for setting the viewer parameters.
pub struct SetupDialog {}

impl SetupDialog {
    /// Show the "Setup" dialog.
    ///
    /// # Arguments
    ///
    /// * `default` - default file name
    ///
    /// # Return value
    ///
    /// true if settings were changed
    pub fn show(view: &mut View) -> bool {
        // create dialog
        let mut dlg = Dialog::new(27 + Dialog::PADDING_X * 2, 8, DialogType::Normal, "Setup");

        let fixed = dlg.add_next(Checkbox::new("Fixed width (16 bytes)", view.fixed_width));
        dlg.add_separator();

        // ASCII encoding
        dlg.add_next(Text::new("ASCII field:"));
        let mut select = 0;
        let mut tables = Vec::with_capacity(TABLES.len() + 1 /* None */);
        tables.push("None (hide)".to_string());
        for (index, table) in TABLES.iter().enumerate() {
            tables.push(table.name.to_string());
            if let Some(current) = view.ascii_table {
                if current.id == table.id {
                    select = index + 1 /* "None (hide)" */;
                }
            }
        }
        let ascii = dlg.add_next(Listbox::new(tables, select));

        // buttons
        dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));

        if let Some(id) = dlg.run_simple() {
            if id != btn_cancel {
                if let WidgetData::Bool(value) = dlg.get(fixed) {
                    view.fixed_width = value;
                }
                if let WidgetData::Number(value) = dlg.get(ascii) {
                    view.ascii_table = if value == 0 {
                        None
                    } else {
                        TABLES.get(value - 1)
                    }
                }
                return true;
            }
        }
        false
    }
}
