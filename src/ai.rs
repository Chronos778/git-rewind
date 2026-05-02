use crate::git::RepoState;
use anyhow::{Context, Result};
use colored::*;
use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Default)]
struct Config {
    groq_api_key: Option<String>,
    gemini_api_key: Option<String>,
    openai_api_key: Option<String>,
}

fn get_config_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "Rewind", "Rewind").map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}

fn load_config() -> Config {
    if let Some(path) = get_config_path() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                return config;
            }
        }
    }
    Config::default()
}

pub fn handle_config_command(action: crate::ConfigCommands) -> Result<()> {
    let mut config = load_config();

    match action {
        crate::ConfigCommands::Set { provider, key } => {
            match provider.to_lowercase().as_str() {
                "groq" => config.groq_api_key = Some(key.clone()),
                "gemini" => config.gemini_api_key = Some(key.clone()),
                "openai" => config.openai_api_key = Some(key.clone()),
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} API key for {} has been set.", "✅".green(), provider);
        }
        crate::ConfigCommands::Clear { provider } => {
            match provider.to_lowercase().as_str() {
                "groq" => config.groq_api_key = None,
                "gemini" => config.gemini_api_key = None,
                "openai" => config.openai_api_key = None,
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} API key for {} has been cleared.", "✅".green(), provider);
        }
        crate::ConfigCommands::Show => {
            println!("{} \n", "--- Configured API Keys ---".bold());
            if let Some(key) = &config.groq_api_key {
                println!("{}: {}", "Groq".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]));
            } else {
                println!("{}: Not set", "Groq".bright_black());
            }

            if let Some(key) = &config.gemini_api_key {
                println!("{}: {}", "Gemini".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]));
            } else {
                println!("{}: Not set", "Gemini".bright_black());
            }

            if let Some(key) = &config.openai_api_key {
                println!("{}: {}", "OpenAI".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]));
            } else {
                println!("{}: Not set", "OpenAI".bright_black());
            }
        }
    }
    Ok(())
}

fn save_config(config: &Config) -> Result<()> {
    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(config)?;
        fs::write(path, contents)?;
    }
    Ok(())
}

const SYSTEM_PROMPT: &str = "\
You are an AI assistant helping a developer instantly get back into their flow. \
The developer has just opened their terminal and run the `rewind` command. \
You will be provided with the current state of their git repository (branch, status, recent commits, and unstaged/staged diffs). \
Your job is to read this context and tell the developer what they were doing and what they likely need to do next.

Keep it concise, engaging, and direct. \
Don't use lists unless absolutely necessary. \
Focus on the 'why' and 'what' of the changes. \
Do not output markdown code blocks unless it's a specific shell command they should run. \
Make it sound like a helpful colleague bringing them up to speed.";

pub fn ensure_configured() -> Result<()> {
    let mut config = load_config();

    // Check Env Vars or Config File
    if env::var("GROQ_API_KEY").is_ok() || env::var("GEMINI_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
        return Ok(());
    }
    if config.groq_api_key.is_some() || config.gemini_api_key.is_some() || config.openai_api_key.is_some() {
        return Ok(());
    }

    println!("Welcome to Rewind. First-time setup required.");
    println!("Please enter your preferred API key (Groq, Gemini, or OpenAI supported).");
    print!("API Key: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let key = input.trim().to_string();

    if key.is_empty() {
        anyhow::bail!("No API key provided. Exiting.");
    }

    // Auto-detect based on key prefix
    if key.starts_with("gsk_") {
        println!("Provider detected: Groq. Saving configuration...\n");
        config.groq_api_key = Some(key.clone());
    } else if key.starts_with("AIza") {
        println!("Provider detected: Gemini. Saving configuration...\n");
        config.gemini_api_key = Some(key.clone());
    } else {
        println!("Provider detected: OpenAI. Saving configuration...\n");
        config.openai_api_key = Some(key.clone());
    }
    
    let _ = save_config(&config);
    Ok(())
}

pub async fn analyze_repo(state: &RepoState) -> Result<String> {
    let config = load_config();

    // Auto-detect provider based on available keys.
    // Order of precedence: Env Vars -> Config File

    let mut configured_key = None;

    // 1. Check Env Vars or Config File
    if let Ok(key) = env::var("GROQ_API_KEY") {
        configured_key = Some(("GROQ", key));
    } else if let Ok(key) = env::var("GEMINI_API_KEY") {
        configured_key = Some(("GEMINI", key));
    } else if let Ok(key) = env::var("OPENAI_API_KEY") {
        configured_key = Some(("OPENAI", key));
    } else if let Some(key) = config.groq_api_key.clone() {
        configured_key = Some(("GROQ", key));
    } else if let Some(key) = config.gemini_api_key.clone() {
        configured_key = Some(("GEMINI", key));
    } else if let Some(key) = config.openai_api_key.clone() {
        configured_key = Some(("OPENAI", key));
    }

    // 3. Map to specific endpoints based on detected provider
    let (vendor, api_key) = configured_key.unwrap();
    
    let (api_base, model) = match vendor {
        "GROQ" => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.groq.com/openai/v1".to_string()),
            env::var("OPENAI_MODEL").unwrap_or_else(|_| "llama3-70b-8192".to_string()),
        ),
        "GEMINI" => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".to_string()),
            env::var("OPENAI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
        ),
        _ => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        ),
    };

    let mut user_prompt = format!(
        "Repository State:\n\
        - Branch: {}\n\
        - Status:\n{}\n\
        - Recent Commits:\n{}\n",
        state.branch, state.status, state.log
    );

    if !state.diff_cached.is_empty() {
        let diff = truncate_lines(&state.diff_cached, 10000);
        user_prompt.push_str(&format!("- Staged Changes:\n{}\n", diff));
    }

    if !state.diff.is_empty() {
        let diff = truncate_lines(&state.diff, 10000);
        user_prompt.push_str(&format!("- Unstaged Changes:\n{}\n", diff));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30)) // Security/Robustness: Prevents infinite hangs if the API is unresponsive
        .build()
        .context("Failed to build HTTP client")?;

    let res = client
        .post(format!("{}/chat/completions", api_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 500,
        }))
        .send()
        .await
        .context("Failed to connect to the AI API")?;

    if !res.status().is_success() {
        let text = res.text().await?;
        anyhow::bail!("API returned an error: {}", text);
    }

    let body: serde_json::Value = res.json().await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to parse response content")?;

    Ok(content.to_string())
}

/// Truncates a string to roughly `max_bytes` while preserving line breaks.
/// If it gets truncated, it appends an indicator.
fn truncate_lines(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }

    let mut current_bytes = 0;
    let mut truncated = String::new();

    for line in s.lines() {
        let line_len = line.len() + 1; // +1 for newline
        if current_bytes + line_len > max_bytes {
            truncated.push_str("\n... [Diff truncated due to length] ...\n");
            break;
        }
        truncated.push_str(line);
        truncated.push('\n');
        current_bytes += line_len;
    }

    truncated.trim_end().to_string()
}
