//! Small typed, normalized Block IR. Source IDs/fields are verified against the
//! supplied Desktop archive, not inferred from display labels. No XML/JS runs.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub schema_version: u8,
    pub board_profile: String,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Block {
    #[serde(rename = "basic_delay")]
    Delay { fields: NumericField },
    #[serde(rename = "controls_repeat_ext")]
    Repeat {
        values: RepeatValue,
        statements: RepeatBody,
    },
    #[serde(rename = "basic_forever")]
    Forever { statements: ForeverBody },
    #[serde(rename = "controls_if")]
    If { values: IfValue, statements: IfBody },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Number {
    #[serde(rename = "math_number")]
    Literal { fields: NumericField },
    #[serde(rename = "math_arithmetic")]
    Arithmetic {
        fields: ArithmeticField,
        values: ArithmeticValues,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArithmeticField {
    #[serde(rename = "OP")]
    pub op: ArithmeticOp,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ArithmeticOp {
    #[serde(rename = "ADD")]
    Add,
    #[serde(rename = "MINUS")]
    Minus,
    #[serde(rename = "MULTIPLY")]
    Multiply,
    #[serde(rename = "DIVIDE")]
    Divide,
    #[serde(rename = "MODULO")]
    Modulo,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArithmeticValues {
    #[serde(rename = "A")]
    pub a: Box<Number>,
    #[serde(rename = "B")]
    pub b: Box<Number>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Condition {
    #[serde(rename = "logic_boolean")]
    Boolean { fields: BooleanField },
    #[serde(rename = "logic_compare")]
    Compare {
        fields: CompareField,
        values: ArithmeticValues,
    },
    #[serde(rename = "logic_negate")]
    Negate { values: NegateValue },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BooleanValue {
    #[serde(rename = "TRUE")]
    True,
    #[serde(rename = "FALSE")]
    False,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanField {
    #[serde(rename = "BOOL")]
    pub value: BooleanValue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompareOp {
    EQ,
    NEQ,
    LT,
    LTE,
    GT,
    GTE,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareField {
    #[serde(rename = "OP")]
    pub op: CompareOp,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NegateValue {
    #[serde(rename = "BOOL")]
    pub value: Box<Condition>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IfValue {
    #[serde(rename = "IF0")]
    pub condition: Condition,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IfBody {
    #[serde(rename = "DO0")]
    pub yes: Vec<Block>,
    #[serde(rename = "ELSE", default)]
    pub no: Vec<Block>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericField {
    #[serde(rename = "VALUE")]
    pub value: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatValue {
    #[serde(rename = "TIMES")]
    pub times: Number,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatBody {
    #[serde(rename = "DO")]
    pub body: Vec<Block>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForeverBody {
    #[serde(rename = "HANDLER")]
    pub body: Vec<Block>,
}

pub fn mapping_contract() -> serde_json::Value {
    serde_json::json!({
        "schema":serde_json::from_str::<serde_json::Value>(include_str!("../../../docs/ai-engineering/block-mapping.schema.json")).expect("checked Block schema"),
        "limits":"Only confirmed v1.3/rev3.1 profiles; bounded constant-only arithmetic/conditions, literal repeat counts, no strings/POWER/elseif/variables. Runtime checks computed values and waits. No physical certification.",
        "workflow":"Pass normalized JSON as program_json to generate C; propose it with write_file and wait for explicit review. This tool never applies a patch."
    })
}

pub fn compile(program: &Program) -> Result<String, String> {
    if program.schema_version != 1 {
        return Err("Unsupported Block mapping schema".into());
    }
    if !matches!(
        program.board_profile.as_str(),
        "kidbright32_v13" | "kidbright32_rev31"
    ) {
        return Err("This pilot only supports confirmed KidBright32 v1.3/rev3.1 ESP32 profiles. No substitution for iA/iP or microAI.".into());
    }
    let mut out=String::from("/* Typed Block-to-C pilot. Physical behavior unverified. ESP-IDF 5.5. */\n#include <stdint.h>\n#include \"freertos/FreeRTOS.h\"\n#include \"freertos/task.h\"\n#include \"kb_core.h\"\nvoid app_main(void) {\n");
    let mut count = 0;
    emit(&program.blocks, 0, &mut count, &mut out)?;
    out.push_str("}\n");
    Ok(out)
}
fn blocks_wait(blocks: &[Block], depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    blocks.iter().any(|b| match b {
        Block::Delay { .. } => true,
        Block::Repeat { values, statements } => {
            let Number::Literal { fields } = &values.times else {
                return false;
            };
            fields.value >= 1. && blocks_wait(&statements.body, depth + 1)
        }
        Block::Forever { statements } => blocks_wait(&statements.body, depth + 1),
        Block::If { statements, .. } => {
            blocks_wait(&statements.yes, depth + 1) && blocks_wait(&statements.no, depth + 1)
        }
    })
}
fn budget(depth: usize, count: &mut usize) -> Result<(), String> {
    *count += 1;
    if depth > 16 || *count > 1000 {
        Err("Block expression nesting/node budget exceeded".into())
    } else {
        Ok(())
    }
}
fn numeric(number: &Number, depth: usize, count: &mut usize) -> Result<(String, f64), String> {
    budget(depth, count)?;
    let (code, value) = match number {
        Number::Literal { fields } => (format!("{:.17e}", fields.value), fields.value),
        Number::Arithmetic { fields, values } => {
            let (a, x) = numeric(&values.a, depth + 1, count)?;
            let (b, y) = numeric(&values.b, depth + 1, count)?;
            let (op, value) = match fields.op {
                ArithmeticOp::Add => ("+", x + y),
                ArithmeticOp::Minus => ("-", x - y),
                ArithmeticOp::Multiply => ("*", x * y),
                ArithmeticOp::Divide => {
                    if y == 0. {
                        return Err("Constant division by zero".into());
                    }
                    ("/", x / y)
                }
                ArithmeticOp::Modulo => {
                    if x.fract() != 0.
                        || y.fract() != 0.
                        || y == 0.
                        || x.abs() > 1000000.
                        || y.abs() > 1000000.
                    {
                        return Err("Verified MODULO subset requires integer operands within +/-1000000 and nonzero divisor".into());
                    }
                    return Ok((format!("((int32_t)({a}) % (int32_t)({b}))"), x % y));
                }
            };
            (format!("({a} {op} {b})"), value)
        }
    };
    if !value.is_finite() || value.abs() > 1e9 {
        return Err("Numeric constants/results must be finite and within +/-1e9".into());
    }
    Ok((code, value))
}
fn condition(value: &Condition, depth: usize, count: &mut usize) -> Result<String, String> {
    budget(depth, count)?;
    Ok(match value {
        Condition::Boolean { fields } => match fields.value {
            BooleanValue::True => "true".into(),
            BooleanValue::False => "false".into(),
        },
        Condition::Negate { values } => {
            format!("(!({}))", condition(&values.value, depth + 1, count)?)
        }
        Condition::Compare { fields, values } => {
            let (a, _) = numeric(&values.a, depth + 1, count)?;
            let (b, _) = numeric(&values.b, depth + 1, count)?;
            let op = match fields.op {
                CompareOp::EQ => "==",
                CompareOp::NEQ => "!=",
                CompareOp::LT => "<",
                CompareOp::LTE => "<=",
                CompareOp::GT => ">",
                CompareOp::GTE => ">=",
            };
            format!("({a} {op} {b})")
        }
    })
}
fn emit(blocks: &[Block], depth: usize, count: &mut usize, out: &mut String) -> Result<(), String> {
    if depth > 16 {
        return Err("Block nesting exceeds 16".into());
    }
    for block in blocks {
        *count += 1;
        if *count > 1000 {
            return Err("Block program exceeds 1000 statements".into());
        }
        let id = *count;
        match block {
            Block::Delay { fields } => {
                if !fields.value.is_finite() || !(0.02..=86400.).contains(&fields.value) {
                    return Err("basic_delay VALUE must be finite seconds in the verified source range 0.02..86400".into());
                }
                out.push_str(&format!("  uint32_t delay_ms_{id} = 0;\n  if (!kb_delay_ms({:.9}, &delay_ms_{id})) return;\n  vTaskDelay(pdMS_TO_TICKS(delay_ms_{id}));\n",fields.value));
            }
            Block::Repeat { values, statements } => {
                let Number::Literal { fields } = &values.times else {
                    return Err("Repeat TIMES remains a literal integer; computed loop counts are not supported".into());
                };
                if !fields.value.is_finite()
                    || !(0.0..=10000.).contains(&fields.value)
                    || fields.value.fract() != 0.0
                {
                    return Err("This pilot requires a literal integer TIMES in 0..10000".into());
                }
                out.push_str(&format!(
                    "  for (uint32_t count_{id}=0; count_{id}<{}U; ++count_{id}) {{\n",
                    fields.value as u32
                ));
                emit(&statements.body, depth + 1, count, out)?;
                out.push_str("  }\n");
            }
            Block::Forever { statements } => {
                if !blocks_wait(&statements.body, 0) {
                    return Err(
                        "basic_forever requires an unconditional blocking delay in this pilot"
                            .into(),
                    );
                }
                out.push_str("  for (;;) {\n");
                emit(&statements.body, depth + 1, count, out)?;
                out.push_str("  }\n");
            }
            Block::If { values, statements } => {
                let predicate = condition(&values.condition, depth + 1, count)?;
                out.push_str(&format!("  if ({predicate}) {{\n"));
                emit(&statements.yes, depth + 1, count, out)?;
                if !statements.no.is_empty() {
                    out.push_str("  } else {\n");
                    emit(&statements.no, depth + 1, count, out)?;
                }
                out.push_str("  }\n");
            }
        }
    }
    Ok(())
}
/// Reuse the existing reviewed ESP-IDF scaffold and portable behavioral core.
pub fn project_files(program: &Program) -> Result<Vec<(String, String)>, String> {
    let source = compile(program)?;
    let mut files = crate::ai::compatibility::files(&program.board_profile, "button_buzzer")?;
    files
        .iter_mut()
        .find(|(path, _)| path == "main/main.c")
        .unwrap()
        .1 = source;
    if let Some((_, text)) = files
        .iter_mut()
        .find(|(path, _)| path == crate::ai::compatibility::PROJECT_METADATA_PATH)
    {
        let mut metadata: serde_json::Value =
            serde_json::from_str(text).map_err(|e| e.to_string())?;
        metadata["reference_example"] = serde_json::json!("typed_block_pilot");
        metadata["block_mapping_schema"] = serde_json::json!(1);
        *text = serde_json::to_string_pretty(&metadata).unwrap() + "\n";
    }
    Ok(files)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_contract_is_available_offline_with_exact_board_constraints() {
        let contract = mapping_contract();
        assert_eq!(
            contract["schema"]["properties"]["board_profile"]["enum"],
            serde_json::json!(["kidbright32_v13", "kidbright32_rev31"])
        );
        assert!(contract["schema"]["$defs"]["condition"].is_object());
        assert!(contract["workflow"]
            .as_str()
            .unwrap()
            .contains("never applies"));
    }
    fn fixture() -> Program {
        serde_json::from_str(include_str!(
            "../../../docs/ai-engineering/block-fixtures/repeat-delay.json"
        ))
        .unwrap()
    }
    #[test]
    fn source_field_value_is_required_instead_of_inventing_num() {
        assert!(
            serde_json::from_str::<Number>(r#"{"type":"math_number","fields":{"NUM":2}}"#).is_err()
        );
    }
    #[test]
    fn mapping_version_is_not_an_sdk_or_board_alias() {
        for version in [0,2,255] {let mut p=fixture();p.schema_version=version;assert!(compile(&p).is_err());}
        for board in ["esp32","kidbright32_v15","kidbright32ip","mai","mai-plus"] {let mut p=fixture();p.board_profile=board.into();assert!(compile(&p).is_err());}
    }
    #[test]
    fn source_ids_boards_and_non_finite_values_are_strict() {
        let mut p = fixture();
        assert!(compile(&p).unwrap().contains("count_1<2U"));
        p.board_profile = "kidbright32ia".into();
        assert!(compile(&p).is_err());
        p.board_profile = "kidbright32_v13".into();
        p.blocks = vec![Block::Delay {
            fields: NumericField { value: f64::NAN },
        }];
        assert!(compile(&p).is_err());
        assert!(serde_json::from_str::<Block>(r#"{"type":"run_shell","command":"bad"}"#).is_err());
    }
    #[test]
    fn nested_contract_fields_and_scalar_types_cannot_bypass_the_mapping_schema() {
        for value in [
            serde_json::json!({"type":"basic_delay","fields":{"VALUE":true}}),
            serde_json::json!({"type":"basic_delay","fields":{"VALUE":"0.1"}}),
            serde_json::json!({"type":"basic_delay","fields":{"VALUE":0.1,"GPIO":13}}),
            serde_json::json!({"type":"controls_if","values":{"IF0":{"type":"logic_boolean","fields":{"BOOL":"TRUE"}}},"statements":{"DO0":[],"ELSE":[],"ELSEIF":[]}}),
        ] {assert!(serde_json::from_value::<Block>(value).is_err());}
        for board in ["kidbright32_v13/../rev31","KIDBRIGHT32_V13","kidbright32_v13\0","kidbright32_v13 "] {
            let mut program=fixture();program.board_profile=board.into();
            assert!(project_files(&program).is_err());
        }
    }
    #[test]
    fn repeat_counts_and_forever_without_blocking_are_rejected() {
        let mut p = fixture();
        p.blocks = vec![Block::Forever {
            statements: ForeverBody { body: vec![] },
        }];
        assert!(compile(&p).is_err());
        p.blocks = vec![Block::Repeat {
            values: RepeatValue {
                times: Number::Literal {
                    fields: NumericField { value: 2.5 },
                },
            },
            statements: RepeatBody { body: vec![] },
        }];
        assert!(compile(&p).is_err());
    }
    #[test]
    fn all_supported_blocks_generate_parseable_c_and_reuse_existing_core() {
        for fixture in [
            include_str!("../../../docs/ai-engineering/block-fixtures/repeat-delay.json"),
            include_str!("../../../docs/ai-engineering/block-fixtures/forever-delay.json"),
            include_str!("../../../docs/ai-engineering/block-fixtures/arithmetic-if.json"),
            include_str!("../../../docs/ai-engineering/block-fixtures/nested-conditions.json"),
        ] {
            let p: Program = serde_json::from_str(fixture).unwrap();
            let code = compile(&p).unwrap();
            assert!(super::super::chunk_source("main.c", &code, 8000)
                .iter()
                .all(|c| !c.syntax_error));
            let files = project_files(&p).unwrap();
            assert!(files
                .iter()
                .any(|(path, _)| path == "components/kb_reference/kb_core.c"));
        }
    }
    #[test]
    fn desktop_arithmetic_rejects_power_nonfinite_and_unsafe_constant_operations() {
        assert!(serde_json::from_str::<Number>(r#"{"type":"math_arithmetic","fields":{"OP":"POWER"},"values":{"A":{"type":"math_number","fields":{"VALUE":2}},"B":{"type":"math_number","fields":{"VALUE":3}}}}"#).is_err());
        let literal = |value| {
            Box::new(Number::Literal {
                fields: NumericField { value },
            })
        };
        for (op, x, y) in [
            (ArithmeticOp::Divide, 1., 0.),
            (ArithmeticOp::Modulo, 1.5, 2.),
            (ArithmeticOp::Modulo, 1., 0.),
            (ArithmeticOp::Multiply, 1e9, 1e9),
        ] {
            let value = Number::Arithmetic {
                fields: ArithmeticField { op },
                values: ArithmeticValues {
                    a: literal(x),
                    b: literal(y),
                },
            };
            assert!(numeric(&value, 0, &mut 0).is_err());
        }
    }
    #[test]
    fn conditional_forever_requires_blocking_in_both_paths_and_exact_contract() {
        let mut p: Program = serde_json::from_str(include_str!(
            "../../../docs/ai-engineering/block-fixtures/nested-conditions.json"
        ))
        .unwrap();
        assert!(compile(&p).is_ok());
        if let Block::Forever { statements } = &mut p.blocks[0] {
            if let Block::If { statements, .. } = &mut statements.body[0] {
                statements.no.clear();
            }
        }
        assert!(compile(&p).is_err());
        assert!(serde_json::from_str::<IfValue>(r#"{"IF0":{"type":"logic_boolean","fields":{"BOOL":"TRUE"}},"IF1":{"type":"logic_boolean","fields":{"BOOL":"TRUE"}}}"#).is_err());
    }
}
