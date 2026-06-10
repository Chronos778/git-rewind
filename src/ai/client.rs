use crate::config;
use crate::provider::Provider;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;
use std::io::Write;

use super::models::discover_best_model;

/// Maximum number of tokens the AI can return in a single response.
const MAX_RESPONSE_TOKENS: u32 = 500;

/// HTTP request timeout in seconds.
const API_TIMEOUT_SECS: u64 = 30;

/// Resolve which provider and API key to use.
/// Priority: env vars first (Groq > Gemini > OpenAI), then config file.
fn resolve_provider_and_key(cfg: &config::Config) -> Result<(Provider, String)> {
    // Check environment variables first
    for &provider in Provider::all() {
        if let Ok(key) = env::var(provider.env_key_name()) {
            return Ok((provider, key));
        }
    }

    // Fall back to config file
    for &provider in Provider::all() {
        if let Some(key) = cfg.get_api_key(provider) {
            return Ok((provider, key.clone()));
        }
    }

    anyhow::bail!("API key not configured. Run `rewind config set <provider>` to set one.")
}

/// Build an HTTP client and resolve the model for a given provider.
async fn setup_client(provider: Provider, api_key: &str, cfg: &config::Config) -> Result<(Client, String, String)> {
    let api_base = env::var("OPENAI_API_BASE")
        .unwrap_or_else(|_| provider.default_api_base().to_string());

    let maybe_model = env::var("OPENAI_MODEL")
        .ok()
        .or_else(|| cfg.get_model(provider).cloned());

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    let model = match maybe_model {
        Some(m) => m,
        None => discover_best_model(&client, &api_base, api_key, provider).await,
    };

    Ok((client, api_base, model))
}

/// Send a non-streaming API call and return the full response.
pub async fn api_call(system_prompt: &str, user_prompt: &str) -> Result<String> {
    let cfg = config::load_config();
    let (provider, api_key) = resolve_provider_and_key(&cfg)?;
    let (client, api_base, model) = setup_client(provider, &api_key, &cfg).await?;

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

/// Send a streaming API call, printing tokens to stdout as they arrive.
/// Calls `on_first_token` exactly once when the first content token is received
/// (useful for clearing a spinner before output begins).
/// Returns the full accumulated response.
pub async fn api_call_streaming(
    system_prompt: &str,
    user_prompt: &str,
    on_first_token: impl FnOnce(),
) -> Result<String> {
    let cfg = config::load_config();
    let (provider, api_key) = resolve_provider_and_key(&cfg)?;
    let (client, api_base, model) = setup_client(provider, &api_key, &cfg).await?;

    let mut res = client
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
            "stream": true,
        }))
        .send()
        .await
        .context("Failed to connect to the AI API")?;

    if !res.status().is_success() {
        let text = res.text().await?;
        anyhow::bail!("API returned an error: {}", text);
    }

    let mut full_response = String::new();
    let mut buffer = String::new();
    let mut first_token = true;
    let mut on_first_token = Some(on_first_token);

    while let Some(chunk) = res.chunk().await? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process all complete SSE lines from the buffer
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    return Ok(full_response);
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        if first_token {
                            if let Some(callback) = on_first_token.take() {
                                callback();
                            }
                            first_token = false;
                        }
                        print!("{}", content);
                        let _ = std::io::stdout().flush();
                        full_response.push_str(content);
                    }
                }
            }
        }
    }

    // If we never got a token, still fire the callback to clean up the spinner
    if let Some(callback) = on_first_token.take() {
        callback();
    }

    Ok(full_response)
}
