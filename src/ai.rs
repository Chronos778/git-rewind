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
    groq_model: Option<String>,
    gemini_model: Option<String>,
    openai_model: Option<String>,
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
            println!("{} API key for {} has been set.", "[SUCCESS]".green(), provider);
        }
        crate::ConfigCommands::Model { provider, model } => {
            match provider.to_lowercase().as_str() {
                "groq" => config.groq_model = Some(model.clone()),
                "gemini" => config.gemini_model = Some(model.clone()),
                "openai" => config.openai_model = Some(model.clone()),
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} Custom model '{}' for {} has been set.", "[SUCCESS]".green(), model, provider);
        }
        crate::ConfigCommands::Clear { provider } => {
            match provider.to_lowercase().as_str() {
                "groq" => {
                    config.groq_api_key = None;
                    config.groq_model = None;
                },
                "gemini" => {
                    config.gemini_api_key = None;
                    config.gemini_model = None;
                },
                "openai" => {
                    config.openai_api_key = None;
                    config.openai_model = None;
                },
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} Settings for {} have been cleared.", "[SUCCESS]".green(), provider);
        }
        crate::ConfigCommands::Show => {
            println!("{} \n", "[ CONFIGURED API KEYS & MODELS ]".bold());
            if let Some(key) = &config.groq_api_key {
                let model = config.groq_model.as_deref().unwrap_or("llama-3.3-70b-versatile (default)");
                println!("{}: {} [Model: {}]", "Groq".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]), model);
            } else {
                println!("{}: Not set", "Groq".bright_black());
            }

            if let Some(key) = &config.gemini_api_key {
                let model = config.gemini_model.as_deref().unwrap_or("gemini-1.5-flash (default)");
                println!("{}: {} [Model: {}]", "Gemini".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]), model);
            } else {
                println!("{}: Not set", "Gemini".bright_black());
            }

            if let Some(key) = &config.openai_api_key {
                let model = config.openai_model.as_deref().unwrap_or("gpt-4o-mini (default)");
                println!("{}: {} [Model: {}]", "OpenAI".green(), format!("{}...", &key[..std::cmp::min(10, key.len())]), model);
            } else {
                println!("{}: Not set", "OpenAI".bright_black());
            }
        }
    }
    Ok(())
}

pub fn clear_all_data() {
    if let Some(proj_dirs) = ProjectDirs::from("com", "Rewind", "Rewind") {
        let _ = fs::remove_dir_all(proj_dirs.config_dir());
    }
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

    let key = rpassword::read_password().unwrap_or_default().trim().to_string();

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

pub fn build_user_prompt(state: &RepoState) -> String {
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
    user_prompt
}

async fn api_call(system_prompt: &str, user_prompt: &str) -> Result<String> {
    let config = load_config();

    let mut configured_key = None;

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

    if configured_key.is_none() {
        anyhow::bail!("API key not configured. Run `rewind config set <provider> <key>`.");
    }

    let (vendor, api_key) = configured_key.unwrap();
    
    let (api_base, maybe_model) = match vendor {
        "GROQ" => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.groq.com/openai/v1".to_string()),
            env::var("OPENAI_MODEL").ok().or(config.groq_model.clone()),
        ),
        "GEMINI" => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".to_string()),
            env::var("OPENAI_MODEL").ok().or(config.gemini_model.clone()),
        ),
        _ => (
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            env::var("OPENAI_MODEL").ok().or(config.openai_model.clone()),
        ),
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let model = match maybe_model {
        Some(m) => m,
        None => discover_best_model(&client, &api_base, &api_key, vendor).await,
    };

    let res = client
        .post(format!("{}/chat/completions", api_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
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

async fn discover_best_model(client: &Client, api_base: &str, api_key: &str, vendor: &str) -> String {
    let default_fallback = match vendor {
        "GROQ" => "llama-3.3-70b-versatile",
        "GEMINI" => "gemini-1.5-flash",
        _ => "gpt-4o-mini",
    };

    let res = match client
        .get(format!("{}/models", api_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return default_fallback.to_string(),
    };

    if !res.status().is_success() {
        return default_fallback.to_string();
    }

    let body: serde_json::Value = match res.json().await {
        Ok(b) => b,
        Err(_) => return default_fallback.to_string(),
    };

    let mut available_models = Vec::new();
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                let lower_id = id.to_lowercase();
                // Filter out non-text/vision utility models
                if lower_id.contains("embedding") || lower_id.contains("whisper") || lower_id.contains("dall-e") || lower_id.contains("vision") || lower_id.contains("tts") || lower_id.contains("audio") || lower_id.contains("moderation") {
                    continue;
                }
                available_models.push(id.to_string());
            }
        }
    }

    if available_models.is_empty() {
        return default_fallback.to_string();
    }

    match vendor {
        "GROQ" => {
            if let Some(m) = available_models.iter().find(|m| m.contains("versatile") || (m.contains("llama") && m.contains("70b"))) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| (m.contains("llama") && m.contains("8b")) || m.contains("mixtral")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("llama")) {
                return m.clone();
            }
            available_models[0].clone()
        }
        "GEMINI" => {
            if let Some(m) = available_models.iter().find(|m| m.contains("flash")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("gemini")) {
                return m.clone();
            }
            available_models[0].clone()
        }
        _ => {
            if let Some(m) = available_models.iter().find(|m| m.contains("mini")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("turbo")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("gpt-4")) {
                return m.clone();
            }
            available_models[0].clone()
        }
    }
}

pub async fn analyze_repo(state: &RepoState, short: bool, json_format: bool) -> Result<String> {
    let mut actual_system_prompt = SYSTEM_PROMPT.to_string();
    if short {
        actual_system_prompt.push_str("\n\nConstraint: Your response MUST be extremely short. 2 sentences maximum.");
    }
    if json_format {
        actual_system_prompt.push_str("\n\nConstraint: Output only raw plaintext without formatting. Do not use code blocks.");
    }
    let user_prompt = build_user_prompt(state);
    api_call(&actual_system_prompt, &user_prompt).await
}

pub async fn generate_commit_message(state: &RepoState) -> Result<String> {
    let system_prompt = "You are an expert developer. Generate a clean, descriptive, and conventional Git commit message based on the provided diff. Output ONLY the commit message. First line should be the subject. Then a blank line, then bullet points for details if needed.";
    let mut user_prompt = String::new();
    if !state.diff_cached.is_empty() {
        user_prompt.push_str(&state.diff_cached);
    } else if !state.diff.is_empty() {
        user_prompt.push_str(&state.diff);
    } else {
        anyhow::bail!("No changes found to generate a commit message for.");
    }
    
    let diff = truncate_lines(&user_prompt, 10000);
    api_call(system_prompt, &diff).await
}

pub async fn ask_question(state: &RepoState, query: &str) -> Result<String> {
    let system_prompt = "You are an expert AI pair programmer embedded in the user's terminal. Answer the user's question accurately based on their current repository state and diffs.";
    let user_prompt = format!("{}\n\nUser Question:\n{}", build_user_prompt(state), query);
    api_call(system_prompt, &user_prompt).await
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
