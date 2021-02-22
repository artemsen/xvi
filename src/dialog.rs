// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::widget::*;

/// Dialog window
pub struct Dialog<'a> {
    pub items: Vec<DialogItem<'a>>,
    pub focus: isize,
}
impl<'a> Dialog<'a> {
    const MARGIN_X: usize = 3;
    const MARGIN_Y: usize = 1;

    pub fn run(&mut self, cui: &'a dyn Cui) -> Option<usize> {
        let mut rc = None;

        let canvas = self.canvas(cui);

        // set focus to the first available widget
        if self.focus < 0 {
            self.move_focus(true);
        }

        // main event handler loop
        loop {
            // redraw
            self.draw_background(&canvas);
            let cursor = self.draw_widgets(&canvas);
            if let Some((x, y)) = cursor {
                cui.show_cursor(x, y);
            } else {
                cui.hide_cursor();
            }

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
                        _ => {
                            if self.focus >= 0 {
                                if !self.items[self.focus as usize].widget.has_input() {
                                    if event.key == Key::Left {
                                        self.move_focus(false);
                                        continue;
                                    } else if event.key == Key::Right {
                                        self.move_focus(true);
                                        continue;
                                    }
                                }
                                if let Some(id) =
                                    self.items[self.focus as usize].widget.keypress(event)
                                {
                                    rc = Some(id);
                                    break;
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

    fn canvas(&self, cui: &'a dyn Cui) -> Canvas<'a> {
        let mut width = 0;
        let mut height = 0;
        for item in self.items.iter() {
            let right = item.x + item.width;
            if right > width {
                width = right;
            }
            let bottom = item.y + item.height;
            if bottom > height {
                height = bottom;
            }
        }
        width += Dialog::MARGIN_X * 2;
        height += Dialog::MARGIN_Y * 2;

        let (screen_width, screen_height) = cui.size();

        Canvas {
            x: screen_width / 2 - width / 2,
            y: screen_height / 2 - height / 2,
            width,
            height,
            cui,
        }
    }

    /// Draw background and shadow of dialog window
    fn draw_background(&self, canvas: &Canvas) {
        let spaces = (0..canvas.width).map(|_| " ").collect::<String>();
        for y in 0..canvas.height {
            canvas.print(0, y, &spaces);
            canvas.color(0, y, canvas.width, Color::DialogNormal);
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

    fn draw_widgets(&self, canvas: &Canvas) -> Option<(usize, usize)> {
        let mut cursor: Option<(usize, usize)> = None;
        for (index, item) in self.items.iter().enumerate() {
            let subcan = Canvas {
                cui: canvas.cui,
                x: canvas.x + item.x + Dialog::MARGIN_X,
                y: canvas.y + item.y + Dialog::MARGIN_Y,
                width: item.width,
                height: item.height,
            };
            let cursor_x = item.widget.draw(index == self.focus as usize, &subcan);
            if let Some(x) = cursor_x {
                cursor = Some((subcan.x + x, subcan.y));
            }
        }
        cursor
    }

    /// Move the focus to the next/previous widget
    fn move_focus(&mut self, forward: bool) {
        debug_assert!(!self.items.is_empty());
        let mut focus = self.focus;
        loop {
            focus += if forward { 1 } else { -1 };
            if focus == self.focus {
                break; // no one focusable items
            }
            if focus < 0 {
                focus = self.items.len() as isize - 1;
            } else if focus == self.items.len() as isize {
                focus = 0;
            }
            if self.items[focus as usize].widget.focusable() {
                break;
            }
        }
        self.focus = focus;
    }
}

/// Single dialog item
pub struct DialogItem<'a> {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub widget: &'a mut dyn Widget,
}
impl<'a> DialogItem<'a> {
    pub fn new(
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        widget: &'a mut dyn Widget,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            widget,
        }
    }
}

/// Message box dialog
pub struct MessageBox {
    border: Border,
    message: Vec<Text>,
    separator: Separator,
    buttons: Vec<Button>,
}
impl MessageBox {
    pub fn create(title: &str) -> Self {
        Self {
            border: Border::new(title),
            message: Vec::new(),
            separator: Separator::new(""),
            buttons: Vec::new(),
        }
    }

    pub fn add_line(&mut self, text: &str) -> &mut Self {
        self.message.push(Text::new(text));
        self
    }

    pub fn add_multiline(&mut self, text: &str) -> &mut Self {
        for line in text.lines() {
            self.add_line(line);
        }
        self
    }

    pub fn add_button(&mut self, button: usize, default: bool) -> &mut Self {
        self.buttons.push(Button::std(button, default));
        self
    }

    /// Show message box
    pub fn show(&mut self, cui: &dyn Cui) -> Option<usize> {
        // calculate buttons line width
        let mut buttons_width = 0;
        for button in self.buttons.iter() {
            if buttons_width != 0 {
                buttons_width += 1; // space between buttons
            }
            buttons_width += button.text.len();
        }

        // calculate min width
        let mut width = buttons_width;
        for line in self.message.iter() {
            if width < line.text.len() {
                width = line.text.len();
            }
        }

        let mut items = Vec::new();
        let mut focus = -1;
        let mut y = 0;

        // border
        let height = self.message.len() + if self.buttons.is_empty() { 2 } else { 4 };
        items.push(DialogItem::new(0, 0, width + 4, height, &mut self.border));

        // message text
        y += 1;
        for line in self.message.iter_mut() {
            let len = line.text.len();
            let x = 2 + (width - len) / 2;
            items.push(DialogItem::new(x, y, len, 1, line));
            y += 1;
        }

        // separator between message and buttons
        if !self.buttons.is_empty() {
            items.push(DialogItem::new(0, y, width + 4, 1, &mut self.separator));
            y += 1;
        }

        // buttons line
        let mut x = 2 + width / 2 - buttons_width / 2;
        for button in self.buttons.iter_mut() {
            let width = button.text.len();
            if button.default {
                focus = items.len() as isize;
            }
            items.push(DialogItem::new(x, y, width, 1, button));
            x += width + 1;
        }

        Dialog { items, focus }.run(cui)
    }
}

/// "Save As" dialog
pub struct SaveAsDialog;
impl SaveAsDialog {
    /// Show "Save As" dialog, returns new file path
    pub fn show(cui: &dyn Cui, default: &str) -> Option<String> {
        let width = 40;
        let mut border = Border::new("Save as");
        let mut message = Text::new("File name:");
        let mut edit = Edit::new(width, default, EditFormat::Any);
        let mut separator = Separator::new("");
        let mut button_ok = Button::std(Button::OK, true);
        let mut button_cancel = Button::std(Button::CANCEL, false);
        let mut dlg = Dialog {
            items: vec![
                DialogItem::new(0, 0, width + 4, 6, &mut border),
                DialogItem::new(2, 1, message.text.len(), 1, &mut message),
                DialogItem::new(2, 2, width, 1, &mut edit),
                DialogItem::new(0, 3, width + 4, 1, &mut separator),
                DialogItem::new(14, 4, button_ok.text.len(), 1, &mut button_ok),
                DialogItem::new(21, 4, button_cancel.text.len(), 1, &mut button_cancel),
            ],
            focus: -1,
        };
        if let Some(id) = dlg.run(cui) {
            if id == Button::OK {
                return Some(edit.value);
            }
        }
        None
    }
}

/// "Go to" dialog
pub struct GotoDialog;
impl GotoDialog {
    /// Show "Go to" dialog, return absolute address to jump
    pub fn show(cui: &dyn Cui, default: u64) -> Option<u64> {
        let width = 40;
        let mut border = Border::new("Go to");
        let mut message = Text::new("Address:");
        let mut edit = Edit::new(width, &format!("{:x}", default), EditFormat::Hex);
        let mut separator = Separator::new("");
        let mut button_ok = Button::std(Button::OK, true);
        let mut button_cancel = Button::std(Button::CANCEL, false);
        let mut dlg = Dialog {
            items: vec![
                DialogItem::new(0, 0, width + 4, 6, &mut border),
                DialogItem::new(2, 1, message.text.len(), 1, &mut message),
                DialogItem::new(2, 2, width, 1, &mut edit),
                DialogItem::new(0, 3, width + 4, 1, &mut separator),
                DialogItem::new(14, 4, button_ok.text.len(), 1, &mut button_ok),
                DialogItem::new(21, 4, button_cancel.text.len(), 1, &mut button_cancel),
            ],
            focus: -1,
        };
        if let Some(id) = dlg.run(cui) {
            if id == Button::OK {
                return match u64::from_str_radix(&edit.value, 16) {
                    Ok(offset) => Some(offset),
                    Err(_) => None,
                };
            }
        }
        None
    }
}

/// "Find" dialog
#[allow(dead_code)]
pub struct FindDialog;
#[allow(unused_variables)]
impl FindDialog {
    /// Show "Find" dialog, return ?
    pub fn show(cui: &dyn Cui, default: &[u8]) -> Option<Vec<u8>> {
        let width = 40;
        let mut init = String::with_capacity(default.len() * 2);
        for byte in default {
            init.push_str(&format!("{:02x}", byte));
        }
        let mut border = Border::new("Find");
        let mut message = Text::new("Hex:");
        let mut edit = Edit::new(width, &init, EditFormat::Hex);
        let mut separator = Separator::new("");
        let mut button_ok = Button::std(Button::OK, true);
        let mut button_cancel = Button::std(Button::CANCEL, false);
        let mut dlg = Dialog {
            items: vec![
                DialogItem::new(0, 0, width + 4, 6, &mut border),
                DialogItem::new(2, 1, message.text.len(), 1, &mut message),
                DialogItem::new(2, 2, width, 1, &mut edit),
                DialogItem::new(0, 3, width + 4, 1, &mut separator),
                DialogItem::new(14, 4, button_ok.text.len(), 1, &mut button_ok),
                DialogItem::new(21, 4, button_cancel.text.len(), 1, &mut button_cancel),
            ],
            focus: -1,
        };
        if let Some(id) = dlg.run(cui) {
            if id == Button::OK {
                let mut value = edit.value;
                if value.is_empty() {
                    return None;
                }
                if value.len() % 2 != 0 {
                    value.push('0');
                }
                let find = (0..value.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                    .collect();
                return Some(find);
            }
        }
        None
    }
}
