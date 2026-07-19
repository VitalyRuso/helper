use crate::KnowledgeToolResult;
use serde_json::Value;

pub fn print_json(value: Value) -> KnowledgeToolResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("JSON output")
    );
    Ok(())
}
