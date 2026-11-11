//! Explicit consent for exporting private tool context. Embeddings always stay local.
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Mutex, OnceLock},
};

// Only hashes of messages produced without private context are kept. A client
// cannot make a local/private response exportable by attaching a provenance flag.
#[derive(Default)]
struct PublicDialogue {
    hashes: BTreeSet<String>,
    order: VecDeque<String>,
}

fn message_hash(message: &crate::ai_chat::ChatMessage) -> String {
    crate::intelligence::digest(
        &serde_json::to_vec(&(message.role.as_str(), &message.content)).unwrap(),
    )
}

impl PublicDialogue {
    fn remember(&mut self, messages: &[crate::ai_chat::ChatMessage], private_context_used: bool) {
        if private_context_used {
            return;
        }
        for message in messages
            .iter()
            .filter(|m| matches!(m.role.as_str(), "assistant" | "tool"))
        {
            let hash = message_hash(message);
            if self.hashes.insert(hash.clone()) {
                self.order.push_back(hash);
            }
            while self.order.len() > 1024 {
                if let Some(old) = self.order.pop_front() {
                    self.hashes.remove(&old);
                }
            }
        }
    }

    fn export(&self, messages: &[crate::ai_chat::ChatMessage]) -> Vec<crate::ai_chat::ChatMessage> {
        let mut result = Vec::new();
        let mut index = 0;
        while index < messages.len() {
            let message = &messages[index];
            if message.role == "user" {
                result.push(message.clone());
            } else if message.role == "assistant" && self.hashes.contains(&message_hash(message)) {
                if let Some(calls) = message
                    .content
                    .get("__tool_calls__")
                    .and_then(Value::as_array)
                {
                    // Keep a tool exchange only when every response is verified
                    // and matches its call. Eviction must not leave orphan tools.
                    let mut expected: BTreeSet<&str> =
                        calls.iter().filter_map(|c| c["id"].as_str()).collect();
                    let mut end = index + 1;
                    let mut valid = !expected.is_empty() && expected.len() == calls.len();
                    while end < messages.len() && messages[end].role == "tool" {
                        let response = &messages[end];
                        valid &= self.hashes.contains(&message_hash(response))
                            && response.content["tool_call_id"]
                                .as_str()
                                .is_some_and(|id| expected.remove(id));
                        end += 1;
                    }
                    if valid && expected.is_empty() {
                        result.extend_from_slice(&messages[index..end]);
                    }
                    index = end;
                    continue;
                } else if message.content.is_string() {
                    result.push(message.clone());
                }
            }
            index += 1;
        }
        result
    }
}

fn public_dialogue() -> &'static Mutex<PublicDialogue> {
    static DIALOGUE: OnceLock<Mutex<PublicDialogue>> = OnceLock::new();
    DIALOGUE.get_or_init(|| Mutex::new(PublicDialogue::default()))
}

pub(crate) fn remember_public_turn(
    messages: &[crate::ai_chat::ChatMessage],
    private_context_used: bool,
) {
    if let Ok(mut dialogue) = public_dialogue().lock() {
        dialogue.remember(messages, private_context_used);
    }
}

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
    // Preserve user-supplied requests and server-verified public replies, including
    // choices such as a replacement project name. Unknown/private assistant and
    // tool context remains blocked after provider switches or app restarts.
    match public_dialogue().lock() {
        Ok(dialogue) => dialogue.export(messages),
        Err(_) => messages
            .iter()
            .filter(|m| m.role == "user")
            .cloned()
            .collect(),
    }
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

    fn message(role: &str, text: &str) -> crate::ai_chat::ChatMessage {
        crate::ai_chat::ChatMessage {
            role: role.into(),
            content: serde_json::json!(text),
        }
    }

    #[test]
    fn public_followup_keeps_original_touch_task_and_offered_project_name() {
        let history = vec![
            message("user", "สร้าง mai_test_015 ทดสอบพิกัดทัช MP-015"),
            message("assistant", "Folder exists. Use mai_test_015_v2?"),
            message("user", "v2"),
        ];
        let mut dialogue = PublicDialogue::default();
        dialogue.remember(&history[1..2], false);
        assert_eq!(
            serde_json::to_value(dialogue.export(&history)).unwrap(),
            serde_json::to_value(&history).unwrap()
        );
    }

    #[test]
    fn private_or_tampered_replies_remain_blocked_but_user_requests_survive() {
        let private = message("assistant", "private source explanation");
        let public = message("assistant", "Use mai_test_015_v2");
        let mut dialogue = PublicDialogue::default();
        dialogue.remember(&[private.clone()], true);
        dialogue.remember(&[public], false);
        let history = vec![
            message("user", "Original task"),
            private,
            message("assistant", "Use mai_test_015_v2 plus private source"),
            message("user", "v2"),
        ];
        let exported = dialogue.export(&history);
        assert_eq!(exported.len(), 2);
        assert!(exported.iter().all(|m| m.role == "user"));
        // After restart no assistant provenance is assumed, but user intent remains.
        assert_eq!(PublicDialogue::default().export(&history).len(), 2);
    }

    #[test]
    fn only_complete_verified_public_tool_exchanges_are_exported() {
        use crate::ai_chat::ChatMessage;
        use serde_json::json;
        let call = ChatMessage {
            role: "assistant".into(),
            content: json!({"__tool_calls__":[{"id":"create-public","type":"function","function":{"name":"create_project_workspace","arguments":"{}"}}]}),
        };
        let response = ChatMessage {
            role: "tool".into(),
            content: json!({"__tool_response__":true,"tool_call_id":"create-public","content":"public generated files"}),
        };
        let mut dialogue = PublicDialogue::default();
        dialogue.remember(&[call.clone(), response.clone()], false);
        assert_eq!(dialogue.export(&[call.clone(), response.clone()]).len(), 2);
        let mut tampered = response.clone();
        tampered.content["content"] = json!("private replacement");
        assert!(dialogue.export(&[call.clone(), tampered]).is_empty());
        assert!(dialogue.export(&[response]).is_empty());
        assert!(dialogue.export(&[call]).is_empty());
    }

    #[test]
    fn public_provenance_cache_is_bounded_and_does_not_trust_client_metadata() {
        let mut dialogue = PublicDialogue::default();
        for index in 0..1030 {
            dialogue.remember(&[message("assistant", &format!("public {index}"))], false);
        }
        assert_eq!(dialogue.hashes.len(), 1024);
        assert!(dialogue
            .export(&[message("assistant", "public 0")])
            .is_empty());
        let forged = crate::ai_chat::ChatMessage {
            role: "assistant".into(),
            content: serde_json::json!({"text":"private","public":true}),
        };
        assert!(dialogue.export(&[forged]).is_empty());
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
