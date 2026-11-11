//! Provider-independent, optional actual counters. No key disclosure, token
//! estimation, quota polling or extra successful generation is required.
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub cache_write_input_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub source: &'static str,
    pub total_tokens_derived: bool,
}

#[derive(Clone, Default, Debug)]
struct Snapshot {
    input: Option<u64>, output: Option<u64>, total: Option<u64>,
    cached: Option<u64>, cache_write: Option<u64>, reasoning: Option<u64>,
    source: &'static str,
}
fn number(value: &Value, names: &[&str]) -> Option<u64> {
    names.iter().find_map(|name|value.get(*name).and_then(Value::as_u64))
}
impl Snapshot {
    fn any(&self) -> bool { self.input.is_some() || self.output.is_some() || self.total.is_some() || self.cached.is_some() || self.cache_write.is_some() || self.reasoning.is_some() }
    fn normalized(&self) -> Usage {
        // Anthropic input_tokens excludes cache read/write; other supported
        // protocols already include their cached prompt tokens in input.
        let input = if self.source == "anthropic" {
            self.input.and_then(|n|n.checked_add(self.cached.unwrap_or(0)))
                .and_then(|n|n.checked_add(self.cache_write.unwrap_or(0)))
        } else { self.input };
        let total = self.total.or_else(|| input.zip(self.output).and_then(|(a,b)|a.checked_add(b))
            .and_then(|n|if self.source == "google" {n.checked_add(self.reasoning.unwrap_or(0))} else {Some(n)}));
        Usage { input_tokens:input,output_tokens:self.output,total_tokens:total,
            cached_input_tokens:self.cached,cache_write_input_tokens:self.cache_write,
            reasoning_tokens:self.reasoning,source:self.source,total_tokens_derived:self.total.is_none()&&total.is_some() }
    }
    fn merge(&mut self, other: Self) {
        // Stream usage is cumulative snapshots, never per-chunk deltas.
        if other.total.is_none() && ((other.input.is_some()&&other.input!=self.input)
            || (other.output.is_some()&&other.output!=self.output)
            || (other.reasoning.is_some()&&other.reasoning!=self.reasoning)) { self.total=None; }
        if other.input.is_some(){self.input=other.input;}
        if other.output.is_some(){self.output=other.output;}
        if other.total.is_some(){self.total=other.total;}
        if other.cached.is_some(){self.cached=other.cached;}
        if other.cache_write.is_some(){self.cache_write=other.cache_write;}
        if other.reasoning.is_some(){self.reasoning=other.reasoning;}
        self.source=other.source;
    }
}
fn generic(value:&Value, source:&'static str)->Snapshot {
    Snapshot {
        input:number(value,&["prompt_tokens","input_tokens"]),
        output:number(value,&["completion_tokens","output_tokens"]),total:number(value,&["total_tokens"]),
        cached:number(&value["prompt_tokens_details"],&["cached_tokens"])
            .or_else(||number(&value["input_tokens_details"],&["cached_tokens"]))
            .or_else(||number(value,&["prompt_cache_hit_tokens","cache_read_input_tokens"])),
        cache_write:number(&value["prompt_tokens_details"],&["cache_write_tokens"])
            .or_else(||number(value,&["cache_creation_input_tokens"])),
        reasoning:number(&value["completion_tokens_details"],&["reasoning_tokens"])
            .or_else(||number(&value["output_tokens_details"],&["reasoning_tokens"]))
            .or_else(||number(value,&["reasoning_tokens"])),source,
    }
}
fn snapshot(event:&Value)->Option<Snapshot> {
    for key in ["usageMetadata","usage_metadata"] {
        if let Some(value)=event.get(key) {
            let result=Snapshot{input:number(value,&["promptTokenCount","prompt_token_count"]),
                output:number(value,&["candidatesTokenCount","candidates_token_count"]),
                total:number(value,&["totalTokenCount","total_token_count"]),
                cached:number(value,&["cachedContentTokenCount","cached_content_token_count"]),
                reasoning:number(value,&["thoughtsTokenCount","thoughts_token_count"]),source:"google",..Default::default()};
            if result.any(){return Some(result);}
        }
    }
    let anthropic=matches!(event["type"].as_str(),Some("message_start"|"message_delta"));
    for (value,source) in [(&event["usage"],if anthropic {"anthropic"}else{"openai_compatible"}),
        (&event["response"]["usage"],"openai_compatible"),
        (&event["message"]["usage"],"anthropic"),(&event["x_groq"]["usage"],"groq")] {
        let result=generic(value,source);
        if result.any(){return Some(result);}
    }
    let ollama=Snapshot{input:number(event,&["prompt_eval_count"]),output:number(event,&["eval_count"]),
        cached:number(event,&["prompt_eval_cached_count"]),source:"ollama",..Default::default()};
    ollama.any().then_some(ollama)
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Report {
    pub request_id:String, pub message_id:String, pub provider:String, pub model:String,
    pub status:&'static str, pub usage:Option<Usage>,
}

/// Scope guard emits exactly one report even on cancellation, stream errors or
/// early returns. Missing counters stay null, never a fabricated zero.
pub(crate) struct RequestUsage {
    snapshot:Snapshot, report:Report, sink:Box<dyn FnMut(Report)+Send>,
    emitted:bool, interrupted:bool,
}
impl RequestUsage {
    pub(crate) fn new(provider:&str,model:&str,message_id:&str,sink:impl FnMut(Report)+Send+'static)->Self {
        static NEXT:AtomicU64=AtomicU64::new(1);
        Self {snapshot:Snapshot::default(),report:Report{request_id:NEXT.fetch_add(1,Ordering::Relaxed).to_string(),
            message_id:message_id.into(),provider:provider.into(),model:model.into(),status:"unavailable",usage:None},
            sink:Box::new(sink),emitted:false,interrupted:false}
    }
    pub(crate) fn observe(&mut self,event:&Value) {
        if let Some(value)=snapshot(event){self.snapshot.merge(value);}
        let error=event.get("error").is_some_and(|value|!value.is_null()&&value!=false&&value.as_str()!=Some(""));
        if error || event["type"]=="error" {self.interrupted=true;}
    }
    pub(crate) fn observe_tail(&mut self,tail:&str) {
        // Some compatible servers omit a final newline. Usage-only tail frames
        // must not be lost merely because the normal text loop did not see '\n'.
        for line in tail.lines() {
            let line=line.trim();let data=line.strip_prefix("data:").unwrap_or(line).trim();
            if let Ok(event)=serde_json::from_str::<Value>(data){self.observe(&event);}
        }
    }
    pub(crate) fn interrupt(&mut self){self.interrupted=true;}
    pub(crate) fn finish(&mut self,completed:bool) {
        if self.emitted{return;}
        let usage=self.snapshot.any().then(||self.snapshot.normalized());
        self.report.status=if !completed||self.interrupted {"interrupted"}
            else if usage.is_none(){"unavailable"}
            else if usage.as_ref().is_some_and(|u|u.input_tokens.is_some()&&u.output_tokens.is_some()&&u.total_tokens.is_some()) {"reported"}
            else {"partial"};
        self.report.usage=usage;
        self.emitted=true;(self.sink)(self.report.clone());
    }
}
impl Drop for RequestUsage {fn drop(&mut self){self.finish(false);}}

pub(crate) fn provider_label(base:&str,openrouter:bool,local:bool)->&'static str {
    if openrouter {return "OpenRouter";}
    let host=reqwest::Url::parse(base).ok().and_then(|url|url.host_str().map(str::to_owned)).unwrap_or_default();
    match host.as_str() {
        "api.openai.com"=>"OpenAI", "openrouter.ai"=>"OpenRouter",
        "api.deepseek.com"=>"DeepSeek", "api.groq.com"=>"Groq",
        "api.mistral.ai"=>"Mistral", "api.x.ai"=>"xAI",
        "api.together.xyz"|"api.together.ai"=>"Together",
        _ if host.ends_with(".openai.azure.com")=>"Azure OpenAI-compatible",
        _ if local=>"Local / OpenAI-compatible", _=>"Custom / OpenAI-compatible",
    }
}

