use crate::config;
use crate::provider::Provider;
use anyhow::{Context, Result};
use reqwest::Client;
use secrecy::ExposeSecret;
use serde_json::json;
use std::env;
use std::io::Write;

use super::models::discover_best_model;

/// Maximum number of tokens the AI can return in a single response.
const MAX_RESPONSE_TOKENS: u32 = 1500;

/// Maximum API retries.
const MAX_RETRIES: u32 = 3;

fn resolve_provider_and_key(cfg: &config::Config) -> Result<(Provider, String)> {
    for &provider in Provider::all() {
        if let Ok(key) = env::var(provider.env_key_name()) {
            return Ok((provider, key));
        }
        if let Some(key) = cfg.get_api_key(provider) {
            return Ok((provider, key.expose_secret().to_string()));
        }
    }

    anyhow::bail!("API key not configured. Run `rewind config set <provider>` to set one.")
}

/// Build an HTTP client and resolve the model for a given provider.
/// Checks the on-disk model cache (24 h TTL) before calling `GET /models`.
async fn setup_client(
    provider: Provider,
    api_key: &str,
    cfg: &config::Config,
) -> Result<(Client, String, String)> {
    let api_base = env::var("OPENAI_API_BASE")
        .unwrap_or_else(|_| provider.default_api_base().to_string())
        .trim_end_matches('/')
        .to_string();

    let maybe_model = env::var("OPENAI_MODEL")
        .ok()
        .or_else(|| cfg.get_model(provider).cloned());

    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let model = match maybe_model {
        Some(m) => m,
        None => {
            // Serve from cache when fresh — avoids a network round-trip on every run
            if let Some(cached) = cfg.get_cached_model(provider) {
                cached
            } else {
                let discovered = discover_best_model(&client, &api_base, api_key, provider).await;
                // Persist to config so the next invocation hits the cache
                let mut fresh_cfg = config::load_config();
                fresh_cfg.set_cached_model(provider, discovered.clone());
                let _ = config::save_config(&fresh_cfg);
                discovered
            }
        }
    };

    Ok((client, api_base, model))
}

/// Send a non-streaming API call and return the full response along with optional token usage (prompt, completion).
pub async fn api_call(
    system_prompt: &str,
    user_prompt: &str,
) -> Result<(String, Option<(u32, u32)>)> {
    let cfg = config::load_config();
    let (provider, api_key) = resolve_provider_and_key(&cfg)?;
    let (client, api_base, model) = setup_client(provider, &api_key, &cfg).await?;

    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "temperature": 0.7,
        "max_tokens": MAX_RESPONSE_TOKENS,
    });

    let mut attempt = 0;
    let res = loop {
        attempt += 1;
        let response = client
            .post(format!("{}/chat/completions", api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(r) if r.status().is_success() => break r,
            Ok(r)
                if attempt < MAX_RETRIES
                    && (r.status().as_u16() == 429 || r.status().is_server_error()) =>
            {
                // Exponential backoff: 1 s, 2 s, 4 s … capped at 30 s
                let backoff =
                    std::time::Duration::from_secs(std::cmp::min(1u64 << (attempt - 1), 30));
                tokio::time::sleep(backoff).await;
                continue;
            }
            Ok(r) => anyhow::bail!("API returned an error: {}", r.text().await?),
            Err(_) if attempt < MAX_RETRIES => {
                let backoff =
                    std::time::Duration::from_secs(std::cmp::min(1u64 << (attempt - 1), 30));
                tokio::time::sleep(backoff).await;
                continue;
            }
            Err(e) => return Err(e).context("Failed to connect to the AI API"),
        }
    };

    let body: serde_json::Value = res.json().await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to parse response content")?;

    let mut usage = None;
    if let Some(usage_obj) = body.get("usage") {
        if let (Some(p), Some(c)) = (
            usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()),
            usage_obj.get("completion_tokens").and_then(|v| v.as_u64()),
        ) {
            usage = Some((p as u32, c as u32));
        }
    }

    Ok((content.to_string(), usage))
}

/// Send a streaming API call, printing tokens to stdout as they arrive.
/// Calls `on_first_token` exactly once when the first content token is received
/// (useful for clearing a spinner before output begins).
/// Returns the full accumulated response and optional token usage (prompt, completion).
pub async fn api_call_streaming(
    system_prompt: &str,
    user_prompt: &str,
    on_first_token: impl FnOnce(),
) -> Result<(String, Option<(u32, u32)>)> {
    let cfg = config::load_config();
    let (provider, api_key) = resolve_provider_and_key(&cfg)?;
    let (client, api_base, model) = setup_client(provider, &api_key, &cfg).await?;

    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "temperature": 0.7,
        "max_tokens": MAX_RESPONSE_TOKENS,
        "stream": true,
        "stream_options": {"include_usage": true}
    });

    let mut attempt = 0;
    let mut res = loop {
        attempt += 1;
        let response = client
            .post(format!("{}/chat/completions", api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(r) if r.status().is_success() => break r,
            Ok(r)
                if attempt < MAX_RETRIES
                    && (r.status().as_u16() == 429 || r.status().is_server_error()) =>
            {
                // Exponential backoff: 1 s, 2 s, 4 s … capped at 30 s
                let backoff =
                    std::time::Duration::from_secs(std::cmp::min(1u64 << (attempt - 1), 30));
                tokio::time::sleep(backoff).await;
                continue;
            }
            Ok(r) => anyhow::bail!("API returned an error: {}", r.text().await?),
            Err(_) if attempt < MAX_RETRIES => {
                let backoff =
                    std::time::Duration::from_secs(std::cmp::min(1u64 << (attempt - 1), 30));
                tokio::time::sleep(backoff).await;
                continue;
            }
            Err(e) => return Err(e).context("Failed to connect to the AI API"),
        }
    };

    let mut full_response = String::new();
    let mut buffer = String::new();
    let mut first_token = true;
    let mut on_first_token = Some(on_first_token);
    let mut usage = None;

    while let Some(chunk) = res.chunk().await? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process all complete SSE lines from the buffer
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    return Ok((full_response, usage));
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    // Try to extract usage telemetry
                    if let Some(usage_obj) = parsed
                        .get("usage")
                        .or_else(|| parsed.get("x_groq").and_then(|g| g.get("usage")))
                    {
                        if let (Some(p), Some(c)) = (
                            usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()),
                            usage_obj.get("completion_tokens").and_then(|v| v.as_u64()),
                        ) {
                            usage = Some((p as u32, c as u32));
                        }
                    }

                    if let Some(choices) = parsed.get("choices") {
                        if let Some(choice) = choices.get(0) {
                            if let Some(delta) = choice.get("delta") {
                                if let Some(content) = delta.get("content").and_then(|c| c.as_str())
                                {
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
            }
        }
    }

    // If we never got a token, still fire the callback to clean up the spinner
    if let Some(callback) = on_first_token.take() {
        callback();
    }

    Ok((full_response, usage))
}
