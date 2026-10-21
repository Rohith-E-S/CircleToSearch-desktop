//! Stamps the binary with the version and commit it was built from.
//!
//! `CIRCLE_TO_SEARCH_VERSION` and `CIRCLE_TO_SEARCH_COMMIT` are set by CI.
//! When they are absent the version falls back to `Cargo.toml` and the commit
//! is read from the local git checkout, so local builds are still identifiable.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=CIRCLE_TO_SEARCH_VERSION");
    println!("cargo:rerun-if-env-changed=CIRCLE_TO_SEARCH_COMMIT");

    let version = std::env::var("CIRCLE_TO_SEARCH_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CIRCLE_TO_SEARCH_VERSION={version}");

    let commit = std::env::var("CIRCLE_TO_SEARCH_COMMIT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(git_commit)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=CIRCLE_TO_SEARCH_COMMIT={commit}");
}

fn git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if commit.is_empty() {
        None
    } else {
        Some(commit)
    }
}
