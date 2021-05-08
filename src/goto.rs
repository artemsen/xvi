// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;

/// "Go to" dialog.
pub struct GotoDialog;

impl GotoDialog {
    /// Show "Go to" dialog, return absolute address to jump.
    pub fn show(default: u64, current: u64) -> Option<u64> {
        let width = 44;
        let mut dlg = Dialog::new(DialogType::Normal);
        dlg.add(0, 0, width + 4, 8, Border::new("Go to"));

        dlg.add(2, 1, 0, 1, Text::new("Absolute offset:"));

        dlg.add(2, 2, 0, 1, Text::new("hex:"));
        let editor = Edit::new(17, format!("{:x}", default), EditFormat::HexUnsigned);
        let abshex = dlg.add(6, 2, editor.width, 1, editor);

        dlg.add(24, 2, 0, 1, Text::new("dec:"));
        let editor = Edit::new(18, String::new(), EditFormat::DecUnsigned);
        let absdec = dlg.add(28, 2, editor.width, 1, editor);

        dlg.add(2, 3, 0, 1, Text::new("Relative offset:"));

        dlg.add(2, 4, 0, 1, Text::new("hex:"));
        let editor = Edit::new(17, String::new(), EditFormat::HexSigned);
        let relhex = dlg.add(6, 4, editor.width, 1, editor);

        dlg.add(24, 4, 0, 1, Text::new("dec:"));
        let editor = Edit::new(18, String::new(), EditFormat::DecSigned);
        let reldec = dlg.add(28, 4, editor.width, 1, editor);

        dlg.add(0, 5, width + 4, 1, Separator::new(None));
        dlg.add(16, 6, 10, 1, Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add(23, 6, 10, 1, Button::std(StdButton::Cancel, false));

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
                EditFormat::DecSigned => {
                    self.current as i64 + i64::from_str_radix(&value, 10).unwrap_or(0)
                }
                EditFormat::DecUnsigned => i64::from_str_radix(&value, 10).unwrap_or(0),
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
