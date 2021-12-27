// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Color, Curses, Event, Key, KeyPress, Window};
use super::widget::{Button, StandardButton, WidgetContext, WidgetType};
use unicode_segmentation::UnicodeSegmentation;

/// Dialog window.
pub struct Dialog {
    /// Dialog window.
    window: Window,
    /// Dialog items.
    items: Vec<DialogItem>,
    /// Last used line number used by constructor.
    lcline: usize,
    /// Currently focused item.
    focus: ItemId,
    /// Title.
    title: String,
    /// Resize flag.
    resized: bool,
}

impl Dialog {
    // Size of the field between window edge and border
    const BORDER_X: usize = 3;
    const BORDER_Y: usize = 1;
    // Size of the padding
    pub const PADDING_X: usize = Dialog::BORDER_X + 2;
    pub const PADDING_Y: usize = Dialog::BORDER_Y + 1;

    /// Create new dialog.
    ///
    /// # Arguments
    ///
    /// * `width` - width of the useful area
    /// * `height` - height of the useful area
    /// * `dt` - dialog type, used for setting background color
    /// * `title` - dialog title
    ///
    /// # Return value
    ///
    /// Dialog instance.
    pub fn new(width: usize, height: usize, dt: DialogType, title: &str) -> Self {
        // dialog size with borders
        let width = width + Dialog::PADDING_X * 2;
        let height = height + Dialog::PADDING_Y * 2 + 2 /* buttons with separator */;

        // dialog color from type
        let color = if dt == DialogType::Normal {
            Color::Dialog
        } else {
            Color::Error
        };

        // separator above buttons block
        let separator = DialogItem {
            widget: WidgetType::Separator {},
            context: WidgetContext {
                x: Dialog::BORDER_X,
                y: height - Dialog::PADDING_Y - 2,
                width: width - Dialog::BORDER_X * 2,
                ..WidgetContext::default()
            },
        };

        Self {
            window: Window::new_centered(width, height, color),
            items: vec![separator],
            lcline: Dialog::PADDING_Y,
            focus: ItemId::MAX,
            title: format!(" {} ", title),
            resized: false,
        }
    }

    /// Get size of the dialog exclude borders and padding.
    ///
    /// # Return value
    ///
    /// Size of the useful area.
    pub fn get_size(&self) -> (usize, usize) {
        let (mut width, mut height) = self.window.get_size();
        width -= Dialog::PADDING_X * 2;
        height -= Dialog::PADDING_Y * 2;
        (width, height)
    }

    /// Run dialog: show window and handle input events.
    ///
    /// # Arguments
    ///
    /// * `handler` - dialog custom handlers
    ///
    /// # Return value
    ///
    /// Last focused item Id or None if Esc pressed.
    pub fn show(&mut self, handler: &mut dyn DialogHandler) -> Option<ItemId> {
        let mut last_focus = None;

        if self.focus == ItemId::MAX {
            self.initialize_focus();
        }

        // main event handler loop
        loop {
            // redraw
            self.draw();

            // handle next event
            match Curses::wait_event() {
                Event::TerminalResize => {
                    self.resized = true;
                }
                Event::KeyPress(event) => {
                    match event.key {
                        Key::Tab => {
                            if let Some(previous) =
                                self.move_focus(event.modifier != KeyPress::SHIFT)
                            {
                                handler.on_focus_lost(self, previous);
                            }
                        }
                        Key::Esc => {
                            break;
                        }
                        Key::Enter => {
                            if handler.on_close(self, self.focus) {
                                last_focus = Some(self.focus);
                                break;
                            }
                        }
                        _ => {
                            if self.focus != ItemId::MAX {
                                let item = &mut self.items[self.focus];
                                if item.widget.key_press(&event) {
                                    handler.on_item_change(self, self.focus);
                                } else if event.key == Key::Left || event.key == Key::Up {
                                    if let Some(previous) = self.move_focus(false) {
                                        handler.on_focus_lost(self, previous);
                                    }
                                } else if event.key == Key::Right || event.key == Key::Down {
                                    if let Some(previous) = self.move_focus(true) {
                                        handler.on_focus_lost(self, previous);
                                    }
                                }
                            }
                        }
                    };
                }
            }
        }

        if self.resized {
            Curses::screen_resize();
        }

        last_focus
    }

    /// Show simple dialog without external handlers.
    pub fn show_unmanaged(&mut self) -> Option<ItemId> {
        let mut dummy = DialogEmptyHandler {};
        self.show(&mut dummy)
    }

    /// Hide dialog window.
    pub fn hide(&self) {
        self.window.hide();
    }

    /// Get item widget instance.
    ///
    /// # Arguments
    ///
    /// * `item` - item Id
    ///
    /// # Return value
    ///
    /// Widget instance.
    pub fn get_widget(&self, item: ItemId) -> &WidgetType {
        &self.items[item].widget
    }

