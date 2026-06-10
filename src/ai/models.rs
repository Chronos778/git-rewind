use reqwest::Client;

pub async fn discover_best_model(client: &Client, api_base: &str, api_key: &str, vendor: &str) -> String {
    let default_fallback = match vendor {
        "GROQ" => "llama-3.3-70b-versatile",
        "GEMINI" => "gemini-2.0-flash",
        _ => "gpt-4o-mini",
    };

    let res = match client
        .get(format!("{}/models", api_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return default_fallback.to_string(),
    };

    if !res.status().is_success() {
        return default_fallback.to_string();
    }

    let body: serde_json::Value = match res.json().await {
        Ok(b) => b,
        Err(_) => return default_fallback.to_string(),
    };

    let mut available_models = Vec::new();
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                let lower_id = id.to_lowercase();
                // Filter out non-text/vision utility models
                if lower_id.contains("embedding") || lower_id.contains("whisper") || lower_id.contains("dall-e") || lower_id.contains("vision") || lower_id.contains("tts") || lower_id.contains("audio") || lower_id.contains("moderation") {
                    continue;
                }
                available_models.push(id.to_string());
            }
        }
    }

    if available_models.is_empty() {
        return default_fallback.to_string();
    }

    match vendor {
        "GROQ" => {
            if let Some(m) = available_models.iter().find(|m| m.contains("versatile") || (m.contains("llama") && m.contains("70b"))) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| (m.contains("llama") && m.contains("8b")) || m.contains("mixtral")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("llama")) {
                return m.clone();
            }
            available_models[0].clone()
        }
        "GEMINI" => {
            if let Some(m) = available_models.iter().find(|m| m.contains("flash")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("gemini")) {
                return m.clone();
            }
            available_models[0].clone()
        }
        _ => {
            if let Some(m) = available_models.iter().find(|m| m.contains("mini")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("turbo")) {
                return m.clone();
            }
            if let Some(m) = available_models.iter().find(|m| m.contains("gpt-4")) {
                return m.clone();
            }
            available_models[0].clone()
        }
    }
}
