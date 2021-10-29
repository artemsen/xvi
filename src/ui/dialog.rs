// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::*;
use super::widget::{Border, Button, Separator, Widget, WidgetData};

/// Dialog window.
pub struct Dialog {
    /// Dialog size and position.
    wnd: Window,
    /// Items on the dialog window.
    items: Vec<DialogItem>,
    /// Dialog's rules (links between items, etc).
    pub rules: Vec<DialogRule>,
    /// Last used line number (used for easy dialog construction).
    pub last_line: usize,
    /// Currently focused item.
    pub focus: ItemId,
    /// Identifier of the Cancel button to force exit.
    pub cancel: ItemId,
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
            rules: Vec::new(),
            last_line: Dialog::PADDING_Y,
            focus: -1,
            cancel: -1,
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

    /// Construct dialog: add centered item.
    pub fn add_center(&mut self, y: usize, width: usize, widget: Box<dyn Widget>) -> ItemId {
        let center = (self.wnd.width - (Dialog::MARGIN_X * 2)) / 2;
        let x = if !self.items.iter().any(|i| i.wnd.y == y) {
            center - width / 2
        } else {
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
        };

        let wnd = Window {
            x,
            y,
            width,
            height: 1,
        };
        self.add(wnd, widget)
    }

    /// Construct dialog: add button to the main block (Ok, Cancel, etc).
    pub fn add_button(&mut self, button: Box<Button>) -> ItemId {
        let y = self.wnd.height - (Dialog::MARGIN_Y + Dialog::PADDING_Y) * 2;
        self.add_center(y, button.text.len(), button)
    }

    /// Get item's data.
    pub fn get(&self, id: ItemId) -> WidgetData {
        self.items[id as usize].widget.as_ref().get_data()
    }

    /// Set item's data.
    pub fn set(&mut self, id: ItemId, data: WidgetData) {
        self.items[id as usize].widget.as_mut().set_data(data);
    }

    /// Apply rules for specified items.
    pub fn apply(&mut self, item: ItemId) {
        for it in self.rules.iter() {
            match it {
                DialogRule::CopyData(src, dst, handler) => {
                    if item == *src {
                        let data = self.items[*src as usize].widget.as_ref().get_data();
                        if let Some(data) = handler.as_ref().copy_data(&data) {
                            self.items[*dst as usize].widget.as_mut().set_data(data);
                        }
                    }
                }
                DialogRule::StateChange(src, dst, handler) => {
                    if item == *src {
                        let data = self.items[*src as usize].widget.as_ref().get_data();
                        if let Some(state) = handler.as_ref().set_state(&data) {
                            self.items[*dst as usize].enabled = state;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Run dialog: show window and handle external events.
    pub fn run(&mut self) -> Option<ItemId> {
        let mut rc = None;

        // set focus to the first available widget
        if self.focus < 0 {
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
                            self.move_focus(event.modifier != KeyPress::SHIFT);
                        }
                        Key::Esc => {
                            break;
                        }
                        Key::Enter => {
                            if self.focus == self.cancel {
                                rc = Some(self.cancel);
                                break;
                            }
                            let mut allow = true;
                            'out: for it in self.rules.iter() {
                                if let DialogRule::AllowExit(id, handler) = it {
                                    for item in 0..self.items.len() {
                                        if item as isize == *id
                                            && !handler.allow_exit(&self.get(*id))
                                        {
                                            allow = false;
                                            break 'out;
                                        }
                                    }
                                }
                            }
                            if allow {
                                rc = Some(self.focus);
                                break;
                            }
                        }
                        _ => {
                            if self.focus >= 0 {
                                if self.items[self.focus as usize].widget.keypress(&event) {
                                    self.apply(self.focus);
                                } else if event.key == Key::Left || event.key == Key::Up {
                                    self.move_focus(false);
                                } else if event.key == Key::Right || event.key == Key::Down {
                                    self.move_focus(true);
                                }
                            }
                        }
                    };
                }
            }
        }

        rc
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

    /// Move the focus to the next/previous widget.
    fn move_focus(&mut self, forward: bool) {
        debug_assert!(!self.items.is_empty());
        let mut focus = self.focus;
        loop {
            focus += if forward { 1 } else { -1 };
            if focus == self.focus {
                return; // no one focusable items
            }
            if focus < 0 {
                focus = self.items.len() as isize - 1;
            } else if focus == self.items.len() as isize {
                if self.focus == -1 {
                    return; // no one focusable items
                }
                focus = 0;
            }
            if self.items[focus as usize].enabled && self.items[focus as usize].widget.focusable() {
                break;
            }
        }
        self.focus = focus;
        self.items[focus as usize].widget.focus();
    }
}

/// Dialog type.
#[derive(Copy, Clone, PartialEq)]
pub enum DialogType {
    Normal,
    Error,
}

/// Type of dialog's item ID.
pub type ItemId = isize;

/// Single dialog item.
pub struct DialogItem {
    wnd: Window,
    enabled: bool,
    widget: Box<dyn Widget>,
}

/// Dialog rules.
pub enum DialogRule {
    /// Copy data from one widget to another if first one has been changed.
    CopyData(ItemId, ItemId, Box<dyn CopyData>),
    /// Enable or disable item depending on source data.
    StateChange(ItemId, ItemId, Box<dyn StateChange>),
    /// Check for exit.
    AllowExit(ItemId, Box<dyn AllowExit>),
}

pub trait CopyData {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData>;
}
pub trait StateChange {
    fn set_state(&self, data: &WidgetData) -> Option<bool>;
}
pub trait AllowExit {
    fn allow_exit(&self, data: &WidgetData) -> bool;
}

/// Dialog rule: enables or disables item if data is empty.
pub struct StateOnEmpty;
impl StateChange for StateOnEmpty {
    fn set_state(&self, data: &WidgetData) -> Option<bool> {
        match data {
            WidgetData::Text(value) => Some(!value.is_empty()),
            _ => None,
        }
    }
}

/// Dialog rule: prevents exit if data is empty.
pub struct DisableEmpty;
impl AllowExit for DisableEmpty {
    fn allow_exit(&self, data: &WidgetData) -> bool {
        match data {
            WidgetData::Text(value) => !value.is_empty(),
            _ => true,
        }
    }
}
