mod ai;
mod git;

use anyhow::Result;
use clap::{Parser, Subcommand};
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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Update rewind to the latest version from GitHub
    Update,
    /// Configure API keys
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Set an API key
    Set {
        /// The provider to set the key for (groq, gemini, openai)
        provider: String,
        /// The API key
        key: String,
    },
    /// Clear an API key
    Clear {
        /// The provider to clear the key for (groq, gemini, openai)
        provider: String,
    },
    /// Show configured API keys (redacted)
    Show,
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let args = Args::parse();

    match args.command {
        Some(Commands::Update) => {
            update_binary()?;
            return Ok(());
        }
        Some(Commands::Config { action }) => {
            ai::handle_config_command(action)?;
            return Ok(());
        }
        None => {}
    }

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

    // 2. Ensure API keys are configured before starting the loading spinner
    ai::ensure_configured()?;

    // 3. Fetch AI summary with a spinner
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

fn update_binary() -> Result<()> {
    println!("Checking for updates...");
    let status = self_update::backends::github::Update::configure()
        .repo_owner("Chronos778")
        .repo_name("git-rewind")
        .bin_name("rewind")
        .show_download_progress(true)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update()?;
    
    if status.updated() {
        println!("{} Updated successfully to version {}!", "✅".green(), status.version());
    } else {
        println!("{} You are already running the latest version ({}).", "✅".green(), status.version());
    }
    
    Ok(())
}
