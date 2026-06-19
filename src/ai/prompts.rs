use crate::git::RepoState;

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
        user_prompt.push_str(&format!("- Staged Changes:\n{}\n", state.diff_cached));
    }

    if !state.diff.is_empty() {
        user_prompt.push_str(&format!("- Unstaged Changes:\n{}\n", state.diff));
    }
    user_prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_user_prompt_basic() {
        let state = RepoState {
            root: "/mock/root".to_string(),
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
            root: "/mock/root".to_string(),
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
