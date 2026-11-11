//! Offline mechanical checks. Every reply/pricing entry is synthetic.
//! Only the redirect test opens loopback sockets owned by this test.
use super::*;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

static IDS: AtomicUsize = AtomicUsize::new(0);
fn unique() -> String {
    format!("synthetic-{}", IDS.fetch_add(1, Ordering::SeqCst))
}
fn pricing() -> SyntheticPricing {
    SyntheticPricing {
        input_tokens_per_byte_upper: 1,
        input_nano_per_token: 0,
        output_nano_per_token: 0,
        max_output_tokens: 128,
        other_nano_per_attempt: 1_000,
        metadata_nano_upper: Some(1_000),
        all_charges_bounded: true,
    }
}
fn roots() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("repeat_delay");
    let kb = temp.path().join("kb");
    std::fs::create_dir(&fixture).unwrap();
    std::fs::create_dir(&kb).unwrap();
    for (relative, text) in fixture_files("repeat_delay").unwrap() {
        let path = fixture.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::fs::write(
        kb.join("support_scope.md"),
        include_bytes!("../../../../resources/knowledge_base/support_scope.md"),
    )
    .unwrap();
    (
        temp,
        std::fs::canonicalize(fixture).unwrap(),
        std::fs::canonicalize(kb).unwrap(),
    )
}
fn scope(fixture: &Path, kb: &Path) -> Scope {
    Scope::bind(fixture, kb, "repeat_delay", &implementation_revision()).unwrap()
}
fn session(
    temp: &Path,
    fixture: &Path,
    kb: &Path,
    price: Option<SyntheticPricing>,
) -> Arc<Session> {
    let id = unique();
    Session::create(
        &temp.join(&id),
        &id,
        scope(fixture, kb),
        Endpoint::parse("http://127.0.0.1:1/v1").unwrap(),
        Mode::Synthetic,
        price,
    )
    .unwrap()
}
fn envelope(session: Arc<Session>) -> Envelope {
    let mut route = Route::new(session, "synthetic-model-not-a-provider").unwrap();
    route.history.push(crate::ai_chat::ChatMessage {
        role: "user".into(),
        content: json!(case_prompt("grounding-th").unwrap()),
    });
    route.envelope().unwrap()
}
fn reply(status: u16, value: Value) -> Result<WireReply> {
    Ok(WireReply {
        status,
        bytes: serde_json::to_vec(&value).unwrap(),
    })
}
fn answer() -> Value {
    json!({"choices":[{"message":{"content":"Synthetic reply; no provider inference."}}],"usage":{"prompt_tokens":7,"completion_tokens":2,"total_tokens":9}})
}
fn tool(path: &str) -> Value {
    json!({"choices":[{"message":{"content":"Synthetic tool selection.","tool_calls":[{"id":"synthetic-call-1","type":"function","function":{"name":"read_file","arguments":json!({"path":path}).to_string()}}]}}]})
}
struct Fake {
    calls: AtomicUsize,
    script: Mutex<VecDeque<Result<WireReply>>>,
    bodies: Mutex<Vec<Vec<u8>>>,
}
impl Fake {
    fn new(script: Vec<Result<WireReply>>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            script: Mutex::new(script.into()),
            bodies: Mutex::new(Vec::new()),
        }
    }
}
impl Transport for Fake {
    fn mode(&self) -> Mode {
        Mode::Synthetic
    }
    fn send(&self, _: &Endpoint, _: Kind, bytes: &[u8], _: &Credential) -> Result<WireReply> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.bodies.lock().unwrap().push(bytes.to_vec());
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| reply(200, answer()))
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        eprintln!(
            "PILOT_FAKE_TRANSPORT_ATTEMPTS={}",
            self.calls.load(Ordering::SeqCst)
        );
    }
}
fn empty_credential() -> Credential {
    Credential(String::new())
}
fn events(session: &Session) -> Vec<Value> {
    std::fs::read_to_string(session.directory.join("ledger.jsonl"))
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["event"].clone())
        .collect()
}

