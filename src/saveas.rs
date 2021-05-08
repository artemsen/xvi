// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// "Save as" dialog
pub struct SaveAsDialog;

impl SaveAsDialog {
    /// Show "Save As" dialog, returns new file path.
    pub fn show(default: String) -> Option<String> {
        let width = 40;
        let mut dlg = Dialog::new(DialogType::Normal);
        dlg.add(0, 0, width + 4, 6, Border::new("Save as"));

        dlg.add(2, 1, 0, 1, Text::new("File name:"));

        let editor = Edit::new(width, default, EditFormat::Any);
        let name = dlg.add(2, 2, editor.width, 1, editor);

        dlg.add(0, 3, width + 4, 1, Separator::new(None));
        let btn_ok = dlg.add(14, 4, 10, 1, Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add(21, 4, 10, 1, Button::std(StdButton::Cancel, false));

        dlg.rules.push(DialogRule::StateChange(
            name,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));

        dlg.rules
            .push(DialogRule::AllowExit(name, Box::new(DisableEmpty {})));
        dlg.cancel = btn_cancel;

        dlg.apply(name);

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Text(value) = dlg.get(name) {
                    return Some(value);
                }
            }
        }
        None
    }
}