    /// Get mutable item widget instance.
    ///
    /// # Arguments
    ///
    /// * `item` - item Id
    ///
    /// # Return value
    ///
    /// Mutable widget instance.
    pub fn get_widget_mut(&mut self, item: ItemId) -> &mut WidgetType {
        &mut self.items[item].widget
    }

    /// Get item context.
    ///
    /// # Arguments
    ///
    /// * `item` - item Id
    ///
    /// # Return value
    ///
    /// Context of the item.
    pub fn get_context(&self, item: ItemId) -> &WidgetContext {
        &self.items[item].context
    }

    /// Enable or disable item.
    ///
    /// # Arguments
    ///
    /// * `item` - item Id
    /// * `state` - new state to set
    pub fn set_enabled(&mut self, item: ItemId, state: bool) {
        self.items[item].context.enabled = state;
    }

    /// Add new widget onto dialog window.
    ///
    /// # Arguments
    ///
    /// * `x` - widget position
    /// * `y` - widget position
    /// * `width` - widget width
    /// * `widget` - widget to add
    ///
    /// # Return value
    ///
    /// Item Id of added widget.
    pub fn add(&mut self, x: usize, y: usize, width: usize, widget: WidgetType) -> ItemId {
        let context = WidgetContext {
            x,
            y,
            width,
            ..WidgetContext::default()
        };
        self.items.push(DialogItem { widget, context });
        self.items.len() as ItemId - 1
    }

    /// Add new widget on the next line on dialog window.
    ///
    /// # Arguments
    ///
    /// * `widget` - widget to add
    ///
    /// # Return value
    ///
    /// Item Id of added widget.
    pub fn add_line(&mut self, widget: WidgetType) -> ItemId {
        let (width, _) = self.get_size();
        let line = self.lcline;
        self.lcline += 1;
        self.add(Dialog::PADDING_X, line, width, widget)
    }

    /// Add centered text on the next line on dialog window.
    ///
    /// # Arguments
    ///
    /// * `text` - static text to add
    pub fn add_center(&mut self, text: String) {
        debug_assert!(!text.is_empty());
        let (width, _) = self.get_size();
        let len = text.graphemes(true).count();
        debug_assert!(len <= width);
        let x = Dialog::PADDING_X + width / 2 - len / 2;
        let line = self.lcline;
        self.lcline += 1;
        self.add(x, line, len, WidgetType::StaticText(text));
    }

    /// Add standart button on dialog.
    ///
    /// # Arguments
    ///
    /// * `button` - button to add
    /// * `default` - default button flag
    ///
    /// # Return value
    ///
    /// Item Id of added widget.
    pub fn add_button(&mut self, button: StandardButton, default: bool) -> ItemId {
        let text = button.text(default);

        let (width, height) = self.get_size();
        let btn_width = text.len();
        let btn_y = height + Dialog::PADDING_Y - 1;
        let mut btn_x = Dialog::PADDING_X;

        // total width of buttons on the same line
        let mut total_width = btn_width;
        for item in self.items.iter().filter(|i| i.context.y == btn_y) {
            total_width += item.context.width + 1 /* space */;
        }
        debug_assert!(total_width <= width);

        // calculate position of the button
        let center = width / 2 - total_width / 2;
        if total_width == btn_width {
            btn_x += center;
        } else {
            // move items on the same line to the left
            let mut move_x = center;
            for item in self.items.iter_mut().filter(|i| i.context.y == btn_y) {
                item.context.x = Dialog::PADDING_X + move_x;
                move_x += item.context.width + 1 /* space */;
            }
            btn_x += move_x;
        }

        let widget = WidgetType::Button(Button { text, default });
        self.add(btn_x, btn_y, btn_width, widget)
    }

    /// Add separator in the nect line on dialog window.
    pub fn add_separator(&mut self) {
        let (mut width, _) = self.window.get_size();
        width -= Dialog::BORDER_X * 2;
        let line = self.lcline;
        self.lcline += 1;
        self.add(Dialog::BORDER_X, line, width, WidgetType::Separator {});
    }

