// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::ascii;
use super::super::view::View;
use super::dialog::{Dialog, DialogType};
use super::widget::{CheckBox, ListBox, StandardButton, WidgetType};

/// Dialog for setting the viewer parameters.
pub struct SetupDialog {}

impl SetupDialog {
    /// Show the "Setup" dialog.
    ///
    /// # Arguments
    ///
    /// * `view` - viewer instance to set up
    ///
    /// # Return value
    ///
    /// true if settings were changed
    pub fn show(view: &mut View) -> bool {
        // create dialog
        let mut dlg = Dialog::new(27, 4, DialogType::Normal, "Setup");

        // fixed width setup
        let checkbox = CheckBox {
            state: view.fixed_width,
            title: "Fixed width (16 bytes)".to_string(),
        };
        let fixed = dlg.add_line(WidgetType::CheckBox(checkbox));
        dlg.add_separator();

        // ASCII encoding
        dlg.add_line(WidgetType::StaticText("ASCII field:".to_string()));
        let mut select = 0;
        let mut tables = Vec::with_capacity(ascii::TABLES.len() + 1 /* None */);
        tables.push("None (hide)".to_string());
        for (index, table) in ascii::TABLES.iter().enumerate() {
            tables.push(table.name.to_string());
            if let Some(current) = view.ascii_table {
                if current.id == table.id {
                    select = index + 1 /* "None (hide)" */;
                }
            }
        }
        let listbox = ListBox {
            list: tables,
            current: select,
        };
        let ascii = dlg.add_line(WidgetType::ListBox(listbox));

        // buttons
        dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // run dialog
        if let Some(id) = dlg.run_simple() {
            if id != btn_cancel {
                if let WidgetType::CheckBox(widget) = dlg.get_widget(fixed) {
                    view.fixed_width = widget.state;
                }
                if let WidgetType::ListBox(widget) = dlg.get_widget(ascii) {
                    view.ascii_table = if widget.current == 0 {
                        None
                    } else {
                        ascii::TABLES.get(widget.current - 1)
                    }
                }
                return true;
            }
        }
        false
    }
}
