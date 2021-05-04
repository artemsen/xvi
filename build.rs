// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::process::Command;

fn main() {
    // set version from git
    if let Ok(output) = Command::new("git")
        .args(&["describe", "--always", "--tags", "--dirty"])
        .output()
    {
        if output.status.success() {
            let ver = std::str::from_utf8(&output.stdout[1..]).unwrap().trim();
            println!("cargo:rustc-env=CARGO_PKG_VERSION={}", ver);
        }
    }
}