    /// Draw dialog window.
    pub fn draw(&self) {
        self.window.clear();

        // draw border
        let (mut width, mut height) = self.window.get_size();
        width -= Dialog::BORDER_X * 2;
        height -= Dialog::BORDER_Y * 2;
        // top line with title
        let line = "\u{2554}".to_string() + &"\u{2550}".repeat(width - 2) + "\u{2557}";
        self.window.print(Dialog::BORDER_X, Dialog::BORDER_Y, &line);
        let length = self.title.len();
        let center = Dialog::BORDER_X + width / 2 - length / 2;
        self.window.print(center, Dialog::BORDER_Y, &self.title);
        self.window
            .set_style(center, Dialog::BORDER_Y, length, Window::BOLD);
        // bottom line
        let line = "\u{255a}".to_string() + &"\u{2550}".repeat(width - 2) + "\u{255d}";
        self.window
            .print(Dialog::BORDER_X, Dialog::BORDER_Y + height - 1, &line);
        // left and right lines
        for y in Dialog::BORDER_Y + 1..Dialog::BORDER_Y + height - 1 {
            self.window.print(Dialog::BORDER_X, y, "\u{2551}");
            self.window
                .print(Dialog::BORDER_X + width - 1, y, "\u{2551}");
        }

        // dialog items
        let mut cursor: Option<(usize, usize)> = None;
        for item in &self.items {
            if let Some((x, y)) = item.widget.draw(&self.window, &item.context) {
                cursor = Some((x, y));
            }
        }
        if let Some((x, y)) = cursor {
            self.window.show_cursor(x, y);
        } else {
            Window::hide_cursor();
        }

        self.window.refresh();
    }

    /// Set focus to the next windget.
    ///
    /// # Arguments
    ///
    /// * `forward` - focus movement direction
    ///
    /// # Return value
    ///
    /// Previously focus (item that lost the focus).
    fn move_focus(&mut self, forward: bool) -> Option<ItemId> {
        debug_assert_ne!(self.focus, ItemId::MAX);

        let mut focus = self.focus;
        loop {
            if forward {
                focus += 1;
                if focus == self.items.len() as ItemId {
                    focus = 0;
                }
            } else {
                if focus == 0 {
                    focus = self.items.len() as ItemId;
                }
                focus -= 1;
            }

            let item = &self.items[focus];
            if item.context.enabled && item.widget.focusable() {
                break;
            }
        }

        if focus != self.focus {
            let previous = self.focus;

            let item = &mut self.items[self.focus];
            item.context.focused = false;

            self.focus = focus;

            let item = &mut self.items[self.focus];
            item.context.focused = true;
            item.widget.focus_set();

            return Some(previous);
        }
        None
    }

    /// Set initial focus.
    fn initialize_focus(&mut self) {
        debug_assert_eq!(self.focus, ItemId::MAX);

        // find the first focusable widget
        for (index, item) in self.items.iter().enumerate() {
            if item.context.enabled && item.widget.focusable() {
                self.focus = index;
                break;
            }
        }

        debug_assert_ne!(self.focus, ItemId::MAX); // no one focusable item?

        // if focus is inside the buttons block then set it to default button
        if let WidgetType::Button(_) = &self.items[self.focus].widget {
            for (index, item) in self.items.iter().skip(self.focus).enumerate() {
                if let WidgetType::Button(widget) = &item.widget {
                    if widget.default {
                        self.focus += index;
                        break;
                    }
                }
            }
        }

        let item = &mut self.items[self.focus];
        item.context.focused = true;
        item.widget.focus_set();
    }

    /// Get max width of the dialog.
    ///
    /// # Return value
    ///
    /// Max width of useful area (exclude borders).
    pub fn max_width() -> usize {
        let (width, _) = Curses::screen_size();
        let padding = Dialog::PADDING_X * 2;
        if width < padding {
            0
        } else {
            width - padding
        }
    }
}

/// Dialog type.
#[derive(Copy, Clone, PartialEq)]
pub enum DialogType {
    Normal,
    Error,
}

/// Type of dialog item ID.
pub type ItemId = usize;

/// Single dialog item.
pub struct DialogItem {
    widget: WidgetType,
    context: WidgetContext,
}

/// Dialog handlers.
pub trait DialogHandler {
    /// Check if dialog can be closed (not canceled).
    ///
    /// # Arguments
    ///
    /// * `dialog` - dialog instance
    /// * `item` - currently focused item Id
    ///
    /// # Return value
    ///
    /// true if dialog can be closed.
    fn on_close(&mut self, _dialog: &mut Dialog, _item: ItemId) -> bool {
        true
    }

    /// Item change callback.
    ///
    /// # Arguments
    ///
    /// * `dialog` - dialog instance
    /// * `item` - Id of the item that was changed
    fn on_item_change(&mut self, _dialog: &mut Dialog, _item: ItemId) {}

    /// Focus lost callback.
    ///
    /// # Arguments
    ///
    /// * `dialog` - dialog instance
    /// * `item` - Id of the item that lost focus
    fn on_focus_lost(&mut self, _dialog: &mut Dialog, _item: ItemId) {}
}

/// Empty dialog handler, used for simple dialogs.
struct DialogEmptyHandler;
impl DialogHandler for DialogEmptyHandler {}