fn retain_synthetic_capture(session: &Session, label: &str) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/native-pilot-captures");
    std::fs::create_dir_all(&directory).unwrap();
    let mut file = tempfile::Builder::new()
        .prefix(&format!("synthetic-{label}-"))
        .suffix(".jsonl")
        .tempfile_in(&directory)
        .unwrap();
    file.write_all(&std::fs::read(session.directory.join("ledger.jsonl")).unwrap())
        .unwrap();
    file.as_file().sync_all().unwrap();
    let (_, path) = file.keep().unwrap();
    let relative = path
        .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
        .unwrap();
    eprintln!(
        "PILOT_SYNTHETIC_EVIDENCE={}",
        relative.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn ten_attempts_then_eleventh_never_reaches_transport_across_cases_and_kinds() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let body = envelope(session.clone());
    let metadata = Envelope::metadata();
    let fake = Fake::new(vec![]);
    for i in 0..10 {
        let kind = [Kind::Metadata, Kind::Inference, Kind::Retry, Kind::ToolLoop][i % 4];
        let case = if i % 2 == 0 {
            "grounding-th"
        } else {
            "project-th"
        };
        session
            .dispatch(
                case,
                i as u32,
                kind,
                if kind == Kind::Metadata {
                    &metadata
                } else {
                    &body
                },
                &empty_credential(),
                &fake,
            )
            .unwrap();
    }
    assert!(matches!(
        session.dispatch(
            "generation-th",
            99,
            Kind::Retry,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Ceiling)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 10);
    let log = events(&session);
    assert_eq!(log.last().unwrap()["next_attempt_number"], 11);
    assert_eq!(log.last().unwrap()["outcome"], "blocked_before_send");
    let attempts: Vec<_> = log
        .iter()
        .filter(|e| e["outcome"] == "reserved_before_send")
        .map(|e| e["attempt_number"].as_u64().unwrap())
        .collect();
    assert_eq!(attempts, (1..=10).collect::<Vec<_>>());
}

#[test]
fn concurrent_requests_share_one_atomic_durable_reservation() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let body = Arc::new(envelope(session.clone()));
    let fake = Arc::new(Fake::new(vec![]));
    let barrier = Arc::new(std::sync::Barrier::new(24));
    let handles: Vec<_> = (0..24)
        .map(|turn| {
            let (session, body, fake, barrier) =
                (session.clone(), body.clone(), fake.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                session
                    .dispatch(
                        "project-th",
                        turn,
                        Kind::Inference,
                        &body,
                        &empty_credential(),
                        &*fake,
                    )
                    .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .filter(|v| *v)
            .count(),
        10
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 10);
    assert_eq!(session.ledger.lock().unwrap().attempts, 10);
}

#[test]
fn native_route_metadata_retry_and_tool_loop_share_budget_and_capture_serialized_context() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let fake = Fake::new(vec![
        reply(
            200,
            json!({"data":[{"id":"synthetic-model-not-a-provider"}]}),
        ),
        Err(Fault::Transport),
        reply(429, json!({"error":"synthetic retry"})),
        reply(200, tool("fixture/main/main.c")),
        reply(200, answer()),
    ]);
    let mut route = Route::new(session.clone(), "synthetic-model-not-a-provider").unwrap();
    route.metadata("grounding-th", &fake).unwrap();
    let result = route
        .turn("project-th", &fake, &empty_credential())
        .unwrap();
    assert_eq!(result["usage"]["total_tokens"], 9);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 5);
    // Another route/turn/connection handle is not a new session or new budget.
    let mut second = Route::new(session.clone(), "synthetic-model-not-a-provider").unwrap();
    for _ in 0..5 {
        second.metadata("grounding-en", &fake).unwrap();
    }
    assert!(matches!(
        second.turn("generation-th", &fake, &empty_credential()),
        Err(Fault::Ceiling)
    ));
    let log = events(&session);
    let kinds: Vec<_> = log
        .iter()
        .filter(|v| v["outcome"] == "reserved_before_send")
        .take(5)
        .map(|v| v["kind"].clone())
        .collect();
    assert_eq!(
        kinds,
        vec![
            json!("metadata"),
            json!("inference"),
            json!("retry"),
            json!("retry"),
            json!("tool_loop")
        ]
    );
    let bodies = fake.bodies.lock().unwrap();
    let last: Value = serde_json::from_slice(&bodies[4]).unwrap();
    assert!(last["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool"
            && m["content"]
                .as_str()
                .unwrap()
                .contains("fixture/main/main.c")));
    let captured = &log
        .iter()
        .find(|e| e["attempt_number"] == 5 && e["outcome"] == "reserved_before_send")
        .unwrap()["serialized_request_redacted"];
    // The record comes from the exact serialized request sent, after redaction.
    assert_eq!(*captured, redact(&last, &empty_credential()));
    retain_synthetic_capture(&session, "tool-loop");
}

