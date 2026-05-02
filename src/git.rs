use anyhow::{Context, Result};
use std::process::Command;

pub struct RepoState {
    pub branch: String,
    pub status: String,
    pub log: String,
    pub diff: String,
    pub diff_cached: String,
}

pub fn get_repo_state() -> Result<RepoState> {
    // Check if git is installed
    if Command::new("git").arg("--version").output().is_err() {
        anyhow::bail!("Git is not installed or not available on the PATH.");
    }

    // Check if we are in a git repo
    let check = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !check.status.success() {
        anyhow::bail!("Not a git repository (or any of the parent directories).");
    }

    let branch = run_git(&["branch", "--show-current"])?;
    let status = run_git(&["status", "--short", "--branch"])?;
    let log = run_git(&["log", "-n", "5", "--oneline"])?;
    let diff = run_git(&["diff", "--stat", "-p"])?; // Include stat and diff
    let diff_cached = run_git(&["diff", "--cached", "--stat", "-p"])?;

    Ok(RepoState {
        branch,
        status,
        log,
        diff,
        diff_cached,
    })
}

fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context(format!("Failed to run `git {:?}`", args))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout.trim().to_string())
}
