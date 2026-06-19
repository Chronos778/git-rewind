use anyhow::{Context, Result};
use git2::{DiffFormat, DiffOptions, Repository, StatusOptions};
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum number of bytes to read from a git diff output.
pub const MAX_GIT_DIFF_BYTES: usize = 15_000;

/// Maximum number of bytes to include in the status output.
pub const MAX_GIT_STATUS_BYTES: usize = 10_000;

pub struct RepoState {
    pub branch: String,
    pub status: String,
    pub log: String,
    pub diff: String,
    pub diff_cached: String,
    /// Absolute path to the repository root
    pub root: String,
}

fn get_exclusions(repo_root: &Path) -> Vec<String> {
    let mut exclusions = Vec::new();

    let default_excludes = [
        ".env",
        ".env.*",
        "*.pem",
        "*.key",
        "*.crt",
        "secrets.json",
        "id_rsa",
        "id_dsa",
        "*.p12",
        "*.pfx",
        "config.json",
        "Credentials.toml",
        "credentials.json",
        // Lockfiles (massive and useless for logical context)
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Gemfile.lock",
        "poetry.lock",
        // Minified assets
        "*.min.js",
        "*.min.css",
        // Rewind brief itself
        ".rewind-brief.md",
    ];
    for ext in default_excludes {
        exclusions.push(format!(":!{}", ext));
    }

    // Helper closure to read ignore files
    let mut add_ignores = |path: PathBuf| {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    exclusions.push(format!(":!{}", trimmed));
                }
            }
        }
    };

    // 1. Global exclusions (~/.rewindignore)
    if let Some(home) = directories::UserDirs::new() {
        add_ignores(home.home_dir().join(".rewindignore"));
    }

    // 2. Repository exclusions (relative to the repo root)
    add_ignores(repo_root.join(".rewindignore"));

    exclusions
}

pub fn get_repo_state() -> Result<RepoState> {
    let repo = Repository::discover(".")
        .context("Not a git repository (or any of the parent directories).")?;
    let repo_root_path = repo
        .workdir()
        .context("Bare repositories are not supported.")?;
    // canonicalize path for cross-platform robustness
    let repo_root_path =
        dunce::canonicalize(repo_root_path).unwrap_or_else(|_| repo_root_path.to_path_buf());
    let root_str = repo_root_path.to_string_lossy().to_string();

    let exclusions = get_exclusions(&repo_root_path);

    // 1. Branch
    let branch = if let Ok(head) = repo.head() {
        if head.is_branch() {
            head.shorthand().unwrap_or("").to_string()
        } else {
            "HEAD (detached)".to_string()
        }
    } else {
        "No commits yet".to_string()
    };

    // 2. Log
    let mut log = String::new();
    if let Ok(mut revwalk) = repo.revwalk() {
        if revwalk.push_head().is_ok() {
            for oid in revwalk.take(5).flatten() {
                if let Ok(commit) = repo.find_commit(oid) {
                    let id = &commit.id().to_string()[..7];
                    let summary_bytes = commit.summary_bytes().unwrap_or(b"");
                    let summary = String::from_utf8_lossy(summary_bytes);
                    log.push_str(&format!("{} {}\n", id, summary));
                }
            }
        }
    }

    // 3. Status
    let mut status_str = String::new();
    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .recurse_untracked_dirs(true);
    let mut status_truncated = false;
    if let Ok(statuses) = repo.statuses(Some(&mut status_opts)) {
        for entry in statuses.iter() {
            if status_str.len() > MAX_GIT_STATUS_BYTES {
                status_truncated = true;
                break;
            }

            let path = entry.path().unwrap_or("");
            let s = entry.status();

            let mut x = ' ';
            let mut y = ' ';

            if s.contains(git2::Status::INDEX_NEW) {
                x = 'A';
            } else if s.contains(git2::Status::INDEX_MODIFIED) {
                x = 'M';
            } else if s.contains(git2::Status::INDEX_DELETED) {
                x = 'D';
            } else if s.contains(git2::Status::INDEX_RENAMED) {
                x = 'R';
            } else if s.contains(git2::Status::INDEX_TYPECHANGE) {
                x = 'T';
            }

            if s.contains(git2::Status::WT_NEW) {
                y = '?';
                x = '?';
            } else if s.contains(git2::Status::WT_MODIFIED) {
                y = 'M';
            } else if s.contains(git2::Status::WT_DELETED) {
                y = 'D';
            } else if s.contains(git2::Status::WT_RENAMED) {
                y = 'R';
            } else if s.contains(git2::Status::WT_TYPECHANGE) {
                y = 'T';
            }
            if s.contains(git2::Status::CONFLICTED) {
                x = 'U';
                y = 'U';
            }

            status_str.push_str(&format!("{}{} {}\n", x, y, path));
        }
    }
    if status_truncated {
        status_str.push_str("... [Status truncated due to length] ...\n");
    }

    // 4. Diffs
    let mut diff_opts = DiffOptions::new();
    for exc in &exclusions {
        diff_opts.pathspec(exc);
    }
    // We add "*" so it defaults to including everything else.
    // In libgit2 pathspecs, `:!` is the standard for exclusions.
    diff_opts.pathspec("*");
    diff_opts.include_untracked(true);

    let head_tree = repo.head().and_then(|h| h.peel_to_tree()).ok();

    // Diff Cached (Index vs HEAD)
    let diff_cached = match &head_tree {
        Some(tree) => repo.diff_tree_to_index(Some(tree), None, Some(&mut diff_opts)),
        None => repo.diff_tree_to_index(None, None, Some(&mut diff_opts)), // Empty tree if no commits
    }
    .context("Failed to generate cached diff")?;

    // Diff Uncached (Workspace vs Index)
    let diff_uncached = repo
        .diff_index_to_workdir(None, Some(&mut diff_opts))
        .context("Failed to generate workspace diff")?;

    let mut diff_cached_str = String::new();
    let mut diff_cached_truncated = false;
    let _ = diff_cached.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let content = String::from_utf8_lossy(line.content());
        if diff_cached_str.len() + content.len() > MAX_GIT_DIFF_BYTES {
            diff_cached_truncated = true;
            return false;
        }
        let prefix = match line.origin() {
            '+' | '-' | ' ' => line.origin().to_string(),
            _ => String::new(),
        };
        diff_cached_str.push_str(&prefix);
        diff_cached_str.push_str(&content);
        true
    });
    if diff_cached_truncated {
        diff_cached_str.push_str("\n... [Diff truncated due to length] ...\n");
    }

    let mut diff_str = String::new();
    let mut diff_truncated = false;
    let _ = diff_uncached.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let content = String::from_utf8_lossy(line.content());
        if diff_str.len() + content.len() > MAX_GIT_DIFF_BYTES {
            diff_truncated = true;
            return false;
        }
        let prefix = match line.origin() {
            '+' | '-' | ' ' => line.origin().to_string(),
            _ => String::new(),
        };
        diff_str.push_str(&prefix);
        diff_str.push_str(&content);
        true
    });
    if diff_truncated {
        diff_str.push_str("\n... [Diff truncated due to length] ...\n");
    }

    Ok(RepoState {
        branch: branch.trim().to_string(),
        status: status_str.trim().to_string(),
        log: log.trim().to_string(),
        diff: diff_str.trim().to_string(),
        diff_cached: diff_cached_str.trim().to_string(),
        root: root_str,
    })
}
