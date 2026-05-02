use crate::git::RepoState;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;

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

pub async fn analyze_repo(state: &RepoState) -> Result<String> {
    let api_key = env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable is not set. Please set it to use rewind.")?;

    // Allow overriding the API base url for compatibility with other OpenAI-compatible APIs (like Ollama, LiteLLM, vLLM)
    let api_base =
        env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

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

    let client = Client::new();
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
