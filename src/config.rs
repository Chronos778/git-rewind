use crate::provider::Provider;
use crate::ConfigCommands;
use anyhow::{Context, Result};
use colored::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// How long (seconds) a discovered model is considered fresh. 24 hours.
const MODEL_CACHE_TTL_SECS: u64 = 86_400;

/// A cached model entry stored in the config file.
#[derive(Serialize, Deserialize, Clone)]
pub struct CachedModel {
    pub model: String,
    /// Unix timestamp (seconds) when the model was discovered and cached.
    pub cached_at: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub groq_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub groq_model: Option<String>,
    pub gemini_model: Option<String>,
    pub openai_model: Option<String>,
    /// Per-provider model cache so `GET /models` is not called on every invocation.
    #[serde(default)]
    pub model_cache: HashMap<String, CachedModel>,
    /// Optional custom system prompt
    #[serde(default)]
    pub system_prompt: Option<String>,
}

impl Config {
    /// Get the API key for a provider.
    pub fn get_api_key(&self, provider: Provider) -> Option<&String> {
        match provider {
            Provider::Groq => self.groq_api_key.as_ref(),
            Provider::Gemini => self.gemini_api_key.as_ref(),
            Provider::OpenAi => self.openai_api_key.as_ref(),
        }
    }

    /// Set (or clear) the API key for a provider.
    pub fn set_api_key(&mut self, provider: Provider, key: Option<String>) {
        match provider {
            Provider::Groq => self.groq_api_key = key,
            Provider::Gemini => self.gemini_api_key = key,
            Provider::OpenAi => self.openai_api_key = key,
        }
    }

    /// Get the custom model for a provider.
    pub fn get_model(&self, provider: Provider) -> Option<&String> {
        match provider {
            Provider::Groq => self.groq_model.as_ref(),
            Provider::Gemini => self.gemini_model.as_ref(),
            Provider::OpenAi => self.openai_model.as_ref(),
        }
    }

    /// Set (or clear) the custom model for a provider.
    pub fn set_model(&mut self, provider: Provider, model: Option<String>) {
        match provider {
            Provider::Groq => self.groq_model = model,
            Provider::Gemini => self.gemini_model = model,
            Provider::OpenAi => self.openai_model = model,
        }
    }

    /// Return the cached model for a provider if it is still within the TTL window.
    pub fn get_cached_model(&self, provider: Provider) -> Option<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.model_cache
            .get(provider.cache_key())
            .and_then(|entry| {
                if now.saturating_sub(entry.cached_at) < MODEL_CACHE_TTL_SECS {
                    Some(entry.model.clone())
                } else {
                    None
                }
            })
    }

    /// Store a discovered model in the cache with the current timestamp.
    pub fn set_cached_model(&mut self, provider: Provider, model: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.model_cache.insert(
            provider.cache_key().to_string(),
            CachedModel {
                model,
                cached_at: now,
            },
        );
    }

    /// Merge another Config (e.g. from .rewindrc) into this one, overwriting current values.
    pub fn merge(&mut self, other: Config) {
        if other.groq_api_key.is_some() { self.groq_api_key = other.groq_api_key; }
        if other.gemini_api_key.is_some() { self.gemini_api_key = other.gemini_api_key; }
        if other.openai_api_key.is_some() { self.openai_api_key = other.openai_api_key; }
        if other.groq_model.is_some() { self.groq_model = other.groq_model; }
        if other.gemini_model.is_some() { self.gemini_model = other.gemini_model; }
        if other.openai_model.is_some() { self.openai_model = other.openai_model; }
        if other.system_prompt.is_some() { self.system_prompt = other.system_prompt; }
    }
}

fn get_config_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "Rewind", "Rewind")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}

pub fn load_config() -> Config {
    let mut base_config = Config::default();

    // 1. Load global config
    if let Some(path) = get_config_path() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                base_config = config;
            }
        }
    }

    // 2. Load and merge local .rewindrc
    if let Ok(cwd) = env::current_dir() {
        let local_rc = cwd.join(".rewindrc");
        if local_rc.exists() {
            if let Ok(contents) = fs::read_to_string(local_rc) {
                if let Ok(local_config) = serde_json::from_str::<Config>(&contents) {
                    base_config.merge(local_config);
                }
            }
        }
    }

    base_config
}

pub fn save_config(config: &Config) -> Result<()> {
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
        b[0] as char,
        b[1] as char,
        b[2] as char,
        b[3] as char,
        b[n - 4] as char,
        b[n - 3] as char,
        b[n - 2] as char,
        b[n - 1] as char
    )
}

