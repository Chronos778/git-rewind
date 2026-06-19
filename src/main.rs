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
    #[arg(short, long, global = true)]
    json: bool,

    /// Generate a very short summary (2-sentence maximum)
    #[arg(short, long)]
    short: bool,

    /// Show diagnostic information (provider, API base, model)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Do not save the brief to .rewind-brief.md
    #[arg(long)]
    no_save: bool,

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
    /// View the last generated brief
    History,
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
    /// Set a custom system prompt (omit to clear)
    SystemPrompt {
        /// The custom system prompt to use
        prompt: Option<String>,
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

            if args.json {
                let json_output = serde_json::json!({
                    "approx_tokens": approx_tokens,
                    "characters": chars,
                    "words": words
                });
                println!("{}", json_output);
            } else {
                println!(
                    "{} Approximate Tokens: ~{}",
                    "[ESTIMATE]".green().bold(),
                    approx_tokens
                );
                println!(
                    "{} Characters: {} | Words: {}",
                    "[ESTIMATE]".green().bold(),
                    chars,
                    words
                );
                println!(
                    "{} This is a rough estimate. Actual context window and token usage vary by model.",
                    "[NOTE]".bright_black()
                );
                println!(
                    "{} To test specific tokenizers, try: https://tiktokenizer.vercel.app",
                    "[TIP]".bright_black()
                );
            }
            return Ok(());
        }
        Some(Commands::History) => {
            let repo_state = git::get_repo_state()?;
            let brief_filename = ".rewind-brief.md";
            let brief_path = std::path::PathBuf::from(&repo_state.root).join(brief_filename);

            if brief_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&brief_path) {
                    if args.json {
                        let json_output = serde_json::json!({ "brief": content.trim() });
                        println!("{}", json_output);
                    } else {
                        println!("{}", content);
                    }
                } else {
                    anyhow::bail!("Failed to read {}", brief_filename);
                }
            } else {
                if args.json {
                    let json_output = serde_json::json!({ "error": "No previous brief found." });
                    println!("{}", json_output);
                } else {
                    println!(
                        "{} No previous brief found. Run `rewind` to generate one.",
                        "[INFO]".cyan()
                    );
                }
            }
            return Ok(());
        }
        Some(Commands::Commit) => {
            let repo_state = git::get_repo_state()?;
            config::ensure_configured()?;
            if args.verbose {
                print_diagnostics();
            }
            let pb = make_spinner("Generating commit message...")?;
            let result = ai::generate_commit_message(&repo_state).await;
            pb.finish_and_clear();
            let (msg, usage) = result?;

            if args.json {
                let json_output = serde_json::json!({ "commit_message": msg.trim() });
                println!("{}", json_output);
            } else {
                println!("\n{}\n", msg);
            }

            if let (true, Some((p, c))) = (args.verbose, usage) {
                let telemetry = format!(
                    "{} Prompt: {} | Completion: {} | Total: {}",
                    "[TELEMETRY]".bright_black(),
                    p,
                    c,
                    p + c
                );
                if args.json {
                    eprintln!("{}", telemetry);
                } else {
                    println!("{}", telemetry);
                }
            }
            return Ok(());
        }
        Some(Commands::Ask { query }) => {
            let repo_state = git::get_repo_state()?;
            config::ensure_configured()?;
            if args.verbose {
                print_diagnostics();
            }

            let usage;
            if args.json {
                let pb = make_spinner("Thinking...")?;
                let result = ai::ask_question(&repo_state, &query).await;
                pb.finish_and_clear();
                let (ans, useg) = result?;
                usage = useg;
                let json_output = serde_json::json!({ "answer": ans.trim() });
                println!("{}", json_output);
            } else {
                let pb = make_spinner("Thinking...")?;
                let pb_clone = pb.clone();
                let result = ai::ask_question_streaming(&repo_state, &query, move || {
                    pb_clone.finish_and_clear();
                    println!();
                })
                .await;
                pb.finish_and_clear();
                let (_ans, useg) = result?;
                usage = useg;
                println!("\n");
            }

            if let (true, Some((p, c))) = (args.verbose, usage) {
                let telemetry = format!(
                    "{} Prompt: {} | Completion: {} | Total: {}\n",
                    "[TELEMETRY]".bright_black(),
                    p,
                    c,
                    p + c
                );
                if args.json {
                    eprintln!("{}", telemetry);
                } else {
                    println!("{}", telemetry);
                }
            }
            return Ok(());
        }
        None => {}
    }

    // 1. Check if git repo and get state
    let repo_state = git::get_repo_state()?;

    if args.dry_run {
        let prompt = ai::build_user_prompt(&repo_state);
        if args.json {
            let json_output = serde_json::json!({ "prompt": prompt });
            println!("{}", json_output);
        } else {
            println!("{}", "[ DRY RUN: RAW LLM PROMPT ]".yellow().bold());
            println!("{}", "─".repeat(60).bright_black());
            println!("{}", prompt);
            println!("{}", "─".repeat(60).bright_black());
        }
        return Ok(());
    }

    // 2. Ensure API keys are configured before starting the loading spinner
    config::ensure_configured()?;
    if args.verbose {
        print_diagnostics();
    }

    // 3. Fetch AI summary
    if args.short || args.json {
        // Non-streaming mode for --short and --json (need the full response before output)
        let pb = make_spinner("Analyzing repository and generating brief...")?;
        let result = ai::analyze_repo(&repo_state, args.short, args.json).await;
        pb.finish_and_clear();
        let (summary, usage) = result?;

        if args.json {
            let json_output = serde_json::json!({ "brief": summary.trim() });
            println!("{}", json_output);
        } else {
            println!("\n{}", "[ REPOSITORY BRIEF ]".bold());
            println!("{}\n", "─".repeat(60).bright_black());
            println!("{}", summary);
            println!("\n{}", "─".repeat(60).bright_black());
        }

        if let (true, Some((p, c))) = (args.verbose, usage) {
            let msg = format!(
                "{} Prompt: {} | Completion: {} | Total: {}",
                "[TELEMETRY]".bright_black(),
                p,
                c,
                p + c
            );
            if args.json {
                eprintln!("{}", msg);
            } else {
                println!("{}", msg);
            }
        }

        if !args.no_save {
            save_brief(&summary, &repo_state.root);
        }
    } else {
        // Streaming mode — tokens print live as they arrive
        let pb = make_spinner("Analyzing repository and generating brief...")?;
        let pb_clone = pb.clone();
        let result = ai::analyze_repo_streaming(&repo_state, move || {
            pb_clone.finish_and_clear();
            println!("\n{}", "[ REPOSITORY BRIEF ]".bold());
            println!("{}", "─".repeat(60).bright_black());
        })
        .await;
        pb.finish_and_clear();
        let (summary, usage) = result?;
        println!("\n{}", "─".repeat(60).bright_black());

        if let (true, Some((p, c))) = (args.verbose, usage) {
            println!(
                "{} Prompt: {} | Completion: {} | Total: {}",
                "[TELEMETRY]".bright_black(),
                p,
                c,
                p + c
            );
        }

        if !args.no_save {
            save_brief(&summary, &repo_state.root);
        }
    }

    Ok(())
}

