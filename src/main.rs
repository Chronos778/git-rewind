mod ai;
mod config;
mod git;
mod provider;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Rewind - An AI-powered CLI tool that instantly tells you where you left off.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "EXAMPLES:\n  rewind                 Analyze repo, print a brief, and save to .rewind-brief.md\n  rewind -s              Generate a very short 2-sentence summary\n  rewind commit          Generate a commit message based on staged changes\n  rewind ask \"query\"     Ask a specific question about the repository\n"
)]
struct Args {
    /// Print raw repository state without AI analysis
    #[arg(short, long)]
    dry_run: bool,
    
    /// Output the analysis in raw JSON format
    #[arg(short, long)]
    json: bool,

    /// Generate a very short summary (2-sentence maximum)
    #[arg(short, long)]
    short: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Update rewind to the latest version from GitHub
    Update,
    /// Uninstall rewind and remove all configuration
    Uninstall,
    /// Configure API keys
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Ask a question about the repository
    Ask { query: String },
    /// Generate a commit message based on your diff
    Commit,
    /// Estimate the token count for your repository changes
    Estimate,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Set an API key (omit the key to enter it securely via hidden prompt)
    Set {
        /// The provider to set the key for (groq, gemini, openai)
        provider: String,
        /// The API key (optional — omit to enter securely without exposing in shell history)
        key: Option<String>,
    },
    /// Set a custom model for a provider (e.g. if a default model is decommissioned)
    Model {
        /// The provider (groq, gemini, openai)
        provider: String,
        /// The model name
        model: String,
    },
    /// Clear an API key
    Clear {
        /// The provider to clear the key for (groq, gemini, openai)
        provider: String,
    },
    /// Show configured API keys (redacted) and custom models
    Show,
}

fn make_spinner(message: &str) -> Result<ProgressBar> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")?,
    );
    pb.set_message(message.to_string());
    Ok(pb)
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
        Some(Commands::Uninstall) => {
            uninstall_binary()?;
            return Ok(());
        }
        Some(Commands::Config { action }) => {
            config::handle_config_command(action)?;
            return Ok(());
        }
        Some(Commands::Estimate) => {
            let repo_state = git::get_repo_state()?;
            let prompt = ai::build_user_prompt(&repo_state);
            let chars = prompt.len();
            let words = prompt.split_whitespace().count();
            let approx_tokens = (chars / 4 + (words * 13 / 10)) / 2;
            println!("{} Approximate Tokens: ~{}", "[ESTIMATE]".green().bold(), approx_tokens);
            println!("{} Characters: {} | Words: {}", "[ESTIMATE]".green().bold(), chars, words);
            println!("{} This is a rough estimate. Actual count varies by model and tokenizer.", "[NOTE]".bright_black());
            return Ok(());
        }
        Some(Commands::Commit) => {
            let repo_state = git::get_repo_state()?;
            config::ensure_configured()?;
            let pb = make_spinner("Generating commit message...")?;
            let msg = ai::generate_commit_message(&repo_state).await?;
            pb.finish_and_clear();
            println!("\n{}\n", msg);
            return Ok(());
        }
        Some(Commands::Ask { query }) => {
            let repo_state = git::get_repo_state()?;
            config::ensure_configured()?;
            let pb = make_spinner("Thinking...")?;
            let answer = ai::ask_question_streaming(&repo_state, &query, move || {
                pb.finish_and_clear();
                println!();
            }).await?;
            println!("\n");
            let _ = answer; // response already printed via streaming
            return Ok(());
        }
        None => {}
    }

    // 1. Check if git repo and get state
    let repo_state = git::get_repo_state()?;

    if args.dry_run {
        println!("{}", "[ RAW REPOSITORY STATE ]".yellow().bold());
        println!("{}: {}", "Branch".green(), repo_state.branch);
        println!("{}:\n{}", "Status".green(), repo_state.status);
        println!("{}:\n{}", "Log".green(), repo_state.log);
        println!("{}:\n{}", "Diff (Cached)".green(), repo_state.diff_cached);
        println!("{}:\n{}", "Diff".green(), repo_state.diff);
        return Ok(());
    }

    // 2. Ensure API keys are configured before starting the loading spinner
    config::ensure_configured()?;

    // 3. Fetch AI summary
    if args.short || args.json {
        // Non-streaming mode for --short and --json (need the full response before output)
        let pb = make_spinner("Analyzing repository and generating brief...")?;
        let summary = ai::analyze_repo(&repo_state, args.short, args.json).await;
        pb.finish_and_clear();
        let summary = summary?;

        if args.json {
            let json_output = serde_json::json!({ "brief": summary.trim() });
            println!("{}", json_output);
        } else {
            println!("\n{}", "[ REPOSITORY BRIEF ]".bold());
            println!("{}\n", "─".repeat(60).bright_black());
            println!("{}", summary);
            println!("\n{}", "─".repeat(60).bright_black());
        }

        save_brief(&summary);
    } else {
        // Streaming mode — tokens print live as they arrive
        let pb = make_spinner("Analyzing repository and generating brief...")?;
        let summary = ai::analyze_repo_streaming(&repo_state, move || {
            pb.finish_and_clear();
            println!("\n{}", "[ REPOSITORY BRIEF ]".bold());
            println!("{}", "─".repeat(60).bright_black());
        }).await?;
        println!("\n{}", "─".repeat(60).bright_black());

        save_brief(&summary);
    }

    Ok(())
}

/// Save the brief to `.rewind-brief.md` and add it to `.gitignore`.
fn save_brief(summary: &str) {
    let brief_filename = ".rewind-brief.md";
    let Ok(cwd) = std::env::current_dir() else { return };
    let brief_path = cwd.join(brief_filename);

    let file_content = format!(
        "# Rewind Brief\n\n{}\n\n*(Generated by `rewind`)*\n",
        summary
    );

    match std::fs::write(&brief_path, file_content) {
        Ok(()) => {
            println!("{} Brief automatically saved to: {}", "[INFO]".cyan(), brief_filename.bold());

            // Add to gitignore if it exists and isn't already there
            let gitignore_path = cwd.join(".gitignore");
            if gitignore_path.exists() {
                if let Ok(gitignore_content) = std::fs::read_to_string(&gitignore_path) {
                    if !gitignore_content.contains(brief_filename) {
                        use std::io::Write;
                        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&gitignore_path) {
                            let _ = writeln!(file, "\n# Rewind\n{}", brief_filename);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{} Failed to save brief to {}: {}", "[WARN]".yellow(), brief_filename, e);
        }
    }
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
        println!("{} Updated successfully to version {}.", "[SUCCESS]".green(), status.version());
    } else {
        println!("{} You are already running the latest version ({}).", "[INFO]".cyan(), status.version());
    }

    Ok(())
}

fn uninstall_binary() -> Result<()> {
    println!("{}", "Uninstalling Rewind...".bold());

    // 1. Remove config
    config::clear_all_data();
    println!("{} Removed configuration and local data.", "[SUCCESS]".green());

    // 2. Remove binary
    let exe = std::env::current_exe()?;

    #[cfg(target_family = "unix")]
    {
        if std::fs::remove_file(&exe).is_ok() {
            println!("{} Removed executable.", "[SUCCESS]".green());
        } else {
            println!("{} Please remove the executable manually: rm \"{}\"", "[INFO]".cyan(), exe.display());
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows holds a lock on the currently running executable, so we just instruct the user.
        println!("{} To complete uninstallation, please delete the executable manually by running:", "[INFO]".cyan());
        println!("    del \"{}\"", exe.display());
    }

    Ok(())
}
