use std::process::Command;

fn main() {
    // The release workflow checks out a single commit without tags, so it passes
    // the commit in explicitly; locally the working tree is the only source.
    println!("cargo:rerun-if-env-changed=SAS_LAUNCHER_COMMIT");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");

    let commit = std::env::var("SAS_LAUNCHER_COMMIT")
        .ok()
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .map(|sha| sha.chars().take(7).collect())
        .or_else(git_commit)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SAS_LAUNCHER_COMMIT={commit}");

    tauri_build::build();
}

/// The commit this build came from, marked when tracked files no longer match it.
fn git_commit() -> Option<String> {
    let mut commit = git(&["rev-parse", "--short=7", "HEAD"])?;
    if !git(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        commit.push_str("-dirty");
    }
    Some(commit)
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir("..")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