/// Every Chat Completions endpoint is asked for usage, including custom/local.
/// Only an explicit pre-stream option rejection permits one compatibility retry.
pub(crate) struct StreamUsageOptions {key:String, enabled:bool}
fn rejected_endpoints()->&'static Mutex<HashSet<String>> {
    static CACHE:OnceLock<Mutex<HashSet<String>>>=OnceLock::new();
    CACHE.get_or_init(||Mutex::new(HashSet::new()))
}
impl StreamUsageOptions {
    pub(crate) fn new(base:&str,model:&str,credential:&str)->Self {
        // Only an opaque digest is retained in RAM. Capability rejection for
        // one gateway credential must not disable usage for a different key.
        let credential_scope=crate::intelligence::digest(credential.as_bytes());
        let key=crate::intelligence::digest(&serde_json::to_vec(&(base,model,credential_scope)).unwrap());
        let enabled=!rejected_endpoints().lock().is_ok_and(|cache|cache.contains(&key));
        Self{key,enabled}
    }
    pub(crate) fn enabled(&self)->bool{self.enabled}
    pub(crate) fn accept_rejection(&mut self,status:u16,body:&str)->bool {
        if !self.enabled||!matches!(status,400|422){return false;}
        let text=body.to_lowercase();
        let names=text.contains("stream_options")||text.contains("include_usage");
        let rejection=["unsupported","not supported","unknown","unrecognized","unrecognised","unexpected","not permitted","not allowed","extra_forbidden","extra inputs"].iter().any(|term|text.contains(term));
        if !names||!rejection{return false;}
        self.enabled=false;
        if let Ok(mut cache)=rejected_endpoints().lock(){if cache.len()<128{cache.insert(self.key.clone());}}
        true
    }
}

