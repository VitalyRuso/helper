pub mod analyzer;
pub mod candidate_builder;
pub mod category_detector;
pub mod errors;
pub mod fact_extractor;
pub mod intake;
pub mod output;

pub use analyzer::analyze_source;
pub use candidate_builder::build_candidate;
pub use category_detector::detect_category;
pub use errors::{KnowledgeToolError, KnowledgeToolResult};
pub use fact_extractor::extract_facts;
pub use shared_contracts::*;
