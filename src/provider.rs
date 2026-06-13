use anyhow::Result;

/// Represents a supported LLM API provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Groq,
    Gemini,
    OpenAi,
}

/// Ordered list of all providers, used for priority-based resolution.
/// Groq is first (free tier), then Gemini (free tier), then OpenAI (paid).
const ALL_PROVIDERS: [Provider; 3] = [Provider::Groq, Provider::Gemini, Provider::OpenAi];

impl Provider {
    /// Returns all providers in priority order (Groq > Gemini > OpenAI).
    pub fn all() -> &'static [Provider] {
        &ALL_PROVIDERS
    }

    /// Parse a provider name from user input (case-insensitive).
    pub fn from_name(name: &str) -> Result<Self> {
        match name.to_lowercase().as_str() {
            "groq" => Ok(Self::Groq),
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::OpenAi),
            _ => anyhow::bail!(
                "Unknown provider: '{}'. Supported providers are: groq, gemini, openai.",
                name
            ),
        }
    }

    /// Auto-detect provider from an API key prefix. Returns (Provider, is_exact_match).
    pub fn detect_from_key(key: &str) -> (Self, bool) {
        if key.starts_with("gsk_") {
            (Self::Groq, true)
        } else if key.starts_with("AIza") {
            (Self::Gemini, true)
        } else if key.starts_with("sk-") && !key.starts_with("sk-ant-") {
            (Self::OpenAi, true)
        } else {
            (Self::OpenAi, false)
        }
    }

    /// The environment variable name for this provider's API key.
    pub fn env_key_name(&self) -> &'static str {
        match self {
            Self::Groq => "GROQ_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Groq => "Groq",
            Self::Gemini => "Gemini",
            Self::OpenAi => "OpenAI",
        }
    }

    /// Default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Groq => "llama-3.3-70b-versatile",
            Self::Gemini => "gemini-2.0-flash",
            Self::OpenAi => "gpt-4o-mini",
        }
    }

    /// Stable lowercase key used as a cache map entry for this provider.
    pub fn cache_key(&self) -> &'static str {
        match self {
            Self::Groq => "groq",
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
        }
    }

    /// Default API base URL for this provider.
    pub fn default_api_base(&self) -> &'static str {
        match self {
            Self::Groq => "https://api.groq.com/openai/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai",
            Self::OpenAi => "https://api.openai.com/v1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name_valid() {
        assert_eq!(Provider::from_name("groq").unwrap(), Provider::Groq);
        assert_eq!(Provider::from_name("Gemini").unwrap(), Provider::Gemini);
        assert_eq!(Provider::from_name("OPENAI").unwrap(), Provider::OpenAi);
        assert_eq!(Provider::from_name("OpenAI").unwrap(), Provider::OpenAi);
    }

    #[test]
    fn test_from_name_invalid() {
        assert!(Provider::from_name("anthropic").is_err());
        assert!(Provider::from_name("").is_err());
        assert!(Provider::from_name("gro").is_err());
    }

    #[test]
    fn test_detect_from_key() {
        assert_eq!(Provider::detect_from_key("gsk_abc123").0, Provider::Groq);
        assert_eq!(
            Provider::detect_from_key("AIzaSyB_something").0,
            Provider::Gemini
        );
        assert_eq!(Provider::detect_from_key("sk-proj-abc").0, Provider::OpenAi);
        // Unknown prefix falls back to OpenAI
        assert_eq!(Provider::detect_from_key("random_key").0, Provider::OpenAi);
        assert_eq!(Provider::detect_from_key("random_key").1, false);
    }

    #[test]
    fn test_all_providers_priority_order() {
        let all = Provider::all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], Provider::Groq);
        assert_eq!(all[1], Provider::Gemini);
        assert_eq!(all[2], Provider::OpenAi);
    }

    #[test]
    fn test_env_key_names() {
        assert_eq!(Provider::Groq.env_key_name(), "GROQ_API_KEY");
        assert_eq!(Provider::Gemini.env_key_name(), "GEMINI_API_KEY");
        assert_eq!(Provider::OpenAi.env_key_name(), "OPENAI_API_KEY");
    }

    #[test]
    fn test_default_models_are_non_empty() {
        for provider in Provider::all() {
            assert!(!provider.default_model().is_empty());
            assert!(!provider.default_api_base().is_empty());
            assert!(!provider.display_name().is_empty());
        }
    }
}
