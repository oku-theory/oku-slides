//! The binary links `FoundationModels` weakly and targets macOS 12, so it starts before macOS 26.
#![cfg(target_os = "macos")]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::process::Command;

use speculoos::prelude::*;

/// The load commands of the dev launcher, as `otool -l` prints them.
fn load_commands() -> String {
    let out = Command::new("otool")
        .arg("-l")
        .arg(env!("CARGO_BIN_EXE_oku-slides-local-agent-dev"))
        .output()
        .unwrap_or_else(|e| panic!("otool: {e}"));
    assert_that!(out.status.success()).is_true();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The `cmd` line of the load command whose `name` is the `FoundationModels` framework.
fn foundation_models_command(commands: &str) -> Option<String> {
    let lines: Vec<&str> = commands.lines().map(str::trim).collect();
    let at = lines.iter().position(|l| l.starts_with("name ") && l.contains("FoundationModels.framework"))?;
    lines[..at].iter().rev().find(|l| l.starts_with("cmd ")).map(|l| (*l).to_owned())
}

#[test]
fn dev_binary_foundation_models_is_a_weak_dylib() {
    let commands = load_commands();
    assert_that!(foundation_models_command(&commands)).is_equal_to(Some("cmd LC_LOAD_WEAK_DYLIB".to_owned()));
}

#[test]
fn dev_binary_minimum_macos_is_12() {
    let commands = load_commands();
    assert_that!(commands.lines().map(str::trim).any(|l| l == "minos 12.0")).is_true();
}
