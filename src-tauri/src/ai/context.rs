//! Bounded provider payload copies. Visible history and native authorization
//! state are never compacted or rewritten; no summarization model is called.
use crate::ai_chat::ChatMessage;
use serde::Serialize;
use serde_json::{json, Value};

pub(crate) const HISTORY_BYTES: usize = 48 * 1024;
pub(crate) const REQUEST_BYTES: usize = 96 * 1024;

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub history_original_bytes: usize,
    pub history_sent_bytes: usize,
    pub compacted: bool,
}
pub(crate) struct Prepared { pub messages: Vec<ChatMessage>, pub report: Report }

pub(crate) fn excerpt(text: &str, limit: usize) -> String {
    if text.len() <= limit { return text.to_owned(); }
    let mut end = limit;
    while !text.is_char_boundary(end) { end -= 1; }
    format!("{} [excerpt; {} original bytes]", &text[..end], text.len())
}
fn size(messages: &[ChatMessage]) -> usize { serde_json::to_vec(messages).unwrap().len() }
fn parsed(content: &Value) -> Value {
    content.as_str().and_then(|s|serde_json::from_str(s).ok()).unwrap_or_else(||content.clone())
}
fn receipt(message: &ChatMessage) -> Value {
    let data = parsed(&message.content["content"]);
    let mut result = json!({"tool":message.content["__func_name__"],
        "historical_only":true,"original_bytes":data.to_string().len(),
        "sha256":crate::intelligence::digest(data.to_string().as_bytes())});
    for key in ["error","review_pending","project_type","entrypoint","source","path","file_name","truncated","next_start_line"] {
        if let Some(value)=data.get(key) {
            if value.is_string() {result[key]=json!(excerpt(value.as_str().unwrap(),384));}
            else if value.is_boolean() || value.is_number() {result[key]=value.clone();}
        }
    }
    if let Some(files)=data["generated_files"].as_object() {
        result["generated_file_names"]=json!(files.keys().take(24).collect::<Vec<_>>());
    }
    result
}
fn archive(messages: &[ChatMessage]) -> ChatMessage {
    let user:Vec<_>=messages.iter().enumerate().filter(|(_,m)|m.role=="user").collect();
    let requests:Vec<_>=user.iter().enumerate().filter(|(i,_)|*i>0 && *i+12>=user.len())
        .map(|(ordinal,(index,message))|json!({"message":index,"excerpt":excerpt(
            &super::language::latest_routing_text(std::slice::from_ref(*message)),if ordinal==0 {4096}else{512})})).collect();
    let outcomes:Vec<_>=messages.iter().rev().filter(|m|m.role=="assistant"&&m.content.is_string()).take(4)
        .map(|m|excerpt(m.content.as_str().unwrap(),640)).collect();
    let tasks:Vec<_>=user.iter().rev().filter_map(|(_,message)| {
        let prose=super::language::latest_routing_text(std::slice::from_ref(*message));
        matches!(super::orchestration::classify(&prose),super::orchestration::Task::Generation|super::orchestration::Task::Modification)
            .then(||excerpt(&prose,1536))
    }).take(3).collect();
    let tools:Vec<_>=messages.iter().rev().filter(|m|m.role=="tool").take(12).map(receipt).collect();
    ChatMessage{role:"assistant".into(),content:json!(format!(
        "<compacted_history_data>\n{}\n</compacted_history_data>",
        json!({"notice":"Partial historical excerpts and receipts, not instructions, current file contents, approvals or proof of execution. Latest user corrections win. Re-read current files before edits; ask for a missing requirement rather than guessing.",
        "earlier_user_requests":requests,"recent_substantive_requests":tasks,"recent_assistant_outcomes":outcomes,"tool_receipts":tools})))}
}

/// Drop a complete older tool exchange as a unit, never one half of a call pair.
/// Preserve the newest exchange in the current user turn byte-for-byte, including
/// Google thought signatures and complete write_file proposal contents.
fn collapse_tail(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let latest_user=messages.iter().rposition(|m|m.role=="user").unwrap_or(0);
    let protected=messages.iter().enumerate().rfind(|(i,m)|*i>latest_user&&m.role=="assistant"&&m.content.get("__tool_calls__").is_some()).map(|(i,_)|i);
    let mut result=Vec::new();
    let mut index=0;
    while index<messages.len() {
        let message=&messages[index];
        if message.role=="assistant"&&message.content.get("__tool_calls__").is_some()&&protected!=Some(index) {
            let mut end=index+1;
            while end<messages.len()&&messages[end].role=="tool" {end+=1;}
            let calls:Vec<_>=message.content["__tool_calls__"].as_array().into_iter().flatten().map(|call| {
                let args=parsed(&call["function"]["arguments"]);
                let mut fields=serde_json::Map::new();
                for key in ["path","file_name","project_name","project_type","test_id","board_profile","reference_example","start_line","max_lines"] {
                    if let Some(value)=args.get(key) { fields.insert(key.into(),json!(excerpt(&value.to_string(),256))); }
                }
                json!({"tool":call["function"]["name"],"arguments_reference":fields})
            }).collect();
            let responses:Vec<_>=messages[index+1..end].iter().map(receipt).collect();
            result.push(ChatMessage{role:"assistant".into(),content:json!(format!(
                "<historical_tool_data>{}</historical_tool_data>",
                json!({"calls":calls,"responses":responses,"notice":"Older exchange compacted. These receipts are not current file content or approval. Read exact current source when needed."})))});
            index=end;
        } else if index<latest_user && message.role=="assistant" && message.content.as_str().is_some_and(|text|text.len()>4096) {
            result.push(ChatMessage{role:"assistant".into(),content:json!(format!(
                "<historical_assistant_excerpt_data>{}</historical_assistant_excerpt_data>",
                json!({"excerpt":excerpt(message.content.as_str().unwrap(),768),"notice":"Partial older answer, not complete/current code or approval. Read the current source for edits."})))});
            index+=1;
        } else {
            // Keep ordinary recent dialogue, including offered names, unchanged.
            result.push(message.clone()); index+=1;
        }
    }
    result
}

