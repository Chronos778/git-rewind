use crate::ConfigCommands;
use anyhow::{Context, Result};
use colored::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub groq_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub groq_model: Option<String>,
    pub gemini_model: Option<String>,
    pub openai_model: Option<String>,
}

fn get_config_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "Rewind", "Rewind").map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}

pub fn load_config() -> Config {
    if let Some(path) = get_config_path() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                return config;
            }
        }
    }
    Config::default()
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

fn mask_key(k: &str) -> String {
    if k.len() <= 8 {
        return "********".to_string();
    }
    let b = k.as_bytes();
    let n = b.len();
    format!(
        "{}{}{}{}...{}{}{}{}",
        b[0] as char, b[1] as char, b[2] as char, b[3] as char,
        b[n-4] as char, b[n-3] as char, b[n-2] as char, b[n-1] as char
    )
}

pub fn handle_config_command(action: ConfigCommands) -> Result<()> {
    let mut config = load_config();

    match action {
        ConfigCommands::Set { provider, key } => {
            // If key is not provided, prompt securely to avoid exposing it in shell history
            let api_key = match key {
                Some(k) => k,
                None => {
                    print!("Enter API key for {}: ", provider);
                    io::stdout().flush()?;
                    let k = rpassword::read_password().context("Failed to read API key")?;
                    let k = k.trim().to_string();
                    if k.is_empty() {
                        anyhow::bail!("No API key provided. Aborting.");
                    }
                    k
                }
            };

            match provider.to_lowercase().as_str() {
                "groq" => config.groq_api_key = Some(api_key),
                "gemini" => config.gemini_api_key = Some(api_key),
                "openai" => config.openai_api_key = Some(api_key),
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} API key for {} has been set.", "[SUCCESS]".green(), provider);
        }
        ConfigCommands::Model { provider, model } => {
            match provider.to_lowercase().as_str() {
                "groq" => config.groq_model = Some(model.clone()),
                "gemini" => config.gemini_model = Some(model.clone()),
                "openai" => config.openai_model = Some(model.clone()),
                _ => anyhow::bail!("Unknown provider: {}. Supported providers are: groq, gemini, openai.", provider),
            }
            save_config(&config)?;
            println!("{} Custom model '{}' for {} has been set.", "[SUCCESS]".green(), model, provider);
        }
        ConfigCommands::Clear { provider } => {
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
        ConfigCommands::Show => {
            println!("{} \n", "[ CONFIGURED API KEYS & MODELS ]".bold());
            if let Some(key) = &config.groq_api_key {
                let model = config.groq_model.as_deref().unwrap_or("llama-3.3-70b-versatile (default)");
                println!("{}: {} [Model: {}]", "Groq".green(), mask_key(key), model);
            } else {
                println!("{}: Not set", "Groq".bright_black());
            }

            if let Some(key) = &config.gemini_api_key {
                let model = config.gemini_model.as_deref().unwrap_or("gemini-2.0-flash (default)");
                println!("{}: {} [Model: {}]", "Gemini".green(), mask_key(key), model);
            } else {
                println!("{}: Not set", "Gemini".bright_black());
            }

            if let Some(key) = &config.openai_api_key {
                let model = config.openai_model.as_deref().unwrap_or("gpt-4o-mini (default)");
                println!("{}: {} [Model: {}]", "OpenAI".green(), mask_key(key), model);
            } else {
                println!("{}: Not set", "OpenAI".bright_black());
            }
        }
    }
    Ok(())
}

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

    save_config(&config).context("Failed to save configuration after first-time setup")?;
    Ok(())
}

pub fn clear_all_data() {
    if let Some(proj_dirs) = ProjectDirs::from("com", "Rewind", "Rewind") {
        let _ = fs::remove_dir_all(proj_dirs.config_dir());
    }
}
