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
    
    let default_excludes = [
        ".env", ".env.*", "*.pem", "*.key", "*.crt", "secrets.json", "id_rsa", "id_dsa", "*.p12", "*.pfx", "config.json", "Credentials.toml", "credentials.json"
    ];
    for ext in default_excludes {
        exclusions.push(format!(":(exclude){}", ext));
    }

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
    let diff = run_git_limited(&diff_args_uncached, 15000)?; 
    let diff_cached = run_git_limited(&diff_args_cached, 15000)?;

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
        // Silently ignore non-zero exits for commands like git branch in empty repos
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout.trim().to_string())
}

fn run_git_limited(args: &[&str], limit: usize) -> Result<String> {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(format!("Failed to run `git {:?}`", args))?;

    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut buffer = Vec::new();
    stdout.take(limit as u64).read_to_end(&mut buffer)?;
    
    // Clean up to prevent zombie processes and stop further execution
    let _ = child.kill();
    let _ = child.wait();

    let text = String::from_utf8_lossy(&buffer).to_string();
    Ok(text.trim().to_string())
}
