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
            let mut ver = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_string();
            if ver.starts_with('v') {
                ver.remove(0);
            }
            let components: Vec<&str> = ver.split('-').collect();
            if components.len() > 2 {
                ver = format!(
                    "{}.{}-{}",
                    components[0],
                    components[1],
                    components[2..].join("-")
                );
            }
            println!("cargo:rustc-env=CARGO_PKG_VERSION={}", ver);
        }
    }
}
