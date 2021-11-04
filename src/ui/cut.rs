// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::range::RangeControl;
use super::widget::{Button, StdButton, Text};
use std::ops::Range;

/// Dialog for setting "cut out" parameters.
pub struct CutDlg {
    rctl: RangeControl,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl CutDlg {
    /// Show "Cut out" configuration dialog.
    ///
    /// # Arguments
    ///
    /// * `offset` - defualt start offset (current position)
    /// * `max` - max offset (file size)
    ///
    /// # Return value
    ///
    /// Range to cut out.
    pub fn show(offset: u64, max: u64) -> Option<Range<u64>> {
        // create dialog
        let mut dlg = Dialog::new(
            RangeControl::DIALOG_WIDTH + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Cut out range",
        );

        // place range control on dialog
        let rctl = RangeControl::create(&mut dlg, offset..offset + 1, max);

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
            rctl,
            btn_ok,
            btn_cancel,
        };

        // run dialog
        let mut range = None;
        if let Some(id) = dlg.run(&mut handler) {
            if id != handler.btn_cancel {
                range = handler.rctl.get(&dlg);
                debug_assert!(range.is_some());
            }
        }
        range
    }
}

impl DialogHandler for CutDlg {
    fn on_close(&mut self, dialog: &mut Dialog, current: ItemId) -> bool {
        current == self.btn_cancel || dialog.is_enabled(self.btn_ok)
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_item_change(dialog, item);
        dialog.set_state(self.btn_ok, self.rctl.get(dialog).is_some());
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_focus_lost(dialog, item);
    }
}
