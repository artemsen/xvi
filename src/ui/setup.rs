// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::ascii::*;
use super::dialog::*;
use super::widget::*;

/// Dialog for setting up view parameters.
pub struct SetupDlg {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII characters table.
    pub ascii_table: Option<&'static AsciiTable>,
}

impl SetupDlg {
    /// Show configuration dialog.
    pub fn show(&mut self) -> bool {
        let mut dlg = Dialog::new(31, 8, DialogType::Normal, "Setup");
        let fixed = dlg.add_next(Checkbox::new("Fixed width (16 bytes)", self.fixed_width));
        dlg.add_separator();
        dlg.add_next(Text::new("ASCII field:"));

        let mut select = 0;
        let mut tables = Vec::with_capacity(ASCII_TABLES.len() + 1 /* None */);
        tables.push("None (hide)".to_string());
        for (index, table) in ASCII_TABLES.iter().enumerate() {
            tables.push(table.name.to_string());
            if let Some(current) = self.ascii_table {
                if current.id == table.id {
                    select = index + 1 /* "None (hide)" */;
                }
            }
        }
        let ascii = dlg.add_next(Listbox::new(tables, select));

        dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Bool(value) = dlg.get(fixed) {
                    self.fixed_width = value;
                }
                if let WidgetData::Number(value) = dlg.get(ascii) {
                    self.ascii_table = if value == 0 {
                        None
                    } else {
                        ASCII_TABLES.get(value - 1)
                    }
                }
                return true;
            }
        }
        false
    }
}
