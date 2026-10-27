// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ingo Ruhnke <grumbel@gmail.com>

use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let version_path = format!("{}/VERSION", manifest_dir);
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let base = fs::read_to_string(&version_path)
        .unwrap_or_else(|_| "0.0.0-dev".into())
        .trim()
        .to_string();

    let full = if base.contains("-dev") {
        let rev_count = git(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "0".into());
        let short = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
        let dirty = git(&["status", "--porcelain"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let dirty_suffix = if dirty { "-dirty" } else { "" };
        format!("{base}.{rev_count}+g{short}{dirty_suffix}")
    } else {
        base
    };

    println!("cargo:rustc-env=FERNKBD_VERSION={full}");
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
