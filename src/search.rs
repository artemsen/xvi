// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::file::File;
use super::messagebox::MessageBox;
use super::progress::Progress;
use super::widget::*;

/// Search properties.
pub struct Search {
    /// Sequence to search.
    pub data: Vec<u8>,
    /// Search direction.
    pub backward: bool,
}

impl Search {
    /// Create new instance.
    //pub fn new(default: Vec<u8>) -> Self {
    //    Self {
    //        data: default,
    //        backward: false,
    //    }
    //}

    /// Find sequence inside the file.
    pub fn find(&self, file: &mut File, start: u64) -> Option<u64> {
        debug_assert!(!self.data.is_empty());

        let mut progress = Progress::new("Search", 0, file.size);
        let mut pval = 0;

        let step = 1024;
        let size = step + self.data.len() as i64;
        let mut offset = start as i64;

        if !self.backward {
            offset += 1;
        } else {
            offset -= 1;
        }

        let mut round = false;

        loop {
            pval += step as u64;
            if !progress.update(std::cmp::min(pval, file.size)) {
                // aborted by user
                return None;
            }

            if !self.backward {
                // forward search
                if offset as u64 >= file.size {
                    offset = 0;
                    round = true;
                }
            } else {
                // backward search
                if round && (offset as u64) < start {
                    break;
                }
                offset -= size;
                if offset < 0 {
                    if file.size < size as u64 {
                        offset = 0;
                    } else {
                        offset = file.size as i64 - size;
                    }
                    round = true;
                }
            }

            let file_data = file.get(offset as u64, size as usize).unwrap();
            let mut window = file_data.windows(self.data.len());
            if !self.backward {
                if let Some(pos) = window.position(|wnd| wnd == self.data) {
                    return Some(offset as u64 + pos as u64);
                }
            } else if let Some(pos) = window.rposition(|wnd| wnd == self.data) {
                return Some(offset as u64 + pos as u64);
            }

            if !self.backward {
                offset += step;
                if round && offset as u64 >= start {
                    break;
                }
            }
        }

        MessageBox::new("Search", DialogType::Error)
            .center("Sequence not found!")
            .button(StdButton::Ok, true)
            .show();

        None
    }

    /// Show search configuration dialog.
    pub fn configure(&mut self) -> bool {
        let mut init = String::with_capacity(self.data.len() * 2);
        for byte in self.data.iter() {
            init.push_str(&format!("{:02x}", byte));
        }

        let width = 40;
        let mut dlg = Dialog::new(
            width + Dialog::PADDING_X * 2,
            10,
            DialogType::Normal,
            "Search",
        );
        dlg.add_next(Text::new("Hex sequence to search:"));
        let hex = dlg.add_next(Edit::new(width, init, EditFormat::HexStream));
        dlg.add_next(Text::new("ASCII:"));
        let ascii = dlg.add_next(Edit::new(width, String::new(), EditFormat::Any));
        dlg.add_separator();
        let backward = dlg.add_next(Checkbox::new("Backward search", self.backward));

        let btn_ok = dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        dlg.rules
            .push(DialogRule::CopyData(hex, ascii, Box::new(HexToAscii {})));
        dlg.rules
            .push(DialogRule::CopyData(ascii, hex, Box::new(AsciiToHex {})));

        dlg.rules.push(DialogRule::StateChange(
            hex,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));
        dlg.rules.push(DialogRule::StateChange(
            ascii,
            btn_ok,
            Box::new(StateOnEmpty {}),
        ));

        dlg.rules
            .push(DialogRule::AllowExit(hex, Box::new(DisableEmpty {})));
        dlg.cancel = btn_cancel;

        dlg.apply(hex);

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Text(value) = dlg.get(hex) {
                    let mut value = value;
                    if value.len() % 2 != 0 {
                        value.push('0');
                    }
                    self.data = (0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                        .collect();
                }
                if let WidgetData::Bool(value) = dlg.get(backward) {
                    self.backward = value;
                }
                return true;
            }
        }
        false
    }

    /// Convert from text to byte array.
    fn get_hex(data: &WidgetData) -> Vec<u8> {
        let mut array = Vec::new();
        if let WidgetData::Text(value) = data {
            for it in (0..value.len()).step_by(2) {
                let hex = &value[it..it + if it + 2 < value.len() { 2 } else { 1 }];
                array.push(u8::from_str_radix(hex, 16).unwrap())
            }
        }
        array
    }
}

/// Dialog rule: set ASCII text from the hex field.
struct HexToAscii;
impl CopyData for HexToAscii {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        let mut ascii = String::new();
        for c in Search::get_hex(data).iter() {
            ascii.push(if *c > 0x20 && *c < 0x7f {
                *c as char
            } else {
                '?'
            });
        }
        Some(WidgetData::Text(ascii))
    }
}

/// Dialog rule: set hex text from the ASCII field.
struct AsciiToHex;
impl CopyData for AsciiToHex {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        if let WidgetData::Text(value) = data {
            Some(WidgetData::Text(
                value.chars().map(|i| format!("{:02x}", i as u8)).collect(),
            ))
        } else {
            None
        }
    }
}