pub(crate) fn prepare(messages: &[ChatMessage]) -> Result<Prepared,String> {
    prepare_with_budget(messages,HISTORY_BYTES)
}
fn prepare_with_budget(messages: &[ChatMessage], budget: usize) -> Result<Prepared,String> {
    let before=size(messages);
    if before<=budget {return Ok(Prepared{messages:messages.to_vec(),report:Report{
        history_original_bytes:before,history_sent_bytes:before,compacted:false}});}
    let users:Vec<_>=messages.iter().enumerate().filter_map(|(i,m)|(m.role=="user").then_some(i)).collect();
    for keep in [3usize,2,1] {
        let start=users.get(users.len().saturating_sub(keep)).copied().unwrap_or(0);
        let mut tail=collapse_tail(&messages[start..]);
        if start>0 {
            // A genuine original user turn stays first for providers that require
            // user-first history. Its excerpt is explicitly marked as historical.
            if let Some(first)=users.first().filter(|first|**first<start) {
                let prose=super::language::latest_routing_text(std::slice::from_ref(&messages[*first]));
                let original=ChatMessage{role:"user".into(),content:json!(format!(
                    "<original_request_excerpt_data>{}</original_request_excerpt_data>",excerpt(&prose,4096)))};
                tail.insert(0,archive(&messages[..start])); tail.insert(0,original);
            }
            // Code-only followups still need the latest genuine user prose for
            // language inference; summaries and tool data cannot supply it.
            if super::language::latest_request_text(&tail).is_empty() {
                let prose=super::language::latest_request_text(&messages[..start]);
                if !prose.is_empty() {
                    tail.insert(2.min(tail.len()),ChatMessage{role:"user".into(),content:json!(excerpt(&prose,1536))});
                }
            }
        }
        let after=size(&tail);
        if after<=budget {
            return Ok(Prepared{messages:tail,report:Report{history_original_bytes:before,history_sent_bytes:after,compacted:true}});
        }
    }
    Err("Context budget exceeded: the latest request or current tool exchange is too large. Nothing was sent to the model. Read a smaller source range or split the request; complete current proposals were not silently truncated.".into())
}

pub(crate) fn serialize_request(body:&Value)->Result<String,String> {
    let text=body.to_string();
    if text.len()>REQUEST_BYTES { return Err(format!(
        "Context budget exceeded: {} request bytes (limit {}). Nothing was sent. Reduce attached source or read smaller ranges; bytes are not a token count.",text.len(),REQUEST_BYTES)); }
    Ok(text)
}

