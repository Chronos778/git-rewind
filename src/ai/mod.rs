mod client;
mod models;
mod prompts;

use crate::git::RepoState;
use anyhow::Result;

pub use prompts::build_user_prompt;

/// Analyze the repo state. Uses streaming for normal output, non-streaming for json/short.
pub async fn analyze_repo(state: &RepoState, short: bool, json_format: bool) -> Result<String> {
    let mut actual_system_prompt = prompts::SYSTEM_PROMPT.to_string();
    if short {
        actual_system_prompt.push_str(
            "\n\nConstraint: Your response MUST be extremely short. 2 sentences maximum.",
        );
    }
    if json_format {
        actual_system_prompt.push_str(
            "\n\nConstraint: Output only raw plaintext without formatting. Do not use code blocks.",
        );
    }
    let user_prompt = prompts::build_user_prompt(state);
    client::api_call(&actual_system_prompt, &user_prompt).await
}

/// Analyze the repo with streaming output (prints tokens as they arrive).
/// The `on_first_token` callback fires once when the first token is received
/// — use it to clear the spinner and print the brief header.
pub async fn analyze_repo_streaming(
    state: &RepoState,
    on_first_token: impl FnOnce(),
) -> Result<String> {
    let user_prompt = prompts::build_user_prompt(state);
    client::api_call_streaming(prompts::SYSTEM_PROMPT, &user_prompt, on_first_token).await
}

/// Ask a question, streaming the answer to stdout.
pub async fn ask_question_streaming(
    state: &RepoState,
    query: &str,
    on_first_token: impl FnOnce(),
) -> Result<String> {
    let system_prompt = "You are an expert AI pair programmer embedded in the user's terminal. Answer the user's question accurately based on their current repository state and diffs.";
    let user_prompt = format!(
        "{}\n\nUser Question:\n{}",
        prompts::build_user_prompt(state),
        query
    );
    client::api_call_streaming(system_prompt, &user_prompt, on_first_token).await
}

/// Generate a commit message (non-streaming — returns the full message).
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

    let diff = prompts::truncate_lines(&user_prompt, prompts::MAX_PROMPT_DIFF_BYTES);
    client::api_call(system_prompt, &diff).await
}
