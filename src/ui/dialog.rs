// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Color, Curses, Event, Key, KeyPress, Window};
use super::widget::{Border, Button, Separator, Widget, WidgetData};

/// Dialog window.
pub struct Dialog {
    /// Dialog size and position.
    wnd: Window,
    /// Items on the dialog window.
    items: Vec<DialogItem>,
    /// Last used line number (used for easy dialog construction).
    pub last_line: usize,
    /// Currently focused item.
    pub focus: ItemId,
    /// Dialog type (background color).
    dtype: DialogType,
}

impl Dialog {
    const MARGIN_X: usize = 3;
    const MARGIN_Y: usize = 1;
    pub const PADDING_X: usize = 2;
    pub const PADDING_Y: usize = 1;

    /// Create new dialog instance.
    pub fn new(width: usize, height: usize, dt: DialogType, title: &str) -> Self {
        // calculate dialogs's window size and position
        let screen = Curses::get_screen();
        let dlg_width = width + Dialog::MARGIN_X * 2;
        let dlg_height = height + Dialog::MARGIN_Y * 2;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let wnd = Window {
            x: screen.width / 2 - dlg_width / 2,
            y: (screen.height as f32 / 2.5) as usize - dlg_height / 2,
            width: dlg_width,
            height: dlg_height,
        };

        // initial items: border with separator
        let border = DialogItem {
            wnd: Window {
                x: 0,
                y: 0,
                width,
                height,
            },
            enabled: true,
            widget: Border::new(title),
        };
        let separator = DialogItem {
            wnd: Window {
                x: 0,
                y: height - 3,
                width,
                height: 1,
            },
            enabled: true,
            widget: Separator::new(None),
        };

        Self {
            wnd,
            items: vec![border, separator],
            last_line: Dialog::PADDING_Y,
            focus: ItemId::MAX,
            dtype: dt,
        }
    }

    /// Construct dialog: add new item.
    pub fn add(&mut self, wnd: Window, widget: Box<dyn Widget>) -> ItemId {
        self.items.push(DialogItem {
            wnd,
            enabled: true,
            widget,
        });
        self.items.len() as ItemId - 1
    }

    /// Construct dialog: add one lined item to the next row.
    pub fn add_next(&mut self, widget: Box<dyn Widget>) -> ItemId {
        let wnd = Window {
            x: Dialog::PADDING_X,
            y: self.last_line,
            width: self.wnd.width - (Dialog::MARGIN_X + Dialog::PADDING_X) * 2,
            height: 1,
        };
        self.last_line += 1;
        self.add(wnd, widget)
    }

    /// Construct dialog: add one lined item to the next row at the center.
    pub fn add_center(&mut self, width: usize, widget: Box<dyn Widget>) -> ItemId {
        let center = (self.wnd.width - (Dialog::MARGIN_X * 2)) / 2;
        let wnd = Window {
            x: center - width / 2,
            y: self.last_line,
            width,
            height: 1,
        };
        self.last_line += 1;
        self.add(wnd, widget)
    }

    /// Construct dialog: add horizontal separator to the next row.
    pub fn add_separator(&mut self) {
        let wnd = Window {
            x: 0,
            y: self.last_line,
            width: self.wnd.width - Dialog::MARGIN_X * 2,
            height: 1,
        };
        self.last_line += 1;
        self.add(wnd, Separator::new(None));
    }

    /// Construct dialog: add button to the main block (Ok, Cancel, etc).
    pub fn add_button(&mut self, button: Box<Button>) -> ItemId {
        let y = self.wnd.height - (Dialog::MARGIN_Y + Dialog::PADDING_Y) * 2;
        let width = button.text.len();
        let center = (self.wnd.width - (Dialog::MARGIN_X * 2)) / 2;
        let x = if self.items.iter().any(|i| i.wnd.y == y) {
            // total width of the items on the same line
            let mut total_width = width;
            for item in self.items.iter().filter(|i| i.wnd.y == y) {
                total_width += item.wnd.width + 1 /* space */;
            }
            // move items on the same line to the left
            let mut x = center - total_width / 2;
            for item in self.items.iter_mut().filter(|i| i.wnd.y == y) {
                item.wnd.x = x;
                x += item.wnd.width + 1 /* space */;
            }
            x
        } else {
            center - width / 2
        };
        let wnd = Window {
            x,
            y,
            width,
            height: 1,
        };
        self.add(wnd, button)
    }

    /// Get item's data.
    pub fn get(&self, id: ItemId) -> WidgetData {
        self.items[id as usize].widget.as_ref().get_data()
    }

    /// Set item's data.
    pub fn set(&mut self, id: ItemId, data: WidgetData) {
        self.items[id as usize].widget.as_mut().set_data(data);
    }

    /// Check item state.
    pub fn is_enabled(&self, id: ItemId) -> bool {
        self.items[id as usize].enabled
    }

    /// Enable or disable item.
    pub fn set_state(&mut self, id: ItemId, state: bool) {
        self.items[id as usize].enabled = state;
    }

