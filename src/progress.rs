// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// Progress dialog.
pub struct Progress {
    min: u64,
    max: u64,
    dlg: Dialog,
    did: ItemId,
}

#[allow(dead_code)]
impl Progress {
    pub fn new(title: &str, min: u64, max: u64) -> Self {
        let mut dlg = Dialog::new(50, 6, DialogType::Normal, title);
        let did = dlg.add_next(ProgressBar::new());
        Self { min, max, dlg, did }
    }

    /// Show progress dialog.
    pub fn update(&mut self, value: u64) {
        debug_assert!(value >= self.min && value <= self.max);
        let percent = ((value - self.min) * 100)
            / if self.max == self.min {
                1
            } else {
                self.max - self.min
            };

        self.dlg.set(self.did, WidgetData::Number(percent as usize));
        self.dlg.draw();
    }
}
