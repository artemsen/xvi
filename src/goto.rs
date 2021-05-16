// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::curses::Window;
use super::dialog::*;
use super::widget::*;

/// "Go to" dialog.
pub struct GotoDialog;

impl GotoDialog {
    /// Show "Go to" dialog, return absolute address to jump.
    pub fn show(default: u64, current: u64) -> Option<u64> {
        let width = 44;
        let mut dlg = Dialog::new(width + Dialog::PADDING_X * 2, 9, DialogType::Normal, "Goto");

        dlg.add_next(Text::new("Absolute offset"));
        let abshex =
            GotoDialog::add_edit(&mut dlg, Dialog::PADDING_X, "hex:", EditFormat::HexUnsigned);
        let absdec = GotoDialog::add_edit(
            &mut dlg,
            Dialog::PADDING_X + 23,
            "dec:",
            EditFormat::DecUnsigned,
        );
        dlg.last_line += 1; // skip

        dlg.add_separator();
        dlg.add_next(Text::new("Relative offset"));
        let relhex =
            GotoDialog::add_edit(&mut dlg, Dialog::PADDING_X, "hex:", EditFormat::HexSigned);
        let reldec = GotoDialog::add_edit(
            &mut dlg,
            Dialog::PADDING_X + 23,
            "dec:",
            EditFormat::DecSigned,
        );

        dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        let conv = OffsetConverter::new(EditFormat::HexUnsigned, EditFormat::DecUnsigned, 0);
        dlg.rules.push(DialogRule::CopyData(abshex, absdec, conv));
        let conv = OffsetConverter::new(EditFormat::HexUnsigned, EditFormat::HexSigned, current);
        dlg.rules.push(DialogRule::CopyData(abshex, relhex, conv));
        let conv = OffsetConverter::new(EditFormat::HexUnsigned, EditFormat::DecSigned, current);
        dlg.rules.push(DialogRule::CopyData(abshex, reldec, conv));

        let conv = OffsetConverter::new(EditFormat::DecUnsigned, EditFormat::HexUnsigned, 0);
        dlg.rules.push(DialogRule::CopyData(absdec, abshex, conv));
        let conv = OffsetConverter::new(EditFormat::DecUnsigned, EditFormat::HexSigned, current);
        dlg.rules.push(DialogRule::CopyData(absdec, relhex, conv));
        let conv = OffsetConverter::new(EditFormat::DecUnsigned, EditFormat::DecSigned, current);
        dlg.rules.push(DialogRule::CopyData(absdec, reldec, conv));

        let conv = OffsetConverter::new(EditFormat::HexSigned, EditFormat::DecSigned, current);
        dlg.rules.push(DialogRule::CopyData(relhex, reldec, conv));
        let conv = OffsetConverter::new(EditFormat::HexSigned, EditFormat::HexUnsigned, current);
        dlg.rules.push(DialogRule::CopyData(relhex, abshex, conv));
        let conv = OffsetConverter::new(EditFormat::HexSigned, EditFormat::DecUnsigned, current);
        dlg.rules.push(DialogRule::CopyData(relhex, absdec, conv));

        let conv = OffsetConverter::new(EditFormat::DecSigned, EditFormat::HexSigned, current);
        dlg.rules.push(DialogRule::CopyData(reldec, relhex, conv));
        let conv = OffsetConverter::new(EditFormat::DecSigned, EditFormat::HexUnsigned, current);
        dlg.rules.push(DialogRule::CopyData(reldec, abshex, conv));
        let conv = OffsetConverter::new(EditFormat::DecSigned, EditFormat::DecUnsigned, current);
        dlg.rules.push(DialogRule::CopyData(reldec, absdec, conv));

        dlg.set(abshex, WidgetData::Text(format!("{:x}", default)));
        dlg.apply(abshex);

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Text(value) = dlg.get(abshex) {
                    return match u64::from_str_radix(&value, 16) {
                        Ok(offset) => Some(offset),
                        Err(_) => Some(0),
                    };
                }
            }
        }
        None
    }

    /// Add edit field with title.
    fn add_edit(dlg: &mut Dialog, x: usize, title: &str, fmt: EditFormat) -> ItemId {
        let text = Window {
            x,
            y: dlg.last_line,
            width: title.len(),
            height: 1,
        };
        let width = 17; // edit field length
        let edit = Window {
            x: x + text.width,
            y: dlg.last_line,
            width,
            height: 1,
        };
        dlg.add(text, Text::new(title));
        dlg.add(edit, Edit::new(width, String::new(), fmt))
    }
}

/// Dialog rule: convert offset to different type.
struct OffsetConverter {
    pub src: EditFormat,
    pub dst: EditFormat,
    pub current: u64,
}

impl OffsetConverter {
    fn new(src: EditFormat, dst: EditFormat, current: u64) -> Box<Self> {
        Box::new(Self { src, dst, current })
    }
}

impl CopyData for OffsetConverter {
    fn copy_data(&self, data: &WidgetData) -> Option<WidgetData> {
        if let WidgetData::Text(value) = data {
            let src = match self.src {
                EditFormat::Any => unreachable!(),
                EditFormat::HexStream => unreachable!(),
                EditFormat::HexSigned => {
                    self.current as i64 + i64::from_str_radix(&value, 16).unwrap_or(0)
                }
                EditFormat::HexUnsigned => i64::from_str_radix(&value, 16).unwrap_or(0),
                EditFormat::DecSigned => self.current as i64 + (&value).parse::<i64>().unwrap_or(0),
                EditFormat::DecUnsigned => (&value).parse::<i64>().unwrap_or(0),
            };
            let dst = match self.dst {
                EditFormat::Any => unreachable!(),
                EditFormat::HexStream => unreachable!(),
                EditFormat::HexSigned => {
                    let offset = src - self.current as i64;
                    let sign = if offset < 0 { "-" } else { "+" };
                    format!("{}{:x}", sign, i64::abs(offset))
                }
                EditFormat::HexUnsigned => format!("{:x}", if src >= 0 { src } else { 0 }),
                EditFormat::DecSigned => format!("{:+}", src - self.current as i64),
                EditFormat::DecUnsigned => format!("{}", if src >= 0 { src } else { 0 }),
            };
            Some(WidgetData::Text(dst))
        } else {
            None
        }
    }
}
