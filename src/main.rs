mod ai;
mod git;

use anyhow::Result;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Rewind - An AI-powered CLI tool that instantly tells you where you left off.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Print raw repository state without AI analysis
    #[arg(short, long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let args = Args::parse();

    // 1. Check if git repo and get state
    let repo_state = git::get_repo_state()?;

    if args.dry_run {
        println!("{}", "--- RAW REPOSITORY STATE ---".yellow().bold());
        println!("{}: {}", "Branch".green(), repo_state.branch);
        println!("{}:\n{}", "Status".green(), repo_state.status);
        println!("{}:\n{}", "Log".green(), repo_state.log);
        println!("{}:\n{}", "Diff (Cached)".green(), repo_state.diff_cached);
        println!("{}:\n{}", "Diff".green(), repo_state.diff);
        return Ok(());
    }

    // 2. Fetch AI summary with a spinner
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")?,
    );
    pb.set_message("Analyzing repository and generating brief...");

    let summary = ai::analyze_repo(&repo_state).await;

    // Clear the spinner
    pb.finish_and_clear();

    let summary = summary?;

    // 3. Output result
    println!("\n{}", " REPOSITORY BRIEF ".bold());
    println!("{}\n", "─".repeat(60).bright_black());
    println!("{}", summary);
    println!("\n{}", "─".repeat(60).bright_black());

    Ok(())
}
