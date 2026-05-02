use anyhow::{Context, Result};
use std::process::Command;
use std::fs;

pub struct RepoState {
    pub branch: String,
    pub status: String,
    pub log: String,
    pub diff: String,
    pub diff_cached: String,
}

fn get_exclusions() -> Vec<String> {
    let mut exclusions = Vec::new();
    if let Ok(content) = fs::read_to_string(".rewindignore") {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                exclusions.push(format!(":(exclude){}", trimmed));
            }
        }
    }
    exclusions
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

    let exclusions = get_exclusions();
    let mut diff_args_uncached = vec!["diff", "--stat", "-p", "--", "."];
    let mut diff_args_cached = vec!["diff", "--cached", "--stat", "-p", "--", "."];
    
    let exclusion_refs: Vec<&str> = exclusions.iter().map(|s| s.as_str()).collect();
    diff_args_uncached.extend(&exclusion_refs);
    diff_args_cached.extend(&exclusion_refs);

    let branch = run_git(&["branch", "--show-current"])?;
    let status = run_git(&["status", "--short", "--branch"])?;
    let log = run_git(&["log", "-n", "5", "--oneline"])?;
    let diff = run_git(&diff_args_uncached)?; 
    let diff_cached = run_git(&diff_args_cached)?;

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

    // Security/Robustness: Check if command actually succeeded, otherwise log might silently fail
    // In new repos, git log returns 128 "no commits yet", which we can treat as empty text.
    if !output.status.success() && args[0] != "log" {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
             // We don't necessarily want to panic, but warn or ignore depending on command.
             // For safety, we keep it proceeding but the stderr could be useful in dry-run
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout.trim().to_string())
}
