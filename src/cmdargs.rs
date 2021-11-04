// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

/// Command line arguments.
pub struct CmdLineArgs {
    /// Initial cursor offset.
    pub offset: Option<u64>,
    /// Flag to print version info.
    pub version: bool,
    /// Flag to print help.
    pub help: bool,
    /// List of files to open.
    pub files: Vec<String>,
}

impl CmdLineArgs {
    /// Create new instance from current command line arguments.
    pub fn new() -> Result<Self, String> {
        // get command line args and skip self executable file name
        let args: Vec<String> = std::env::args().skip(1).collect();
        CmdLineArgs::parse(args)
    }

    /// Parse command line arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - command line arguments without self file name
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut instance = Self {
            files: Vec::new(),
            offset: None,
            version: false,
            help: false,
        };

        let mut last_index = args.len();
        let mut it = args.iter().enumerate();
        while let Some((index, arg)) = it.next() {
            if !arg.starts_with('-') {
                last_index = index;
                break;
            }
            match arg.as_ref() {
                "-o" | "--offset" => {
                    if let Some((_, text)) = it.next() {
                        let (start, radix) = if text.starts_with("0x") {
                            (2 /* skip 0x */, 16)
                        } else if text.to_lowercase().chars().any(|c| matches!(c, 'a'..='f')) {
                            (0, 16)
                        } else {
                            (0, 10)
                        };
                        if let Ok(offset) = u64::from_str_radix(&text[start..], radix) {
                            instance.offset = Some(offset);
                        } else {
                            return Err(format!("Invalid offset value: {}", text));
                        }
                    } else {
                        return Err("Offset not specified".to_string());
                    }
                }
                "-v" | "--version" => {
                    instance.version = true;
                }
                "-h" | "--help" => {
                    instance.help = true;
                }
                "--" => {
                    last_index = index + 1;
                    break;
                }
                _ => {
                    return Err(format!("Invalid argument: {}", arg));
                }
            };
        }

        if last_index < args.len() {
            instance.files = args.into_iter().skip(last_index).collect();
        }

        Ok(instance)
    }

    /// Print usage info.
    pub fn help() {
        println!("Usage: xvi [OPTION...] FILE...");
        println!("  -o, --offset ADDRESS   Set initial cursor offset");
        println!("  -v, --version          Print version info and exit");
        println!("  -h, --help             Print this help and exit");
    }
}

#[test]
fn test_simple() {
    let args = ["--help".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(args.help);
    assert!(!args.version);
    assert!(args.offset.is_none());
    assert!(args.files.is_empty());

    let args = ["-v".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(!args.help);
    assert!(args.version);
    assert!(args.offset.is_none());
    assert!(args.files.is_empty());

    let args = ["--version".to_string(), "-h".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(args.help);
    assert!(args.version);
    assert!(args.offset.is_none());
    assert!(args.files.is_empty());
}

#[test]
fn test_offset() {
    let args = ["--offset".to_string(), "0x12345678".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(!args.help);
    assert!(!args.version);
    assert!(args.files.is_empty());
    assert_eq!(args.offset, Some(0x12345678));

    let args = ["--offset".to_string(), "1234567a".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(!args.help);
    assert!(!args.version);
    assert!(args.files.is_empty());
    assert_eq!(args.offset, Some(0x1234567a));

    let args = ["-o".to_string(), "12345678".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(!args.help);
    assert!(!args.version);
    assert!(args.files.is_empty());
    assert_eq!(args.offset, Some(12345678));

    let args = ["-o".to_string()];
    assert!(CmdLineArgs::parse(args.to_vec()).is_err());
    let args = ["-o".to_string(), "test".to_string()];
    assert!(CmdLineArgs::parse(args.to_vec()).is_err());
}

#[test]
fn test_files() {
    let args = ["file".to_string()];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert!(!args.help);
    assert!(!args.version);
    assert_eq!(args.files.len(), 1);
    assert_eq!(args.files.get(0), Some(&"file".to_string()));

    let args = [
        "-v".to_string(),
        "--".to_string(),
        "--file1".to_string(),
        "file2".to_string(),
    ];
    let args = CmdLineArgs::parse(args.to_vec()).unwrap();
    assert_eq!(args.files.len(), 2);
    assert_eq!(args.files.get(0), Some(&"--file1".to_string()));
    assert_eq!(args.files.get(1), Some(&"file2".to_string()));
}
