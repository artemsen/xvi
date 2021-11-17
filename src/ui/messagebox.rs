// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogType, ItemId};
use super::widget::StandardButton;
use std::io::Error;
use unicode_segmentation::UnicodeSegmentation;

/// Message box dialog.
pub struct MessageBox {}

impl MessageBox {
    /// Show message box.
    ///
    /// # Arguments
    ///
    /// * `dt` - message type (dialog background)
    /// * `title` - message title
    /// * `message` - message lines
    /// * `buttons` - buttons on the dialog
    ///
    /// # Return value
    ///
    /// Chosen button.
    pub fn show(
        dt: DialogType,
        title: &str,
        message: &[&str],
        buttons: &[(StandardButton, bool)],
    ) -> Option<StandardButton> {
        debug_assert!(!message.is_empty());
        debug_assert!(!buttons.is_empty());

        // create dialog
        let (width, height) = MessageBox::calc_size(message, buttons);
        let mut dlg = Dialog::new(width, height, dt, title);

        // add message text to the dialog window
        for msg in message.iter() {
            let mut line = msg.to_string();
            let line_len = line.graphemes(true).count();
            if line_len > width {
                // shrink line
                let (index, _) = line.grapheme_indices(true).nth(width - 1).unwrap();
                line.truncate(index);
                line.push('\u{2026}');
            }
            dlg.add_center(line);
        }

        // buttons
        let mut first = ItemId::MAX;
        for (button, default) in buttons.iter() {
            let item = dlg.add_button(*button, *default);
            if first == ItemId::MAX {
                first = item;
            }
        }

        // show dialog
        dlg.show_unmanaged().map(|id| buttons[id - first].0)
    }

    /// Show message about reading errors.
    ///
    /// # Arguments
    ///
    /// * `file` - path to the file
    /// * `err` - error description
    /// * `buttons` - buttons on the dialog
    ///
    /// # Return value
    ///
    /// Chosen button.
    pub fn error_read(
        file: &str,
        err: &Error,
        buttons: &[(StandardButton, bool)],
    ) -> Option<StandardButton> {
        let error = format!("{}", err);
        let message = vec!["Error reading file", file, &error];
        MessageBox::show(DialogType::Error, "Error", &message, buttons)
    }

    /// Show message about writing errors.
    ///
    /// # Arguments
    ///
    /// * `file` - path to the file
    /// * `err` - error description
    /// * `buttons` - buttons on the dialog
    ///
    /// # Return value
    ///
    /// Chosen button.
    pub fn error_write(
        file: &str,
        err: &Error,
        buttons: &[(StandardButton, bool)],
    ) -> Option<StandardButton> {
        let error = format!("{}", err);
        let message = vec!["Error writing file", file, &error];
        MessageBox::show(DialogType::Error, "Error", &message, buttons)
    }

    /// Show message about writing errors and ask user for retry.
    ///
    /// # Arguments
    ///
    /// * `file` - path to the file
    /// * `err` - error description
    ///
    /// # Return value
    ///
    /// 'true` if attempt must be repeated
    pub fn retry_write(file: &str, err: &Error) -> bool {
        if let Some(button) = MessageBox::error_write(
            file,
            err,
            &[
                (StandardButton::Retry, true),
                (StandardButton::Cancel, false),
            ],
        ) {
            button == StandardButton::Retry
        } else {
            false
        }
    }

    /// Calculate window size.
    ///
    /// # Arguments
    ///
    /// * `message` - message lines
    /// * `buttons` - buttons on the dialog
    ///
    /// # Return value
    ///
    /// Size of the dialog window.
    fn calc_size(message: &[&str], buttons: &[(StandardButton, bool)]) -> (usize, usize) {
        let max_width = Dialog::max_width();
        let mut width = 0;

        // size of buttons block
        for (button, default) in buttons.iter() {
            width += 1 + button.text(*default).len();
        }
        width -= 1; // remove last space

        // longest message line
        for msg in message.iter() {
            let len = msg.graphemes(true).count();
            if width < max_width && width < len {
                width = len.min(max_width);
            }
        }

        (width, message.len())
    }
}