    /// Run dialog: show window and handle external events.
    pub fn run(&mut self, handler: &mut dyn DialogHandler) -> Option<ItemId> {
        // set focus to the first available widget
        if self.focus == ItemId::MAX {
            self.move_focus(true);
        }

        // main event handler loop
        loop {
            // redraw
            self.draw();

            // handle next event
            match Curses::wait_event() {
                Event::TerminalResize => {}
                Event::KeyPress(event) => {
                    match event.key {
                        Key::Tab => {
                            let previous = self.move_focus(event.modifier != KeyPress::SHIFT);
                            handler.on_focus_lost(self, previous);
                        }
                        Key::Esc => {
                            return None;
                        }
                        Key::Enter => {
                            if handler.on_close(self, self.focus) {
                                return Some(self.focus);
                            }
                        }
                        _ => {
                            if self.focus != ItemId::MAX {
                                if self.items[self.focus as usize].widget.keypress(&event) {
                                    handler.on_item_change(self, self.focus);
                                } else if event.key == Key::Left || event.key == Key::Up {
                                    let previous = self.move_focus(false);
                                    handler.on_focus_lost(self, previous);
                                } else if event.key == Key::Right || event.key == Key::Down {
                                    let previous = self.move_focus(true);
                                    handler.on_focus_lost(self, previous);
                                }
                            }
                        }
                    };
                }
            }
        }
    }

    /// Show simple dialog without external handlers.
    pub fn run_simple(&mut self) -> Option<ItemId> {
        let mut dummy = DialogEmptyHandler {};
        self.run(&mut dummy)
    }

    /// Draw dialog.
    pub fn draw(&self) {
        Curses::color_on(if self.dtype == DialogType::Normal {
            Color::DialogNormal
        } else {
            Color::DialogError
        });

        // background
        let spaces = (0..self.wnd.width).map(|_| " ").collect::<String>();
        for y in 0..self.wnd.height {
            self.wnd.print(0, y, &spaces);
        }

        // shadow
        let screen = Curses::get_screen();
        for y in (self.wnd.y + 1)..(self.wnd.y + self.wnd.height) {
            screen.color(self.wnd.x + self.wnd.width, y, 2, Color::DialogShadow);
        }
        screen.color(
            self.wnd.x + 2,
            self.wnd.y + self.wnd.height,
            self.wnd.width,
            Color::DialogShadow,
        );

        // dialog items
        let mut cursor: Option<(usize, usize)> = None;
        for (index, item) in self.items.iter().enumerate() {
            let subcan = Window {
                x: self.wnd.x + item.wnd.x + Dialog::MARGIN_X,
                y: self.wnd.y + item.wnd.y + Dialog::MARGIN_Y,
                width: item.wnd.width,
                height: item.wnd.height,
            };
            let cursor_x = item
                .widget
                .draw(index == self.focus as usize, item.enabled, &subcan);
            if let Some(x) = cursor_x {
                cursor = Some((subcan.x + x, subcan.y));
            }
        }
        if let Some((x, y)) = cursor {
            Curses::show_cursor(x, y);
        } else {
            Curses::hide_cursor();
        }
    }

    /// Move the focus to the next/previous widget, returns previously focused item Id.
    fn move_focus(&mut self, forward: bool) -> ItemId {
        debug_assert!(!self.items.is_empty());

        let mut focus = if self.focus == ItemId::MAX {
            // first launch, focues wasn't set yet
            self.items.len() as ItemId - 1
        } else {
            self.focus
        };

        let mut lap = false;

        loop {
            if forward {
                focus += 1;
                if focus == self.items.len() as ItemId {
                    if lap {
                        return ItemId::MAX; // no one focusable items
                    }
                    lap = true;
                    focus = 0;
                }
            } else {
                if focus == 0 {
                    if lap {
                        return ItemId::MAX; // no one focusable items
                    }
                    lap = true;
                    focus = self.items.len() as ItemId;
                }
                focus -= 1;
            }

            if self.items[focus as usize].enabled && self.items[focus as usize].widget.focusable() {
                break;
            }
        }

        let previous = self.focus;
        self.focus = focus;
        self.items[focus as usize].widget.focus();
        previous
    }
}

/// Dialog type.
#[derive(Copy, Clone, PartialEq)]
pub enum DialogType {
    Normal,
    Error,
}

/// Type of dialog's item ID.
pub type ItemId = usize;

/// Single dialog item.
pub struct DialogItem {
    wnd: Window,
    enabled: bool,
    widget: Box<dyn Widget>,
}

/// Dialog handlers.
pub trait DialogHandler {
    /// Check if dialog can be closed (not canceled).
    ///
    /// # Arguments
    ///
    /// * `dialog` - dialog instance
    /// * `current` - currently focused item Id
    ///
    /// # Return value
    ///
    /// true if dialog can be closed.
    fn on_close(&mut self, _dialog: &mut Dialog, _current: ItemId) -> bool {
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
