// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

mod ascii;
mod changes;
mod cmdargs;
mod config;
mod curses;
mod cursor;
mod document;
mod editor;
mod file;
mod history;
mod inifile;
mod page;
mod ui;
mod view;

use cmdargs::CmdLineArgs;
use config::Config;
use curses::Curses;
use editor::Editor;

/// Main entry point.
fn main() {
    // handle command line arguments
    let args = CmdLineArgs::new().unwrap_or_else(|err| {
        eprintln!("{}", err);
        std::process::exit(1);
    });
    if args.help {
        print_version();
        CmdLineArgs::help();
        return;
    }
    if args.version {
        print_version();
        return;
    }
    if args.files.is_empty() {
        eprintln!("Input files not specified");
        std::process::exit(1);
    }

    // install custom panic hook to close curses before printing error info
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        Curses::close();
        default_panic(info);
    }));

    // set window title
    println!("\x1b]0;XVI: {}\x07", args.files[0]);

    let config = Config::load();
    Curses::initialize(&config.colors);

    let mut editor = match Editor::new(&args.files[0], args.offset, &config) {
        Ok(editor) => editor,
        Err(err) => {
            Curses::close();
            eprintln!("{}: {}", err, &args.files[0]);
            std::process::exit(1);
        }
    };
    editor.run();

    Curses::close();
}

/// Print program version.
fn print_version() {
    println!(
        "XVI - hexadecimal editor ver.{}.",
        env!("CARGO_PKG_VERSION")
    );
}
