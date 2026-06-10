use crate::config;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;

use super::models::discover_best_model;

/// Maximum number of tokens the AI can return in a single response.
const MAX_RESPONSE_TOKENS: u32 = 500;

/// HTTP request timeout in seconds.
const API_TIMEOUT_SECS: u64 = 30;

pub async fn api_call(system_prompt: &str, user_prompt: &str) -> Result<String> {
    let config = config::load_config();

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

    let (vendor, api_key) = configured_key
        .context("API key not configured. Run `rewind config set <provider>` to set one.")?;

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
        .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECS))
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
            "max_tokens": MAX_RESPONSE_TOKENS,
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
