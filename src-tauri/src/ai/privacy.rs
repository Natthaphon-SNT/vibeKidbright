//! Explicit consent for exporting private tool context. Embeddings always stay local.
use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};

pub(crate) fn consent() -> bool {
    super::config::read_config()["ai_allow_private_context"]
        .as_bool()
        .unwrap_or(false)
}
pub(crate) fn endpoint_is_loopback(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false),
        None => false,
    }
}
pub(crate) fn permitted(endpoint: &str) -> bool {
    consent() || endpoint_is_loopback(endpoint)
}
pub(crate) fn remote_history(
    messages: &[crate::ai_chat::ChatMessage],
) -> Vec<crate::ai_chat::ChatMessage> {
    // A previous local/provider session can contain private tool results. With no
    // export consent, start the remote turn from only the latest user-supplied text.
    messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .cloned()
        .into_iter()
        .collect()
}
pub(crate) fn bundled_source(path: &str, bytes: &[u8]) -> bool {
    static HASHES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    HASHES
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../docs/ai-engineering/bundled-source-hashes.json"
            ))
            .expect("checked bundled source manifest")
        })
        .get(path)
        .is_some_and(|hash| Some(hash.clone()) == normalized_text_hash(bytes))
}
pub(crate) fn normalized_text_hash(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes)
        .ok()
        .map(|text| crate::intelligence::digest(text.replace("\r\n", "\n").as_bytes()))
}
pub(crate) const REQUIRED: &str = "Private project/document context is blocked for this remote provider. In AI Provider Settings, explicitly enable sharing private context, or select a loopback local provider. Do not work around this with another tool.";
#[tauri::command]
pub async fn get_private_context_consent() -> Result<bool, String> {
    Ok(consent())
}
#[tauri::command]
pub async fn set_private_context_consent(
    app_handle: tauri::AppHandle,
    allowed: bool,
) -> Result<(), String> {
    super::config::update_config(|value| value["ai_allow_private_context"] = Value::Bool(allowed))?;
    if !allowed {
        use tauri::Manager;
        if let Some(state) = app_handle.try_state::<crate::ai_chat::AiAbortState>() {
            state.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_detection_cannot_be_spoofed_by_url_text() {
        for url in [
            "http://localhost:11434/v1",
            "http://127.0.0.1:1234/v1",
            "http://[::1]:11434/v1",
        ] {
            assert!(endpoint_is_loopback(url));
        }
        for url in [
            "https://localhost.evil.test/v1",
            "https://remote.test/127.0.0.1",
            "https://127.0.0.1@remote.test/v1",
            "http://192.168.1.5/v1",
            "file://localhost/x",
        ] {
            assert!(!endpoint_is_loopback(url));
        }
    }
    #[test]
    fn changed_or_user_added_docs_are_not_public_by_filename() {
        assert!(!bundled_source(
            "support_scope.md",
            b"private changed source"
        ));
        assert!(!bundled_source("user-private.c", b"private"));
        assert!(bundled_source(
            "support_scope.md",
            include_bytes!("../../../resources/knowledge_base/support_scope.md")
        ));
    }
    #[test]
    fn remote_turn_without_consent_does_not_reuse_private_tool_history() {
        use crate::ai_chat::ChatMessage;
        let history = vec![
            ChatMessage {
                role: "tool".into(),
                content: serde_json::json!("private_source"),
            },
            ChatMessage {
                role: "assistant".into(),
                content: serde_json::json!("private explanation"),
            },
            ChatMessage {
                role: "user".into(),
                content: serde_json::json!("Explain GPIO"),
            },
        ];
        let result = remote_history(&history);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Explain GPIO");
    }
    #[test]
    fn public_provenance_survives_checkout_line_endings_but_not_content_edits() {
        assert_eq!(
            normalized_text_hash(b"one\r\ntwo\r\n"),
            normalized_text_hash(b"one\ntwo\n")
        );
        assert_ne!(
            normalized_text_hash(b"one\ntwo\n"),
            normalized_text_hash(b"one\nprivate\n")
        );
    }
}
