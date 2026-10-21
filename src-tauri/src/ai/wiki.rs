//! Wiki requests use native networking and the current user's provider settings.
use crate::ai::config::{get_secure_key, read_config};
use serde_json::{json, Value};

pub(crate) fn completion_url(base: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(base.trim()).map_err(|_| "Enter a valid AI provider URL in Settings.")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
        || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("AI provider URL must be an HTTP(S) endpoint without credentials or query parameters.".into());
    }
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/chat/completions") { path.to_string() }
        else if path.is_empty() { "/v1/chat/completions".into() }
        else { format!("{path}/chat/completions") };
    url.set_path(&path);
    Ok(url.into())
}

#[tauri::command]
pub async fn wiki_ai_request(system_prompt: String, user_prompt: String) -> Result<String, String> {
    if user_prompt.trim().is_empty() || system_prompt.len() + user_prompt.len() > 200_000 {
        return Err("Enter a Wiki request shorter than 200 KB.".into());
    }
    let config = read_config();
    let provider = config["provider"].as_str().unwrap_or("openai");
    let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(120)).build().map_err(|error| error.to_string())?;
    let request = if provider == "google" {
        let key = get_secure_key("vibekidbright-google", "google_api_key");
        if key.is_empty() { return Err("Configure your Google API key in Vibe Coder Settings first.".into()); }
        let model = config["google_model"].as_str().unwrap_or("gemini-2.5-flash");
        client.post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", urlencoding::encode(model)))
            .header("x-goog-api-key", key)
            .json(&json!({ "systemInstruction": { "parts": [{ "text": system_prompt }] },
                "contents": [{ "role": "user", "parts": [{ "text": user_prompt }] }] }))
    } else {
        let (base, model, key) = match provider {
            "openrouter" => ("https://openrouter.ai/api/v1", config["openrouter_model"].as_str().unwrap_or("google/gemini-2.5-flash:free"), get_secure_key("vibekidbright-openrouter", "openrouter_api_key")),
            "openai" | "local" => (config["base_url"].as_str().unwrap_or("https://api.openai.com/v1"), config["model"].as_str().unwrap_or("gpt-4.1"), get_secure_key("vibekidbright-openai", "api_key")),
            _ => return Err("Choose a supported AI provider in Settings.".into()),
        };
        if provider != "local" && key.is_empty() { return Err("Configure your AI API key in Vibe Coder Settings first.".into()); }
        let request = client.post(completion_url(base)?).json(&json!({ "model": model,
            "messages": [{ "role": "system", "content": system_prompt }, { "role": "user", "content": user_prompt }] }));
        if key.is_empty() || provider == "local" { request } else { request.bearer_auth(key) }
    };
    let response = request.send().await.map_err(|_| "Cannot reach your AI provider. Check the connection and provider settings.")?;
    if !response.status().is_success() { return Err(format!("AI provider returned HTTP {}. Check your key, model and quota.", response.status().as_u16())); }
    let data: Value = response.json().await.map_err(|_| "AI provider returned an invalid response.")?;
    let text = if provider == "google" {
        data["candidates"][0]["content"]["parts"].as_array().map(|parts| parts.iter()
            .filter_map(|part| part["text"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
    } else { data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string() };
    if text.trim().is_empty() { return Err("AI provider returned no article text. Try another request or model.".into()); }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::completion_url;
    #[test]
    fn provider_urls_are_user_configured_and_normalized_without_a_developer_host() {
        assert_eq!(completion_url("http://127.0.0.1:1234").unwrap(), "http://127.0.0.1:1234/v1/chat/completions");
        assert_eq!(completion_url("https://example.org/custom/v1/").unwrap(), "https://example.org/custom/v1/chat/completions");
        assert_eq!(completion_url("https://example.org/v1/chat/completions").unwrap(), "https://example.org/v1/chat/completions");
        for bad in ["file:///tmp/key", "https://user:password@example.org", "https://example.org?key=test"] {
            assert!(completion_url(bad).is_err());
        }
    }
}
