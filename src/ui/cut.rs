// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, DialogType, ItemId};
use super::range::RangeControl;
use super::widget::StandardButton;
use std::ops::Range;

/// "Cut out range" dialog.
pub struct CutDialog {
    rctl: RangeControl,
    btn_ok: ItemId,
    btn_cancel: ItemId,
}

impl CutDialog {
    /// Show the "Cut out" configuration dialog.
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
            RangeControl::DIALOG_WIDTH,
            6,
            DialogType::Normal,
            "Cut out range",
        );

        // place range control on dialog
        let rctl = RangeControl::create(&mut dlg, offset..offset + 1, max);

        // warning message
        dlg.add_separator();
        dlg.add_center("WARNING!".to_string());
        dlg.add_center("This operation cannot be undone!".to_string());

        // buttons
        let btn_ok = dlg.add_button(StandardButton::OK, true);
        let btn_cancel = dlg.add_button(StandardButton::Cancel, false);

        // construct dialog handler
        let mut handler = Self {
            rctl,
            btn_ok,
            btn_cancel,
        };

        // show dialog
        if let Some(id) = dlg.show(&mut handler) {
            if id != handler.btn_cancel {
                let range = handler.rctl.get(&dlg);
                debug_assert!(range.is_some());
                return range;
            }
        }
        None
    }
}

impl DialogHandler for CutDialog {
    fn on_close(&mut self, dialog: &mut Dialog, item: ItemId) -> bool {
        item == self.btn_cancel || dialog.get_context(self.btn_ok).enabled
    }

    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_item_change(dialog, item);
        dialog.set_enabled(self.btn_ok, self.rctl.get(dialog).is_some());
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        self.rctl.on_focus_lost(dialog, item);
    }
}
