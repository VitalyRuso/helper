use assistant_brain_tool::{
    create_change_candidate, create_profile, reject_direct_activation, AssistantChangeType,
    RiskLevel,
};
use serde_json::json;
use std::{env, fs};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match (
        args.first().map(String::as_str),
        args.get(1).map(String::as_str),
    ) {
        (Some("profile"), Some("create")) => {
            let profile = create_profile(
                &option(&args, "--name").unwrap_or_else(|| "Spain Immigration Helper".to_owned()),
                &option(&args, "--slug").unwrap_or_else(|| "spain-immigration-helper".to_owned()),
                &option(&args, "--description").unwrap_or_default(),
            );
            print_json(json!(profile));
        }
        (Some("prompt"), Some("validate")) => {
            let file = option(&args, "--file").ok_or("--file is required")?;
            let prompt = fs::read_to_string(file)?;
            let ok = !prompt.trim().is_empty();
            print_json(json!({ "valid": ok }));
        }
        (Some("candidate"), Some("create")) => {
            let candidate = create_change_candidate(
                &option(&args, "--profile")
                    .unwrap_or_else(|| "spain-immigration-helper".to_owned()),
                parse_candidate_type(
                    &option(&args, "--type").unwrap_or_else(|| "architecture_note".to_owned()),
                ),
                &option(&args, "--title").unwrap_or_else(|| "Assistant change".to_owned()),
                &option(&args, "--description").unwrap_or_default(),
                json!({}),
                &option(&args, "--reason").unwrap_or_default(),
                RiskLevel::Medium,
            );
            print_json(json!(candidate));
        }
        (Some("candidate"), Some("review")) => {
            print_json(
                json!({ "id": option(&args, "--id").unwrap_or_default(), "status": "draft" }),
            );
        }
        (Some("runtime-config"), _) => {
            let profile =
                option(&args, "--profile").unwrap_or_else(|| "spain-immigration-helper".to_owned());
            print_json(json!({ "profile_slug": profile, "fallback": true }));
        }
        (Some("prompt"), Some("activate")) => {
            reject_direct_activation()?;
        }
        _ => {
            eprintln!("usage: assistant-brain-tool profile create | prompt validate --file prompt.md | candidate create --type prompt_change | candidate review --id ... | runtime-config --profile ...");
            std::process::exit(2);
        }
    }
    Ok(())
}

fn print_json(value: serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("JSON output")
    );
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn parse_candidate_type(input: &str) -> AssistantChangeType {
    match input {
        "prompt_change" => AssistantChangeType::PromptChange,
        "policy_change" => AssistantChangeType::PolicyChange,
        "tool_change" => AssistantChangeType::ToolChange,
        "answer_format_change" => AssistantChangeType::AnswerFormatChange,
        "data_source_change" => AssistantChangeType::DataSourceChange,
        _ => AssistantChangeType::ArchitectureNote,
    }
}
