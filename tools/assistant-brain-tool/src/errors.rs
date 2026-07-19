use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum AssistantBrainError {
    EmptyPrompt,
    UnsafeRules(String),
    DirectActivationRejected,
    CandidateAlreadyReviewed,
}

impl fmt::Display for AssistantBrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => write!(f, "system prompt is required"),
            Self::UnsafeRules(rule) => write!(f, "unsafe safety rules: {rule}"),
            Self::DirectActivationRejected => {
                write!(f, "direct activation is rejected; use a change candidate")
            }
            Self::CandidateAlreadyReviewed => write!(f, "candidate was already reviewed"),
        }
    }
}

impl std::error::Error for AssistantBrainError {}

pub type AssistantBrainResult<T> = Result<T, AssistantBrainError>;