#[cfg(test)] mod tests {
    use super::*;use serde_json::json;use std::sync::Arc;
    fn tracker()->(RequestUsage,Arc<Mutex<Vec<Report>>>) {
        let reports=Arc::new(Mutex::new(Vec::new()));let sink=reports.clone();
        (RequestUsage::new("custom","test","reply",move|report|sink.lock().unwrap().push(report)),reports)
    }
    #[test]fn openai_router_and_custom_snapshots_are_not_added_twice(){
        let (mut tracker,reports)=tracker();
        tracker.observe(&json!({"usage":null}));
        for _ in 0..3{tracker.observe(&json!({"choices":[],"error":null,"usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"prompt_tokens_details":{"cached_tokens":60,"cache_write_tokens":4},"completion_tokens_details":{"reasoning_tokens":5}}}));}
        tracker.finish(true);tracker.finish(true);drop(tracker);
        let reports=reports.lock().unwrap();assert_eq!(reports.len(),1);
        assert_eq!(reports[0].status,"reported");let usage=reports[0].usage.as_ref().unwrap();
        assert_eq!(usage.total_tokens,Some(120));assert_eq!(usage.cached_input_tokens,Some(60));
        assert_eq!(usage.cache_write_input_tokens,Some(4));
    }
    #[test]fn google_thinking_and_cache_keep_provider_total(){
        let result=snapshot(&json!({"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":10,"thoughtsTokenCount":20,"totalTokenCount":130,"cachedContentTokenCount":40}})).unwrap().normalized();
        assert_eq!(result.total_tokens,Some(130));assert_eq!(result.output_tokens,Some(10));assert_eq!(result.reasoning_tokens,Some(20));
        let partial=snapshot(&json!({"usage_metadata":{"prompt_token_count":100,"total_token_count":100}})).unwrap().normalized();
        assert_eq!(partial.output_tokens,None);
    }
    #[test]fn groq_ollama_and_responses_style_are_normalized(){
        let groq=snapshot(&json!({"x_groq":{"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}})).unwrap().normalized();
        assert_eq!(groq.source,"groq");assert_eq!(groq.total_tokens,Some(7));
        let ollama=snapshot(&json!({"done":true,"prompt_eval_count":11,"eval_count":18,"prompt_eval_cached_count":4})).unwrap().normalized();
        assert_eq!(ollama.total_tokens,Some(29));assert!(ollama.total_tokens_derived);
        let response=snapshot(&json!({"response":{"usage":{"input_tokens":9,"output_tokens":3,"total_tokens":12,"input_tokens_details":{"cached_tokens":2}}}})).unwrap().normalized();
        assert_eq!(response.cached_input_tokens,Some(2));assert_eq!(response.total_tokens,Some(12));
    }
    #[test]fn split_cumulative_usage_merges_without_double_counting_cache(){
        let (mut tracker,reports)=tracker();
        tracker.observe(&json!({"type":"message_start","message":{"usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":50,"cache_creation_input_tokens":10}}}));
        tracker.observe(&json!({"type":"message_delta","usage":{"output_tokens":3}}));
        tracker.observe(&json!({"type":"message_delta","usage":{"output_tokens":8}}));
        tracker.finish(true);
        let reports=reports.lock().unwrap();let usage=reports[0].usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens,Some(160));assert_eq!(usage.output_tokens,Some(8));assert_eq!(usage.total_tokens,Some(168));
    }
    #[test]fn unknown_partial_zero_and_interrupted_are_distinct(){
        let (mut tracker,reports)=tracker();tracker.finish(true);
        assert_eq!(reports.lock().unwrap()[0].status,"unavailable");assert!(reports.lock().unwrap()[0].usage.is_none());
        let (mut partial,reports)=self::tracker();partial.observe(&json!({"usage":{"prompt_tokens":7}}));partial.finish(true);
        let reports=reports.lock().unwrap();assert_eq!(reports[0].status,"partial");assert_eq!(reports[0].usage.as_ref().unwrap().output_tokens,None);
        let (mut zero,reports)=self::tracker();zero.observe(&json!({"usage":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}}));zero.finish(true);
        assert_eq!(reports.lock().unwrap()[0].status,"reported");
        let (mut stopped,reports)=self::tracker();stopped.observe(&json!({"usage":{"prompt_tokens":12}}));drop(stopped);
        assert_eq!(reports.lock().unwrap()[0].status,"interrupted");assert_eq!(reports.lock().unwrap()[0].usage.as_ref().unwrap().input_tokens,Some(12));
    }
    #[test]fn malformed_negative_or_string_counts_are_not_invented(){
        assert!(snapshot(&json!({"usage":{"prompt_tokens":-1,"completion_tokens":"20","total_tokens":2.5}})).is_none());
        assert_eq!(snapshot(&json!({"usage":{"prompt_tokens":u64::MAX,"completion_tokens":1}})).unwrap().normalized().total_tokens,None);
    }
    #[test]fn last_usage_tail_and_stream_error_emit_once(){
        let (mut tracker,reports)=tracker();
        tracker.observe_tail("data:{\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5}}");
        tracker.observe(&json!({"error":{"message":"failed after tokens"}}));tracker.finish(true);drop(tracker);
        assert_eq!(reports.lock().unwrap().len(),1);assert_eq!(reports.lock().unwrap()[0].status,"interrupted");
        assert_eq!(reports.lock().unwrap()[0].usage.as_ref().unwrap().total_tokens,Some(5));
    }
    #[test]fn partial_snapshots_do_not_reuse_a_stale_total_or_drop_known_cache(){
        let (mut tracker,reports)=tracker();
        tracker.observe(&json!({"usageMetadata":{"promptTokenCount":10,"totalTokenCount":10}}));
        tracker.observe(&json!({"usageMetadata":{"candidatesTokenCount":3,"thoughtsTokenCount":2}}));
        tracker.finish(true);
        let reports=reports.lock().unwrap();let usage=reports[0].usage.as_ref().unwrap();
        assert_eq!(usage.total_tokens,Some(15));assert!(usage.total_tokens_derived);
        let (mut cached,reports)=self::tracker();cached.observe(&json!({"usage":{"cache_read_input_tokens":9}}));cached.finish(true);
        let reports=reports.lock().unwrap();assert_eq!(reports[0].status,"partial");
        assert_eq!(reports[0].usage.as_ref().unwrap().cached_input_tokens,Some(9));
    }
    #[test]fn labels_are_separate_from_capability_negotiation_and_do_not_expose_keys(){
        assert_eq!(provider_label("https://api.deepseek.com/v1",false,false),"DeepSeek");
        assert_eq!(provider_label("http://127.0.0.1:1234/v1",false,true),"Local / OpenAI-compatible");
        assert_eq!(provider_label("https://unknown.example/v1?key=not-a-real-credential",false,false),"Custom / OpenAI-compatible");
    }
    #[test]fn all_endpoints_request_usage_with_only_one_specific_option_retry(){
        for base in ["http://localhost:11434/v1","http://127.0.0.1:1234/v1","https://api.deepseek.com/v1","https://api.groq.com/openai/v1","https://openrouter.ai/api/v1","https://api.openai.com/v1","https://custom.example/v1"] {
            assert!(StreamUsageOptions::new(base,"usage-test","").enabled());
        }
        let base="https://unsupported.example/v1?api_key=not-a-real-credential";
        let mut policy=StreamUsageOptions::new(base,"test","fixture-one");
        assert!(!policy.key.contains("not-a-real-credential"));
        for status in [401,403,429,500] {assert!(!policy.accept_rejection(status,"unsupported stream_options"));}
        assert!(!policy.accept_rejection(400,"unsupported model"));
        assert!(policy.accept_rejection(422,"Extra inputs are not permitted: stream_options.include_usage"));
        assert!(!policy.enabled());assert!(!policy.accept_rejection(422,"unsupported stream_options"));
        assert!(!StreamUsageOptions::new(base,"test","fixture-one").enabled());
        assert!(StreamUsageOptions::new(base,"test","fixture-two").enabled());
    }
}
