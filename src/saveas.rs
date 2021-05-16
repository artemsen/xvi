// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// "Save as" dialog.
pub struct SaveAsDialog;

impl SaveAsDialog {
    /// Show "Save As" dialog, returns new file path.
    pub fn show(default: String) -> Option<String> {
        let width = 40;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            6,
            DialogType::Normal,
            "Save as",
        );

        dlg.add_next(Text::new("File name:"));
        let name = dlg.add_next(Edit::new(width, default, EditFormat::Any));

        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        dlg.rules.push(DialogRule::StateChange(
            name,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));
        dlg.rules
            .push(DialogRule::AllowExit(name, Box::new(DisableEmpty {})));
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
