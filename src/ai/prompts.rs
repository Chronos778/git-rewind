use crate::git::RepoState;

/// Maximum number of bytes to include from a diff in the AI prompt.
pub const MAX_PROMPT_DIFF_BYTES: usize = 10_000;

pub const SYSTEM_PROMPT: &str = "\
You are an AI assistant helping a developer instantly get back into their flow. \
The developer has just opened their terminal and run the `rewind` command. \
You will be provided with the current state of their git repository (branch, status, recent commits, and unstaged/staged diffs). \
Your job is to read this context and tell the developer what they were doing and what they likely need to do next.

Keep it concise, engaging, and direct. \
Don't use lists unless absolutely necessary. \
Focus on the 'why' and 'what' of the changes. \
Do not output markdown code blocks unless it's a specific shell command they should run. \
Make it sound like a helpful colleague bringing them up to speed.";

pub fn build_user_prompt(state: &RepoState) -> String {
    let mut user_prompt = format!(
        "Repository State:\n\
        - Branch: {}\n\
        - Status:\n{}\n\
        - Recent Commits:\n{}\n",
        state.branch, state.status, state.log
    );

    if !state.diff_cached.is_empty() {
        let diff = truncate_lines(&state.diff_cached, MAX_PROMPT_DIFF_BYTES);
        user_prompt.push_str(&format!("- Staged Changes:\n{}\n", diff));
    }

    if !state.diff.is_empty() {
        let diff = truncate_lines(&state.diff, MAX_PROMPT_DIFF_BYTES);
        user_prompt.push_str(&format!("- Unstaged Changes:\n{}\n", diff));
    }
    user_prompt
}

/// Truncates a string to roughly `max_bytes` while preserving line breaks.
/// If it gets truncated, it appends an indicator.
pub fn truncate_lines(s: &str, max_bytes: usize) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_lines_short_string() {
        let input = "hello\nworld";
        assert_eq!(truncate_lines(input, 100), input);
    }

    #[test]
    fn test_truncate_lines_exact_boundary() {
        let input = "hello\nworld\n"; // 12 bytes
        assert_eq!(truncate_lines(input, 12), "hello\nworld\n");
    }

    #[test]
    fn test_truncate_lines_triggers_truncation() {
        let input = "line1\nline2\nline3\nline4\nline5";
        let result = truncate_lines(input, 12);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("[Diff truncated"));
        assert!(!result.contains("line5"));
    }

    #[test]
    fn test_truncate_lines_empty_string() {
        assert_eq!(truncate_lines("", 100), "");
    }

    #[test]
    fn test_truncate_lines_single_long_line() {
        let input = "a".repeat(200);
        let result = truncate_lines(&input, 50);
        assert!(result.contains("[Diff truncated"));
    }

    #[test]
    fn test_build_user_prompt_basic() {
        let state = RepoState {
            branch: "main".to_string(),
            status: "## main".to_string(),
            log: "abc1234 initial commit".to_string(),
            diff: String::new(),
            diff_cached: String::new(),
        };
        let prompt = build_user_prompt(&state);
        assert!(prompt.contains("Branch: main"));
        assert!(prompt.contains("abc1234"));
        assert!(!prompt.contains("Staged Changes"));
        assert!(!prompt.contains("Unstaged Changes"));
    }

    #[test]
    fn test_build_user_prompt_with_diffs() {
        let state = RepoState {
            branch: "feature".to_string(),
            status: "M src/lib.rs".to_string(),
            log: "def5678 add feature".to_string(),
            diff: "+new line".to_string(),
            diff_cached: "+staged line".to_string(),
        };
        let prompt = build_user_prompt(&state);
        assert!(prompt.contains("Staged Changes"));
        assert!(prompt.contains("+staged line"));
        assert!(prompt.contains("Unstaged Changes"));
        assert!(prompt.contains("+new line"));
    }
}