pub fn handle_config_command(action: ConfigCommands) -> Result<()> {
    let mut config = load_config();

    match action {
        ConfigCommands::Set { provider, key } => {
            let p = Provider::from_name(&provider)?;

            // If key is not provided, prompt securely to avoid exposing it in shell history
            let api_key = match key {
                Some(k) => k,
                None => {
                    print!("Enter API key for {}: ", p.display_name());
                    io::stdout().flush()?;
                    let k = rpassword::read_password().context("Failed to read API key")?;
                    let k = k.trim().to_string();
                    if k.is_empty() {
                        anyhow::bail!("No API key provided. Aborting.");
                    }
                    k
                }
            };

            config.set_api_key(p, Some(api_key));
            save_config(&config)?;
            println!(
                "{} API key for {} has been set.",
                "[SUCCESS]".green(),
                p.display_name()
            );
        }
        ConfigCommands::Model { provider, model } => {
            let p = Provider::from_name(&provider)?;
            config.set_model(p, Some(model.clone()));
            save_config(&config)?;
            println!(
                "{} Custom model '{}' for {} has been set.",
                "[SUCCESS]".green(),
                model,
                p.display_name()
            );
        }
        ConfigCommands::Clear { provider } => {
            let p = Provider::from_name(&provider)?;
            config.set_api_key(p, None);
            config.set_model(p, None);
            save_config(&config)?;
            println!(
                "{} Settings for {} have been cleared.",
                "[SUCCESS]".green(),
                p.display_name()
            );
        }
        ConfigCommands::SystemPrompt { prompt } => {
            if let Some(p) = prompt {
                config.system_prompt = Some(p.clone());
                save_config(&config)?;
                println!("{} Custom system prompt has been set.", "[SUCCESS]".green());
            } else {
                config.system_prompt = None;
                save_config(&config)?;
                println!("{} Custom system prompt has been cleared.", "[SUCCESS]".green());
            }
        }
        ConfigCommands::Show => {
            println!("{} \n", "[ CONFIGURED API KEYS & MODELS ]".bold());
            for &p in Provider::all() {
                if let Some(key) = config.get_api_key(p) {
                    let model = match config.get_model(p) {
                        Some(m) => m.clone(),
                        None => format!("{} (default)", p.default_model()),
                    };
                    println!(
                        "{}: {} [Model: {}]",
                        p.display_name().green(),
                        mask_key(key),
                        model
                    );
                } else {
                    println!("{}: Not set", p.display_name().bright_black());
                }
            }
            if let Some(prompt) = &config.system_prompt {
                println!("\n{}: {}", "Custom System Prompt".cyan(), prompt);
            }
        }
    }
    Ok(())
}

pub fn ensure_configured() -> Result<()> {
    let mut config = load_config();

    // Check Env Vars or Config File (in priority order)
    for &p in Provider::all() {
        if env::var(p.env_key_name()).is_ok() {
            return Ok(());
        }
    }
    for &p in Provider::all() {
        if config.get_api_key(p).is_some() {
            return Ok(());
        }
    }

    println!("Welcome to Rewind. First-time setup required.");
    println!("Please enter your preferred API key (Groq, Gemini, or OpenAI supported).");
    print!("API Key: ");
    io::stdout().flush()?;

    let key = rpassword::read_password()
        .unwrap_or_default()
        .trim()
        .to_string();

    if key.is_empty() {
        anyhow::bail!("No API key provided. Exiting.");
    }

    let (provider, exact_match) = Provider::detect_from_key(&key);
    if !exact_match {
        println!(
            "{} Key prefix not recognized — assuming {}. If this is wrong, run `rewind config set <provider>`.",
            "[WARN]".yellow(),
            provider.display_name()
        );
    } else {
        println!(
            "Provider detected: {}. Saving configuration...\n",
            provider.display_name()
        );
    }
    config.set_api_key(provider, Some(key));

    save_config(&config).context("Failed to save configuration after first-time setup")?;
    Ok(())
}

pub fn clear_all_data() {
    if let Some(proj_dirs) = ProjectDirs::from("com", "Rewind", "Rewind") {
        let _ = fs::remove_dir_all(proj_dirs.config_dir());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_long() {
        let masked = mask_key("gsk_abcdefghijklmnop");
        assert_eq!(masked, "gsk_...mnop");
    }

    #[test]
    fn test_mask_key_short() {
        assert_eq!(mask_key("abc"), "********");
        assert_eq!(mask_key("12345678"), "********");
    }

    #[test]
    fn test_mask_key_exactly_nine() {
        let masked = mask_key("123456789");
        assert_eq!(masked, "1234...6789");
    }

    #[test]
    fn test_config_get_set_api_key() {
        let mut config = Config::default();
        assert!(config.get_api_key(Provider::Groq).is_none());

        config.set_api_key(Provider::Groq, Some("test_key".to_string()));
        assert_eq!(config.get_api_key(Provider::Groq).unwrap(), "test_key");

        config.set_api_key(Provider::Groq, None);
        assert!(config.get_api_key(Provider::Groq).is_none());
    }

    #[test]
    fn test_config_get_set_model() {
        let mut config = Config::default();
        assert!(config.get_model(Provider::Gemini).is_none());

        config.set_model(Provider::Gemini, Some("gemini-pro".to_string()));
        assert_eq!(config.get_model(Provider::Gemini).unwrap(), "gemini-pro");
    }
}
