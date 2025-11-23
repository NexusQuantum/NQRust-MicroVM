//! Installer logic modules.
//!
//! Each module handles a specific phase of the installation process.

#![allow(dead_code)]

pub mod build;
pub mod config;
pub mod database;
pub mod deps;
pub mod executor;
pub mod kvm;
pub mod network;
pub mod preflight;
pub mod services;
pub mod verify;

use std::process::{Command, Output};

use anyhow::{anyhow, Result};

/// Execute a shell command and return the output
pub fn run_command(cmd: &str, args: &[&str]) -> Result<Output> {
    Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow!("Failed to execute {}: {}", cmd, e))
}

/// Execute a shell command with sudo
pub fn run_sudo(cmd: &str, args: &[&str]) -> Result<Output> {
    let mut sudo_args = vec![cmd];
    sudo_args.extend(args);
    run_command("sudo", &sudo_args)
}

/// Check if a command exists
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if running as root
pub fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

/// Get the current username
pub fn current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
