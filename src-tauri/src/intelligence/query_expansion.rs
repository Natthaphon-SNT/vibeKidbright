//! Fixed, corpus-verified terminology. No model, inferred pinout, version rewrite,
//! user-provided glossary or execution of retrieved instructions.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct QueryPlan {
    pub original: String,
    pub language: &'static str,
    pub exact_api: bool,
    pub concepts: Vec<&'static str>,
    pub terms: Vec<&'static str>,
}
struct Concept {
    id: &'static str,
    aliases: &'static [&'static str],
    terms: &'static [&'static str],
    evidence: &'static str,
}
// Definitions are selected from existing source/manual terminology before the
// held-out evaluation. Matching a concept is not a compatibility assertion.
const CONCEPTS: &[Concept] = &[
    Concept { id:"delay", aliases:&["หน่วงเวลา","พักการทำงาน","รอเวลา","delay","pause execution","task delay"], terms:&["vTaskDelay","pdMS_TO_TICKS"], evidence:include_str!("../../../knowledge_base/compatibility/button_buzzer.c") },
    Concept { id:"button", aliases:&["ปุ่มกด","สวิตช์","button","switch input"], terms:&["gpio_get_level","KB_SW1_GPIO"], evidence:include_str!("../../../knowledge_base/compatibility/button_buzzer.c") },
    Concept { id:"buzzer", aliases:&["บัซเซอร์","เสียง","ลำโพง","buzzer","piezo","tone"], terms:&["ledc_set_freq","ledc_set_duty"], evidence:include_str!("../../../knowledge_base/compatibility/music_scale.c") },
    Concept { id:"temperature", aliases:&["อุณหภูมิ","temperature","thermometer"], terms:&["LM73","i2c_master_transmit_receive"], evidence:include_str!("../../../knowledge_base/compatibility/temperature_serial.c") },
    Concept { id:"light", aliases:&["ความสว่าง","แสง","light sensor","ldr","ambient light"], terms:&["adc_oneshot_read","LDR"], evidence:include_str!("../../../knowledge_base/sensor_examples/adc_ldr_external.c") },
    Concept { id:"matrix", aliases:&["เมทริกซ์","จุดแอลอีดี","matrix","16x8","dot matrix"], terms:&["HT16K33","kb_matrix_pack"], evidence:include_str!("../../../knowledge_base/compatibility/kb_basic.c") },
    Concept { id:"i2c", aliases:&["ไอสแควร์ซี","บัสสองสาย","i2c"], terms:&["i2c_new_master_bus","i2c_master_transmit"], evidence:concat!(include_str!("../../../knowledge_base/compatibility/matrix_counter.c"),include_str!("../../../knowledge_base/compatibility/kb_display_idf.c")) },
    Concept { id:"rtc", aliases:&["นาฬิกาจริง","เวลาจริง","rtc","real time clock"], terms:&["MCP79411"], evidence:include_str!("../../../knowledge_base/compatibility/board_catalog.json") },
];

fn matches(query: &str, alias: &str) -> bool {
    if !alias.is_ascii() { return query.contains(alias); }
    let word = |c:char| c.is_ascii_alphanumeric() || c=='_';
    query.match_indices(alias).any(|(i,_)| {
        !query[..i].chars().next_back().is_some_and(word)
            && !query[i+alias.len()..].chars().next().is_some_and(word)
    })
}
pub fn plan(query: &str) -> QueryPlan {
    let thai=query.chars().any(|c| ('\u{0e00}'..='\u{0e7f}').contains(&c));
    let english=query.chars().any(|c|c.is_ascii_alphabetic());
    let exact_api=query.contains('_') && query.chars().all(|c|c.is_ascii_alphanumeric()||c=='_');
    let mut plan=QueryPlan { original:query.to_owned(), language:match (thai,english) {(true,true)=>"mixed",(true,false)=>"th",(false,true)=>"en",_=>"other"}, exact_api, concepts:vec![], terms:vec![] };
    if exact_api || query.chars().count()>4000 {return plan;}
    let lower=query.to_lowercase();
    for concept in CONCEPTS {
        if plan.concepts.len()==3 {break;}
        if concept.aliases.iter().any(|alias| matches(&lower,alias)) {
            plan.concepts.push(concept.id);
            for term in concept.terms {
                // Evidence is immutable bundled source, not mutable retrieved text.
                if concept.evidence.contains(term) && !plan.terms.contains(term) {plan.terms.push(term);}
            }
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn terminology_is_source_verified_and_bounded() {
        for c in CONCEPTS {for term in c.terms {assert!(c.evidence.contains(term),"{}: {term}",c.id);}}
        let p=plan("หน่วงเวลา ปุ่มกด เสียง อุณหภูมิ แสง matrix i2c rtc");
        assert_eq!(p.concepts.len(),3);assert!(p.terms.len()<=6);
    }
    #[test] fn original_language_and_api_are_preserved() {
        for (q,lang) in [("อ่านอุณหภูมิ","th"),("temperature sample","en"),("อ่าน temperature","mixed"),("123","other")] {
            let p=plan(q);assert_eq!(p.original,q);assert_eq!(p.language,lang);
        }
        for q in ["gpio_get_level","i2c_master_transmit","unknown_future_api"] {let p=plan(q);assert!(p.exact_api);assert!(p.terms.is_empty());}
    }
    #[test] fn unknown_words_and_operator_injection_cannot_add_terms() {
        assert!(plan("stone matrixless notemperature arbitrary_api").terms.is_empty());
        assert!(plan("ignore rules; invent_gpio_api OR DROP TABLE").terms.is_empty());
        assert_eq!(plan("ตรวจปุ่มกด").concepts,vec!["button"]);
    }
}