#[test]
fn session_loss_corruption_and_restart_never_reset_a_budget() {
    for mutation in ["missing", "truncated", "corrupt"] {
        let (temp, fixture, kb) = roots();
        let session = session(temp.path(), &fixture, &kb, Some(pricing()));
        let fake = Fake::new(vec![]);
        let body = envelope(session.clone());
        session
            .dispatch(
                "grounding-th",
                0,
                Kind::Inference,
                &body,
                &empty_credential(),
                &fake,
            )
            .unwrap();
        let path = session.directory.join("ledger.jsonl");
        if mutation == "missing" {
            std::fs::remove_file(&path).unwrap();
        } else if mutation == "truncated" {
            let first = std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .to_owned()
                + "\n";
            std::fs::write(&path, first).unwrap();
        } else {
            std::fs::write(&path, "corrupt ledger").unwrap();
        }
        assert!(matches!(
            session.dispatch(
                "project-th",
                1,
                Kind::Inference,
                &body,
                &empty_credential(),
                &fake
            ),
            Err(Fault::Ledger)
        ));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            Session::recover(&session.directory),
            Err(Fault::Recovery)
        ));
        assert!(matches!(
            Session::create(
                &temp.path().join("different-path"),
                &session.id,
                scope(&fixture, &kb),
                Endpoint::parse("http://127.0.0.1:1/v1").unwrap(),
                Mode::Synthetic,
                Some(pricing())
            ),
            Err(Fault::Recovery)
        ));
    }
}