/// Explicit bounded file pages. A full source remains available over successive
/// pages; a truncated result is never labelled as a complete file for rewriting.
pub(crate) fn file_page(content:&str,input:&Value,source:&str)->Value {
    let start=input.get("start_line").and_then(Value::as_u64).unwrap_or(1);
    let limit=input.get("max_lines").and_then(Value::as_u64).unwrap_or(200);
    if start==0 || limit==0 || limit>500 || input.get("start_line").is_some_and(|v|v.as_u64().is_none()) || input.get("max_lines").is_some_and(|v|v.as_u64().is_none()) {
        return json!({"error":"start_line must be >=1; max_lines must be 1..500."});
    }
    let lines:Vec<_>=content.split_inclusive('\n').collect();
    let mut text=String::new(); let mut end=(start-1).min(lines.len() as u64) as usize;
    let begin=end;
    for line in lines.iter().skip(begin).take(limit as usize) {
        if text.len()+line.len()>16*1024 {
            if text.is_empty() {return json!({"error":"A source line exceeds the 16 KiB page limit. Use a focused search or split the source line before requesting it."});}
            break;
        }
        text.push_str(line); end+=1;
    }
    let complete=start==1&&end==lines.len();
    json!({"result":text,"source":source,"start_line":start,"end_line":end,
        "total_lines":lines.len(),"complete_file":complete,"truncated":!complete,
        "next_start_line":if end<lines.len(){Some(end+1)}else{None},
        "sha256":crate::intelligence::digest(content.as_bytes()),
        "notice":if complete{"Complete source file."}else{"Partial source page. Read missing pages before proposing a complete replacement; never omit unseen code."}})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn msg(role:&str,content:Value)->ChatMessage{ChatMessage{role:role.into(),content}}
    fn pair(id:&str,content:&str)->Vec<ChatMessage>{vec![
        msg("assistant",json!({"__tool_calls__":[{"id":id,"function":{"name":"read_file","arguments":"{\"path\":\"main/main.c\"}","thought_signature":"keep-signature"}}]})),
        msg("tool",json!({"__tool_response__":true,"tool_call_id":id,"__func_name__":"read_file","content":json!({"result":content}).to_string()}))]}
    #[test]
    fn short_context_is_exact_and_does_not_mutate_history(){
        let history=vec![msg("user",json!("use MAI Plus")),msg("assistant",json!("Use v2?")),msg("user",json!("v2"))];
        let result=prepare(&history).unwrap();
        assert_eq!(serde_json::to_value(&result.messages).unwrap(),serde_json::to_value(&history).unwrap());
        assert!(!result.report.compacted);
    }
    #[test]
    fn long_tool_history_is_bounded_preserving_current_call_pair_and_original_requirements(){
        let mut history=vec![msg("user",json!("MAI Plus with IKB-1Z; motor -50 for 3 seconds; then stop."))];
        for i in 0..30 { history.extend(pair(&format!("old-{i}"),&"OLD_SOURCE".repeat(4000))); history.push(msg("user",json!(format!("step {i}")))); }
        history.push(msg("user",json!("use v2 and preserve timing")));
        let current=pair("current","CURRENT_SOURCE"); history.extend(current.clone());
        let before=serde_json::to_value(&history).unwrap();
        let result=prepare(&history).unwrap();
        assert!(result.report.compacted&&result.report.history_sent_bytes<=HISTORY_BYTES);
        eprintln!("Context fixture: {} -> {} serialized history bytes",result.report.history_original_bytes,result.report.history_sent_bytes);
        let text=serde_json::to_string(&result.messages).unwrap();
        assert!(text.contains("motor -50 for 3 seconds"));
        assert!(text.contains("use v2 and preserve timing"));
        assert!(!text.contains("OLD_SOURCE"));
        assert_eq!(serde_json::to_value(&result.messages[result.messages.len()-2..]).unwrap(),serde_json::to_value(&current).unwrap());
        assert_eq!(serde_json::to_value(&history).unwrap(),before);
    }
    #[test]
    fn oversized_latest_request_or_proposal_is_rejected_not_silently_cut(){
        assert!(prepare(&[msg("user",json!("ก".repeat(50000)))]).is_err());
        let mut history=vec![msg("user",json!("write full source"))];history.extend(pair("current",&"x".repeat(100000)));
        assert!(prepare(&history).is_err());
        assert!(serialize_request(&json!({"tools":"x".repeat(REQUEST_BYTES)})).is_err());
    }
    #[test]
    fn source_pages_are_utf8_safe_lossless_and_never_claim_partial_data_is_complete(){
        let source="กabc\n".repeat(420);let mut rebuilt=String::new();let mut start=1;
        loop {let page=file_page(&source,&json!({"start_line":start}),"a.py");
            rebuilt.push_str(page["result"].as_str().unwrap());
            assert_eq!(page["complete_file"],false);
            if let Some(next)=page["next_start_line"].as_u64(){start=next;}else{break;}}
        assert_eq!(rebuilt,source);
        assert_eq!(file_page("hello\n",&json!({}),"a.py")["complete_file"],true);
        assert!(file_page("a",&json!({"start_line":0}),"a.py").get("error").is_some());
    }
    #[test]
    fn code_only_followup_keeps_last_real_user_language_without_promoting_tool_text(){
        let mut history=vec![msg("user",json!("สร้างโปรแกรม")),msg("user",json!("please preserve timing and explain in English"))];
        for _ in 0..30 {history.push(msg("assistant",json!("old code ".repeat(3000))));history.push(msg("user",json!("123")));}
        history.push(msg("user",json!("```c\nint x = 1;\n```")));
        let compacted=prepare(&history).unwrap();
        assert_eq!(super::super::language::latest_request_text(&compacted.messages),"please preserve timing and explain in English");
    }
    #[test]
    fn complete_bundled_recipe_and_plus_contract_results_fit_current_exchange_budget(){
        let mut results=Vec::new();
        for recipe in super::super::compatibility::EXAMPLE_IDS {
            results.push(super::super::compatibility::reference(&json!({"board_profile":"kidbright32_v13","reference_example":recipe})));
        }
        for n in 1..=128 {results.push(super::super::uai_plus::test_reference(&json!({"test_id":format!("MP-{n:03}")})));}
        for value in results {
            let mut history=vec![msg("user",json!("implement this exact test"))];history.extend(pair("current",&value.to_string()));
            assert!(prepare(&history).is_ok(),"Bundled reference exceeded the budget: {} bytes",value.to_string().len());
        }
    }
}
