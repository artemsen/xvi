// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::{Dialog, DialogHandler, ItemId};
use super::widget::{InputFormat, InputLine, WidgetType};
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
    /// Width of the dialog.
    pub const DIALOG_WIDTH: usize = 43;
    /// Width of the offset field.
    const OFFSET_WIDTH: usize = 13;
    /// Width of the lenght field.
    const LENGTH_WIDTH: usize = 9;

    pub fn create(dlg: &mut Dialog, default: Range<u64>, max: u64) -> Self {
        debug_assert!(!default.is_empty());
        debug_assert!(default.end <= max);

        dlg.add_line(WidgetType::StaticText(
            "            Start                  End".to_string(),
        ));
        dlg.add_line(WidgetType::StaticText(
            "Range:               \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
        ));

        // start offset widget
        let widget = InputLine::new(
            format!("{:x}", default.start),
            InputFormat::HexUnsigned,
            Vec::new(),
            RangeControl::OFFSET_WIDTH,
        );
        let start = dlg.add(
            Dialog::PADDING_X + 8,
            Dialog::PADDING_Y + 1,
            RangeControl::OFFSET_WIDTH,
            WidgetType::Edit(widget),
        );

        // end offset widget
        let widget = InputLine::new(
            format!("{:x}", default.end - 1),
            InputFormat::HexUnsigned,
            Vec::new(),
            RangeControl::OFFSET_WIDTH,
        );
        let end = dlg.add(
            Dialog::PADDING_X + 30,
            Dialog::PADDING_Y + 1,
            RangeControl::OFFSET_WIDTH,
            WidgetType::Edit(widget),
        );

        dlg.add_line(WidgetType::StaticText("Length:       \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}         \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}".to_string()));

        // range length widget
        let widget = InputLine::new(
            format!("{}", default.end - default.start),
            InputFormat::DecUnsigned,
            Vec::new(),
            RangeControl::LENGTH_WIDTH,
        );
        let length = dlg.add(
            Dialog::PADDING_X + 21,
            Dialog::PADDING_Y + 2,
            RangeControl::LENGTH_WIDTH,
            WidgetType::Edit(widget),
        );

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
        if let WidgetType::Edit(widget) = dialog.get_widget(item) {
            offset = u64::from_str_radix(widget.get_value(), 16).unwrap_or(0);
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
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.length) {
                widget.set_value(format!("{}", length));
            }
        } else if item == self.length {
            let mut length = 1;
            if let WidgetType::Edit(widget) = dialog.get_widget(self.length) {
                length = widget.get_value().parse::<u64>().unwrap_or(0);
                if length == 0 {
                    length = 1;
                }
            }
            let mut end = self.get_offset(dialog, self.start) + length - 1;
            if end >= self.max {
                end = self.max - 1;
            }
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.end) {
                widget.set_value(format!("{:x}", end));
            }
        }
    }

    fn on_focus_lost(&mut self, dialog: &mut Dialog, item: ItemId) {
        if item == self.start || item == self.end {
            let offset = self.get_offset(dialog, item);
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(item) {
                widget.set_value(format!("{:x}", offset));
            }
        } else if item == self.length {
            let start = self.get_offset(dialog, self.start);
            let end = self.get_offset(dialog, self.end);
            let length = if start > end { 0 } else { end - start + 1 };
            if let WidgetType::Edit(widget) = dialog.get_widget_mut(self.length) {
                widget.set_value(format!("{}", length));
            }
        }
    }
}