#[test]
fn transport_failure_consumes_budget_and_usage_never_refunds_reservations() {
    let (temp, fixture, kb) = roots();
    let mut price = pricing();
    price.other_nano_per_attempt = 600_000_000;
    let session = session(temp.path(), &fixture, &kb, Some(price));
    let body = envelope(session.clone());
    let fake = Fake::new(vec![Err(Fault::Transport)]);
    assert!(matches!(
        session.dispatch(
            "project-th",
            0,
            Kind::Inference,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Transport)
    ));
    assert!(matches!(
        session.dispatch(
            "project-th",
            1,
            Kind::Retry,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Ceiling)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(session.ledger.lock().unwrap().reserved_nano, 600_000_000);
    let mut price = pricing();
    price.input_nano_per_token = 5;
    price.output_nano_per_token = 7;
    price.other_nano_per_attempt = 11;
    assert_eq!(
        price.reserve(Kind::Inference, 100).unwrap(),
        100 * 5 + 128 * 7 + 11
    );
    price.input_nano_per_token = u64::MAX;
    assert!(matches!(
        price.reserve(Kind::Inference, 100),
        Err(Fault::Cost)
    ));
}

#[test]
fn zero_reported_usage_does_not_restore_reserved_budget() {
    let (temp, fixture, kb) = roots();
    let mut price = pricing();
    price.other_nano_per_attempt = 600_000_000;
    let session = session(temp.path(), &fixture, &kb, Some(price));
    let body = envelope(session.clone());
    let fake = Fake::new(vec![reply(
        200,
        json!({"choices":[{"message":{"content":"Synthetic zero-usage response"}}],"usage":{"total_tokens":0}}),
    )]);
    session
        .dispatch(
            "project-th",
            0,
            Kind::Inference,
            &body,
            &empty_credential(),
            &fake,
        )
        .unwrap();
    assert!(matches!(
        session.dispatch(
            "project-th",
            1,
            Kind::ToolLoop,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Ceiling)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(session.ledger.lock().unwrap().reserved_nano, 600_000_000);
}

#[test]
fn reservation_is_fsynced_before_the_transport_is_invoked() {
    struct DurableProbe {
        directory: PathBuf,
        inner: Fake,
    }
    impl Transport for DurableProbe {
        fn mode(&self) -> Mode {
            Mode::Synthetic
        }
        fn send(&self, e: &Endpoint, k: Kind, b: &[u8], c: &Credential) -> Result<WireReply> {
            let bytes = std::fs::read_to_string(self.directory.join("ledger.jsonl")).unwrap();
            let record: Value = serde_json::from_str(bytes.lines().last().unwrap()).unwrap();
            assert_eq!(record["event"]["outcome"], "reserved_before_send");
            assert_eq!(record["event"]["payload_sha256"], hash(b));
            self.inner.send(e, k, b, c)
        }
    }
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let probe = DurableProbe {
        directory: session.directory.clone(),
        inner: Fake::new(vec![]),
    };
    session
        .dispatch(
            "project-th",
            0,
            Kind::Inference,
            &envelope(session.clone()),
            &empty_credential(),
            &probe,
        )
        .unwrap();
    assert_eq!(probe.inner.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn output_limit_tampering_and_source_changes_block_before_send() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let fake = Fake::new(vec![]);
    let mut body = envelope(session.clone());
    body.max_tokens += 1;
    body.bytes = serialize_request(&body.history, &body.model, body.max_tokens).unwrap();
    assert!(matches!(
        session.dispatch(
            "project-th",
            0,
            Kind::Inference,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Cost)
    ));
    let body = envelope(session.clone());
    std::fs::write(kb.join("support_scope.md"), "private KB replacement").unwrap();
    assert!(matches!(
        session.dispatch(
            "project-th",
            1,
            Kind::Inference,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Revision)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn actual_run_rejects_missing_or_synthetic_cost_proof_even_for_localhost() {
    let (temp, fixture, kb) = roots();
    for proof in [None, Some(pricing())] {
        let id = unique();
        let directory = temp.path().join(&id);
        assert!(matches!(
            Session::create(
                &directory,
                &id,
                scope(&fixture, &kb),
                Endpoint::parse("http://localhost:1234/v1").unwrap(),
                Mode::Actual,
                proof
            ),
            Err(Fault::Cost)
        ));
        assert!(!directory.exists());
    }
    assert!(actual_preflight()
        .unwrap_err()
        .contains("Synthetic pricing cannot authorize"));
}

#[test]
fn synthetic_pricing_cannot_be_used_with_the_real_http_transport() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let real = NativeHttpTransport::new().unwrap();
    assert!(matches!(
        session.dispatch(
            "project-th",
            0,
            Kind::Inference,
            &envelope(session.clone()),
            &empty_credential(),
            &real
        ),
        Err(Fault::Cost)
    ));
    let log = events(&session);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0]["outcome"], "blocked_before_send");
    assert_eq!(session.ledger.lock().unwrap().attempts, 0);
}

#[test]
fn incomplete_synthetic_charge_bounds_block_before_transport() {
    for mode in 0..3 {
        let (temp, fixture, kb) = roots();
        let mut proof = pricing();
        if mode == 1 {
            proof.all_charges_bounded = false;
        } else if mode == 2 {
            proof.metadata_nano_upper = None;
        }
        let session = session(
            temp.path(),
            &fixture,
            &kb,
            if mode == 0 { None } else { Some(proof) },
        );
        let fake = Fake::new(vec![]);
        assert!(matches!(
            session.dispatch(
                "project-th",
                0,
                Kind::Metadata,
                &Envelope::metadata(),
                &empty_credential(),
                &fake
            ),
            Err(Fault::Cost)
        ));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            events(&session).last().unwrap()["outcome"],
            "blocked_before_send"
        );
    }
}

#[test]
fn public_fixture_is_bound_to_generator_revision_and_pinned_kb_bytes() {
    let (_temp, fixture, kb) = roots();
    assert!(matches!(
        Scope::bind(&fixture, &kb, "repeat_delay", "unapproved-revision"),
        Err(Fault::Revision)
    ));
    let approved = scope(&fixture, &kb);
    assert!(approved.read("kb/support_scope.md").is_ok());
    std::fs::write(
        fixture.join("main/private.c"),
        "private data without secret-like text",
    )
    .unwrap();
    for id in [
        "fixture/main/private.c",
        "fixture/../private.c",
        "fixture/main/../../outside",
        "fixture/C:/Users/user/private.txt",
        "/etc/passwd",
        "kb/../private.md",
    ] {
        assert!(matches!(approved.read(id), Err(Fault::Scope)));
    }
    assert!(matches!(
        approved.approve_tool_result(
            "fixture/main/main.c",
            "private data without secret-like text"
        ),
        Err(Fault::Scope)
    ));
    std::fs::write(fixture.join("main/main.c"), "edited source").unwrap();
    assert!(matches!(
        approved.read("fixture/main/main.c"),
        Err(Fault::Revision)
    ));
    assert!(matches!(
        Scope::bind(&fixture, &kb, "repeat_delay", &implementation_revision()),
        Err(Fault::Scope)
    ));
}

#[test]
fn final_serialized_payload_and_unprovenanced_tool_context_are_blocked() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let fake = Fake::new(vec![]);
    let mut body = envelope(session.clone());
    body.bytes.extend_from_slice(b"private appended source");
    assert!(matches!(
        session.dispatch(
            "project-th",
            0,
            Kind::Inference,
            &body,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Payload)
    ));
    let mut route = Route::new(session.clone(), "synthetic-model-not-a-provider").unwrap();
    route.history.push(crate::ai_chat::ChatMessage{role:"tool".into(),content:json!({"__tool_response__":true,"tool_call_id":"synthetic-call-1","__func_name__":"read_file","content":"private source with no regex markers"})});
    let forged = route.envelope().unwrap();
    assert!(matches!(
        session.dispatch(
            "project-th",
            1,
            Kind::ToolLoop,
            &forged,
            &empty_credential(),
            &fake
        ),
        Err(Fault::Scope)
    ));
    route.history = vec![crate::ai_chat::ChatMessage {
        role: "assistant".into(),
        content: json!("unprovenanced prior private session"),
    }];
    assert!(matches!(
        session.dispatch(
            "project-th",
            2,
            Kind::Inference,
            &route.envelope().unwrap(),
            &empty_credential(),
            &fake
        ),
        Err(Fault::Scope)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert!(
        !std::fs::read_to_string(session.directory.join("ledger.jsonl"))
            .unwrap()
            .contains("private source with no regex markers")
    );
}

#[test]
fn malicious_tool_path_never_gets_reflected_into_a_followup_request() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let fake = Fake::new(vec![reply(200, tool("fixture/../../private.txt"))]);
    let mut route = Route::new(session.clone(), "synthetic-model-not-a-provider").unwrap();
    assert!(matches!(
        route.turn("project-th", &fake, &empty_credential()),
        Err(Fault::Scope)
    ));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        events(&session).last().unwrap()["outcome"],
        "blocked_before_send"
    );
}

#[test]
fn known_credentials_url_keys_and_unknown_response_text_are_never_evidence() {
    let (temp, fixture, kb) = roots();
    let session = session(temp.path(), &fixture, &kb, Some(pricing()));
    let body = envelope(session.clone());
    let credential = Credential("SYNTHETIC_HEADER_SECRET_DO_NOT_CAPTURE".into());
    let fake = Fake::new(vec![reply(
        200,
        json!({"Authorization":credential.0,"api_key":"fixture-key","unknown_secret_field":"unrecognizable-private-value","echo":"https://example.invalid/v1?key=fixture-url-key","choices":[{"message":{"content":"arbitrary confidential response text"}}],"usage":{"total_tokens":9}}),
    )]);
    session
        .dispatch("project-th", 0, Kind::Inference, &body, &credential, &fake)
        .unwrap();
    let log = std::fs::read_to_string(session.directory.join("ledger.jsonl")).unwrap();
    for secret in [
        &credential.0,
        "fixture-key",
        "unrecognizable-private-value",
        "fixture-url-key",
        "arbitrary confidential response text",
    ] {
        assert!(!log.contains(secret));
    }
    assert!(log.contains("UNTRUSTED_RESPONSE_TEXT_OMITTED"));
    assert!(events(&session).last().unwrap()["response_redacted"]["usage"]["total_tokens"] == 9);
    retain_synthetic_capture(&session, "redaction");
    for url in [
        "https://u:pass@example.invalid/v1",
        "https://example.invalid/v1?key=secret",
        "https://example.invalid/v1#key",
    ] {
        assert!(Endpoint::parse(url).is_err());
    }
}

#[test]
fn native_http_redirect_is_one_attempt_and_never_follows_to_a_second_destination() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let destination_port = destination.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = [0u8; 4096];
        let count = socket.read(&mut bytes).unwrap();
        assert!(String::from_utf8_lossy(&bytes[..count]).starts_with("GET /v1/models "));
        write!(socket,"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{destination_port}/v1/models?key=synthetic-redirect-secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    struct LocalTest(NativeHttpTransport);
    impl Transport for LocalTest {
        fn mode(&self) -> Mode {
            Mode::Synthetic
        }
        fn send(&self, e: &Endpoint, k: Kind, b: &[u8], c: &Credential) -> Result<WireReply> {
            assert_eq!(e.0.host_str(), Some("127.0.0.1"));
            self.0.send(e, k, b, c)
        }
    }
    let (temp, fixture, kb) = roots();
    let id = unique();
    let session = Session::create(
        &temp.path().join(&id),
        &id,
        scope(&fixture, &kb),
        Endpoint::parse(&format!("http://{address}/v1")).unwrap(),
        Mode::Synthetic,
        Some(pricing()),
    )
    .unwrap();
    let local = LocalTest(NativeHttpTransport::new().unwrap());
    assert!(matches!(
        session.dispatch(
            "grounding-th",
            0,
            Kind::Metadata,
            &Envelope::metadata(),
            &empty_credential(),
            &local
        ),
        Err(Fault::Redirect)
    ));
    server.join().unwrap();
    assert!(
        matches!(destination.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock)
    );
    assert_eq!(session.ledger.lock().unwrap().attempts, 1);
    assert!(
        !std::fs::read_to_string(session.directory.join("ledger.jsonl"))
            .unwrap()
            .contains("synthetic-redirect-secret")
    );
    eprintln!("PILOT_LOCAL_TEST_HTTP_ATTEMPTS=1");
}

#[test]
fn symlink_or_junction_escape_is_rejected_before_read_or_send() {
    let (temp, fixture, kb) = roots();
    let approved = scope(&fixture, &kb);
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("main.c"), "private outside source").unwrap();
    let directory = fixture.join("main");
    let saved = fixture.join("saved-main");
    std::fs::rename(&directory, &saved).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &directory).unwrap();
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&directory)
            .arg(&outside)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    assert!(matches!(
        approved.read("fixture/main/main.c"),
        Err(Fault::Scope)
    ));
    // Remove only the link; keep the real outside directory/data intact.
    #[cfg(windows)]
    std::fs::remove_dir(&directory).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(&directory).unwrap();
    assert_eq!(
        std::fs::read_to_string(outside.join("main.c")).unwrap(),
        "private outside source"
    );
    std::fs::rename(saved, directory).unwrap();
}
