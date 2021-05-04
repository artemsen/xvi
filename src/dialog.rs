// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::widget::*;

/// Dialog window.
pub struct Dialog {
    /// Items on the dialog window.
    items: Vec<DialogItem>,
    /// Dialog's rules (links between items, etc).
    pub rules: Vec<DialogRule>,
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

    /// Create new dialog instance.
    pub fn new(dt: DialogType) -> Self {
        Self {
            items: Vec::new(),
            rules: Vec::new(),
            focus: -1,
            cancel: -1,
            dtype: dt,
        }
    }

    /// Construct dialog: add new item.
    pub fn add(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        widget: Box<dyn Widget>,
    ) -> ItemId {
        self.items.push(DialogItem {
            x,
            y,
            width,
            height,
            widget,
            enabled: true,
        });
        self.items.len() as isize - 1
    }

    /// Get item's data.
    pub fn get(&self, id: ItemId) -> WidgetData {
        self.items[id as usize].widget.as_ref().get_data()
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
    pub fn run(&mut self, cui: &dyn Cui) -> Option<ItemId> {
        let mut rc = None;

        // canvas for the dialog
        let (screen_width, screen_height) = cui.size();
        let mut canvas = Canvas {
            x: screen_width / 2,
            y: (screen_height as f32 / 2.5) as usize,
            width: 0,
            height: 0,
            cui,
        };
        for item in self.items.iter() {
            let right = item.x + item.width;
            if right > canvas.width {
                canvas.width = right;
            }
            let bottom = item.y + item.height;
            if bottom > canvas.height {
                canvas.height = bottom;
            }
        }
        canvas.x -= canvas.width / 2;
        canvas.y -= canvas.height / 2;
        canvas.width += Dialog::MARGIN_X * 2;
        canvas.height += Dialog::MARGIN_Y * 2;

        // set focus to the first available widget
        if self.focus < 0 {
            self.move_focus(true);
        }

        // main event handler loop
        loop {
            // redraw
            self.draw(&canvas);

            // handle next event
            match cui.poll_event() {
                Event::TerminalResize => {}
                Event::KeyPress(event) => {
                    match event.key {
                        Key::Tab => {
                            self.move_focus(event.modifier != KeyPress::SHIFT);
                        }
                        Key::Up => {
                            self.move_focus(false);
                        }
                        Key::Down => {
                            self.move_focus(true);
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
                                } else if event.key == Key::Left {
                                    self.move_focus(false);
                                } else if event.key == Key::Right {
                                    self.move_focus(true);
                                }
                            }
                        }
                    };
                }
            }
        }
        cui.clear();

        rc
    }

    /// Draw dialog.
    fn draw(&self, canvas: &Canvas) {
        canvas.color_on(if self.dtype == DialogType::Normal {
            Color::DialogNormal
        } else {
            Color::DialogError
        });
        self.draw_background(&canvas);
        let cursor = self.draw_items(&canvas);
        if let Some((x, y)) = cursor {
            canvas.cui.show_cursor(x, y);
        } else {
            canvas.cui.hide_cursor();
        }
    }

    /// Draw background and shadow of dialog window.
    fn draw_background(&self, canvas: &Canvas) {
        let spaces = (0..canvas.width).map(|_| " ").collect::<String>();
        for y in 0..canvas.height {
            canvas.print(0, y, &spaces);
        }
        // shadow, out of window
        for y in (canvas.y + 1)..(canvas.y + canvas.height) {
            canvas
                .cui
                .color(canvas.x + canvas.width, y, 2, Color::DialogShadow);
        }
        canvas.cui.color(
            canvas.x + 2,
            canvas.y + canvas.height,
            canvas.width,
            Color::DialogShadow,
        );
    }

    /// Draw items.
    fn draw_items(&self, canvas: &Canvas) -> Option<(usize, usize)> {
        let mut cursor: Option<(usize, usize)> = None;
        for (index, item) in self.items.iter().enumerate() {
            let subcan = Canvas {
                cui: canvas.cui,
                x: canvas.x + item.x + Dialog::MARGIN_X,
                y: canvas.y + item.y + Dialog::MARGIN_Y,
                width: item.width,
                height: item.height,
            };
            let cursor_x = item
                .widget
                .draw(index == self.focus as usize, item.enabled, &subcan);
            if let Some(x) = cursor_x {
                cursor = Some((subcan.x + x, subcan.y));
            }
        }
        cursor
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
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub enabled: bool,
    pub widget: Box<dyn Widget>,
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