/// Print diagnostic information about the configured provider, API base, and model.
fn print_diagnostics() {
    let cfg = config::load_config().unwrap_or_default();

    let resolved = provider::Provider::all().iter().find_map(|&p| {
        if std::env::var(p.env_key_name()).is_ok() {
            Some((p, "environment variable"))
        } else if cfg.get_api_key(p).is_some() {
            Some((p, "config file"))
        } else {
            None
        }
    });

    if let Some((p, source)) = resolved {
        let api_base =
            std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| p.default_api_base().to_string());
        let model = std::env::var("OPENAI_MODEL")
            .ok()
            .or_else(|| cfg.get_model(p).cloned())
            .or_else(|| cfg.get_cached_model(p, &api_base))
            .unwrap_or_else(|| format!("{} (auto-discover)", p.default_model()));

        eprintln!(
            "{} Provider: {} (from {})",
            "[VERBOSE]".bright_black(),
            p.display_name(),
            source
        );
        eprintln!("{} API Base: {}", "[VERBOSE]".bright_black(), api_base);
        eprintln!("{} Model: {}", "[VERBOSE]".bright_black(), model);
    } else {
        eprintln!("{} No provider configured", "[VERBOSE]".bright_black());
    }
}

/// Save the brief to `.rewind-brief.md` at the repo root and add it to the root `.gitignore`.
/// Using the repo root (not CWD) ensures correct placement regardless of which
/// subdirectory the user ran `rewind` from.
fn save_brief(summary: &str, repo_root: &str) {
    let brief_filename = ".rewind-brief.md";
    let repo_path = std::path::PathBuf::from(repo_root);
    let brief_path = repo_path.join(brief_filename);

    let file_content = format!(
        "# Rewind Brief\n\n{}\n\n*(Generated by `rewind`)*\n",
        summary
    );

    match std::fs::write(&brief_path, file_content) {
        Ok(()) => {
            eprintln!(
                "{} Brief automatically saved to: {}",
                "[INFO]".cyan(),
                brief_path.display().to_string().bold()
            );

            // Add to .gitignore at the repo root if not already present
            let gitignore_path = repo_path.join(".gitignore");
            let mut should_append = true;
            if gitignore_path.exists() {
                if let Ok(gitignore_content) = std::fs::read_to_string(&gitignore_path) {
                    if gitignore_content.contains(brief_filename) {
                        should_append = false;
                    }
                }
            }

            if should_append {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&gitignore_path)
                {
                    let _ = writeln!(file, "\n# Rewind\n{}", brief_filename);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{} Failed to save brief to {}: {}",
                "[WARN]".yellow(),
                brief_filename,
                e
            );
        }
    }
}

fn update_binary() -> Result<()> {
    println!("Checking for updates...");

    let target = self_update::get_target();
    let archive_ext = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };
    let identifier = format!("{}{}", target, archive_ext);

    let status = self_update::backends::github::Update::configure()
        .repo_owner("Chronos778")
        .repo_name("git-rewind")
        .bin_name("rewind")
        .identifier(&identifier)
        .show_download_progress(true)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update()?;

    if status.updated() {
        println!(
            "{} Updated successfully to version {}.",
            "[SUCCESS]".green(),
            status.version()
        );
    } else {
        println!(
            "{} You are already running the latest version ({}).",
            "[INFO]".cyan(),
            status.version()
        );
    }

    Ok(())
}

fn uninstall_binary() -> Result<()> {
    println!("{}", "Uninstalling Rewind...".bold());

    // 1. Remove config
    config::clear_all_data();
    println!(
        "{} Removed configuration and local data.",
        "[SUCCESS]".green()
    );

    // 2. Remove binary
    let exe = std::env::current_exe()?;

    #[cfg(target_family = "unix")]
    {
        if std::fs::remove_file(&exe).is_ok() {
            println!("{} Removed executable.", "[SUCCESS]".green());
        } else {
            println!(
                "{} Please remove the executable manually: rm \"{}\"",
                "[INFO]".cyan(),
                exe.display()
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows holds a lock on the currently running executable, so we just instruct the user.
        println!(
            "{} To complete uninstallation, please delete the executable manually by running:",
            "[INFO]".cyan()
        );
        println!("    del \"{}\"", exe.display());
    }

    Ok(())
}
