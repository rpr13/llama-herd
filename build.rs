use std::process::Command;

fn main() {
    let mut version = format!("v{}", env!("CARGO_PKG_VERSION"));

    // Check if git is available and we are inside a work tree
    let is_git = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if is_git {
        let git_ver = Command::new("git")
            .args([
                "describe",
                "--tags",
                "--always",
                "--dirty",
                "--exclude",
                "pre-release",
            ])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(git_ver) = git_ver {
            let trimmed = git_ver.trim();
            if !trimmed.is_empty() {
                version = trimmed.to_string();
            }
        }

        // Rerun build.rs if git head or refs change
        println!("cargo:rerun-if-changed=.git/HEAD");

        // Rerun if branch or tags change
        let git_dir = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(git_dir_str) = git_dir {
            let git_dir = git_dir_str.trim();
            println!("cargo:rerun-if-changed={}/refs/heads", git_dir);
            println!("cargo:rerun-if-changed={}/refs/tags", git_dir);
        }
    }

    println!("cargo:rustc-env=APP_VERSION={}", version);
}
