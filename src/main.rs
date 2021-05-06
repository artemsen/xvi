// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

mod config;
mod cui;
mod curses;
mod cursor;
mod dialog;
mod editor;
mod file;
mod goto;
mod history;
mod inifile;
mod messagebox;
mod page;
mod saveas;
mod search;
mod widget;
use config::*;
use curses::Curses;
use editor::Editor;
use history::*;
use std::collections::HashMap;

struct Argument {
    short: char,
    long: &'static str,
    param: Option<&'static str>,
    help: &'static str,
}

#[rustfmt::skip]
const ARGS: &[Argument] = &[
    Argument {
        short: 'o', long: "offset", param: Some("ADDRESS"),
        help: "Initial cursor position (jump to offset)",
    },
    Argument {
        short: 'v', long: "version", param: None,
        help: "Print version info and exit",
    },
    Argument {
        short: 'h', long: "help", param: None,
        help: "Print this help and exit",
    },
];

/// Main entry point.
fn main() {
    let args = parse_args();
    let (file, options) = args.unwrap_or_else(|err| {
        eprintln!("{}", err);
        std::process::exit(1);
    });
    if options.contains_key(&'h') {
        print_help();
        return;
    }
    if options.contains_key(&'v') {
        print_version();
        return;
    }

    // check arguments
    if file.is_empty() {
        eprintln!("Input file not specified");
        std::process::exit(1);
    }

    // load config
    Config::load();

    // initial cursor position
    let offset = if let Some(opt) = options.get(&'o') {
        let mut text = opt.clone();
        let radix = if text.starts_with("0x") {
            text = text[2..].to_string();
            16
        } else if text.to_lowercase().chars().any(|c| matches!(c, 'a'..='f')) {
            16
        } else {
            10
        };
        u64::from_str_radix(&text, radix).unwrap_or_else(|err| {
            eprintln!("Invalid offset value: {}, {}", opt, err);
            std::process::exit(1);
        })
    } else if let Some(hist) = History::new().get_last_pos(&file) {
        hist
    } else {
        0
    };

    // install custom panic hook to close curses before print error info
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        Curses::close();
        default_panic(info);
    }));

    let cui = Box::new(Curses::new());
    println!("\x1b]0;XVI: {}\x07", file);
    let mut editor = match Editor::new(cui, &file) {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("{}: {}", err, &file);
            std::process::exit(1);
        }
    };
    editor.run(offset);
}

/// Parse command line arguments.
fn parse_args() -> Result<(String, HashMap<char, String>), String> {
    let mut options = HashMap::new();

    let args: Vec<String> = std::env::args().collect();
    let mut it = args.iter().enumerate();
    it.next(); // skip self file name
    let mut last: usize = 1;
    while let Some((argn, argv)) = it.next() {
        if argv == "--" {
            last = argn + 1;
            break;
        }
        if !argv.starts_with('-') {
            last = argn;
            break;
        }
        let mut valid = false;
        for opt in ARGS {
            if *argv == format!("-{}", opt.short) || *argv == format!("--{}", opt.long) {
                if let Some(name) = opt.param {
                    if let Some((argn, argv)) = it.next() {
                        options.insert(opt.short, argv.clone());
                        last = argn + 1;
                    } else {
                        return Err(format!("Argument {} must be specified with {}", argv, name));
                    }
                } else {
                    options.insert(opt.short, "".to_string());
                }

                valid = true;
                break;
            }
        }
        if !valid {
            return Err(format!("Invalid argument: {}", argv));
        }
    }

    // file to open
    let file = match args.get(last) {
        Some(f) => f.clone(),
        None => String::new(),
    };

    // remaining (unexpected) argument
    if let Some(unexpected) = args.get(last + 1) {
        return Err(format!("Unexpected argument: {}", unexpected));
    }

    Ok((file, options))
}

/// Print program version.
fn print_version() {
    println!(
        "XVI - hexadecimal editor ver.{}.",
        env!("CARGO_PKG_VERSION")
    );
}

/// Print usage info.
fn print_help() {
    print_version();
    println!("Usage: xvi [OPTION...] FILE");
    for arg in ARGS {
        let params = format!(
            "-{}, --{} {}",
            arg.short,
            arg.long,
            if let Some(p) = arg.param { p } else { "" }
        );
        println!("  {:<24} {}", params, arg.help);
    }
}
