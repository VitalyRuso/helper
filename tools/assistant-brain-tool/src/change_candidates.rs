use crate::{AssistantBrainError, AssistantBrainResult};
use serde_json::Value;
use shared_contracts::{AssistantChangeCandidate, AssistantChangeType, ReviewStatus, RiskLevel};

pub fn create_change_candidate(
    assistant_profile_id: &str,
    candidate_type: AssistantChangeType,
    title: &str,
    description: &str,
    proposed_payload_json: Value,
    reason: &str,
    risk_level: RiskLevel,
) -> AssistantChangeCandidate {
    AssistantChangeCandidate {
        id: title.to_lowercase().replace(' ', "-"),
        assistant_profile_id: assistant_profile_id.to_owned(),
        candidate_type,
        title: title.to_owned(),
        description: description.to_owned(),
        proposed_payload_json,
        reason: reason.to_owned(),
        risk_level,
        status: ReviewStatus::Draft,
    }
}

pub fn approve_candidate(
    mut candidate: AssistantChangeCandidate,
) -> AssistantBrainResult<AssistantChangeCandidate> {
    if candidate.status != ReviewStatus::Draft {
        return Err(AssistantBrainError::CandidateAlreadyReviewed);
    }
    candidate.status = ReviewStatus::Approved;
    Ok(candidate)
}

pub fn reject_candidate(
    mut candidate: AssistantChangeCandidate,
) -> AssistantBrainResult<AssistantChangeCandidate> {
    if candidate.status != ReviewStatus::Draft {
        return Err(AssistantBrainError::CandidateAlreadyReviewed);
    }
    candidate.status = ReviewStatus::Rejected;
    Ok(candidate)
}
