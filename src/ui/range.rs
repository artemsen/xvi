// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::Window;
use super::dialog::*;
use super::widget::*;
use std::ops::Range;

/// Range control: set of widgets and handlers.
pub struct RangeControl {
    // Max possible value.
    max: u64,
    // Items of the dialog.
    start: ItemId,
    end: ItemId,
    length: ItemId,
}

impl RangeControl {
    pub const DIALOG_WIDTH: usize = 43;

    pub fn create(dialog: &mut Dialog, default: Range<u64>, max: u64) -> Self {
        debug_assert!(!default.is_empty());
        debug_assert!(default.end <= max);

        dialog.add_next(Text::new("            Start                  End"));
        dialog.add_next(Text::new("Range:               ─────────"));

        // start offset widget
        let wnd = Window {
            x: 10,
            y: dialog.last_line - 1,
            width: 13,
            height: 1,
        };
        let widget = Edit::new(
            wnd.width,
            format!("{:x}", default.start),
            EditFormat::HexUnsigned,
        );
        let start = dialog.add(wnd, widget);

        // end offset widget
        let wnd = Window {
            x: 32,
            y: dialog.last_line - 1,
            width: 13,
            height: 1,
        };
        let widget = Edit::new(
            wnd.width,
            format!("{:x}", default.end - 1),
            EditFormat::HexUnsigned,
        );
        let end = dialog.add(wnd, widget);

        dialog.add_next(Text::new("Length:       └──────         ──────┘"));

        // range length widget
        let wnd = Window {
            x: 23,
            y: dialog.last_line - 1,
            width: 9,
            height: 1,
        };
        let widget = Edit::new(
            wnd.width,
            format!("{}", default.end - default.start),
            EditFormat::DecUnsigned,
        );
        let length = dialog.add(wnd, widget);

        Self {
            max,
            start,
            end,
            length,
        }
    }

    /// Get range specified in the control fields.
    pub fn get(&self, dialog: &Dialog) -> Option<Range<u64>> {
        let start = self.get_offset(dialog, self.start);
        let end = self.get_offset(dialog, self.end);
        if start <= end && start < self.max {
            Some(start..end + 1)
        } else {
            None
        }
    }

    /// Get normalized offset value from the widget.
    fn get_offset(&self, dialog: &Dialog, item: ItemId) -> u64 {
        let mut offset = 0;
        if let WidgetData::Text(value) = dialog.get(item) {
            offset = u64::from_str_radix(&value, 16).unwrap_or(0);
            if offset >= self.max {
                offset = self.max - 1;
            }
        }
        offset
    }
}

impl DialogHandler for RangeControl {
    fn on_item_change(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.start || item == self.end {
            let start = self.get_offset(dialog, self.start);
            let end = self.get_offset(dialog, self.end);
            let length = if start > end { 0 } else { end - start + 1 };
            dialog.set(self.length, WidgetData::Text(format!("{}", length)));
        } else if item == self.length {
            let mut length = 1;
            if let WidgetData::Text(value) = dialog.get(self.length) {
                length = value.parse::<u64>().unwrap_or(0);
                if length == 0 {
                    length = 1;
                }
            }
            let mut end = self.get_offset(dialog, self.start) + length - 1;
            if end >= self.max {
                end = self.max - 1;
            }
            dialog.set(self.end, WidgetData::Text(format!("{:x}", end)));
        }
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.start || item == self.end {
            let offset = self.get_offset(dialog, item);
            dialog.set(item, WidgetData::Text(format!("{:x}", offset)));
        } else if item == self.length {
            let start = self.get_offset(dialog, self.start);
            let end = self.get_offset(dialog, self.end);
            let length = if start > end { 0 } else { end - start + 1 };
            dialog.set(self.length, WidgetData::Text(format!("{}", length)));
        }
    }
}
