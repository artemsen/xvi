// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Curses, Event, Key};
use super::super::file::ProgressHandler;
use super::dialog::{Dialog, DialogType, ItemId};
use super::messagebox::MessageBox;
use super::widget::{StandardButton, WidgetType};

/// Progress dialog.
pub struct ProgressDialog {
    dlg: Dialog,
    bar: ItemId,
    confirm: bool,
}

impl ProgressDialog {
    /// Create new progress window.
    ///
    /// # Arguments
    ///
    /// * `title` - window title
    /// * `confirm` - flag to ask confirmation for abort
    ///
    /// # Return value
    ///
    /// Progress window instance.
    pub fn new(title: &str, confirm: bool) -> Self {
        let mut dlg = Dialog::new(50, 1, DialogType::Normal, title);
        let bar = dlg.add_line(WidgetType::ProgressBar(0));
        dlg.add_button(StandardButton::Cancel, true);

        let mut instance = Self { dlg, bar, confirm };
        instance.update(0);

        instance
    }

    /// Hide progress window.
    pub fn hide(&self) {
        self.dlg.hide();
    }

    /// Ask user for confirmation to cancel the current operation.
    ///
    /// # Return value
    ///
    /// `true` if user confirmed abort
    fn confirm_abort() -> bool {
        if let Some(button) = MessageBox::show(
            DialogType::Error,
            "Abort",
            &["Are you sure you want to abort the current operation?"],
            &[(StandardButton::Yes, false), (StandardButton::No, true)],
        ) {
            return button == StandardButton::Yes;
        }
        false
    }
}

impl ProgressHandler for ProgressDialog {
    fn update(&mut self, percent: u8) -> bool {
        debug_assert!(percent <= 100);

        if let WidgetType::ProgressBar(current) = self.dlg.get_widget_mut(self.bar) {
            *current = percent;
            self.dlg.draw();
        }

        // check for user interrupt
        if let Some(Event::KeyPress(key)) = Curses::peek_event() {
            if matches!(key.key, Key::Esc | Key::Enter | Key::Char(' '))
                && (!self.confirm || ProgressDialog::confirm_abort())
            {
                return false;
            }
        }
        true
    }
}
