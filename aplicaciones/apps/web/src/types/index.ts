export type Category = {
  id: string;
  title_ru: string;
  slug: string;
  description_ru: string;
  icon: string;
  sort_order: number;
};

export type Article = {
  id: string;
  category_id: string | null;
  title_ru: string;
  slug: string;
  summary_ru: string;
  body_ru_markdown: string;
  tags: string[];
  legal_risk_level: string;
};

export type Guide = {
  id: string;
  title_ru: string;
  slug: string;
  summary_ru: string;
  target_audience: string;
  required_documents: string[];
  steps: string[];
  deadlines: string[];
  fees: string[];
  where_to_submit: string;
  common_mistakes: string[];
  risks: string[];
  official_sources: string[];
};

export type ChatResponse = {
  answer: string;
  sources: Array<{
    file_name: string;
    source_file: string;
    chunk_index: number;
    score: number;
  }>;
  remaining_guest_questions: number;
  unlocked: boolean;
};

export type AdminKnowledgeSource = {
  id: string;
  title: string;
  source_type: string;
  original_path: string | null;
  source_url: string | null;
  raw_text: string | null;
  trust_level: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type AdminKnowledgeFact = {
  id: string;
  source_id: string;
  fact_type: string;
  title_ru: string;
  text_ru: string;
  original_text: string;
  confidence: number;
  source_location: string | null;
  created_at: string;
};

export type AdminKnowledgeCandidate = {
  id: string;
  source_id: string | null;
  candidate_type: string;
  title_ru: string;
  summary_ru: string;
  body_ru_markdown: string;
  category_slug: string | null;
  risk_level: string;
  status: string;
  review_note: string | null;
  article_id: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminKnowledgeCandidateDetails = {
  candidate: AdminKnowledgeCandidate;
  source: AdminKnowledgeSource | null;
  facts: AdminKnowledgeFact[];
};

export type AdminAssistantProfile = {
  id: string;
  name: string;
  slug: string;
  description: string;
  active_prompt_version_id: string | null;
  active_policy_version_id: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export type AdminAssistantCandidate = {
  id: string;
  assistant_profile_id: string;
  candidate_type: string;
  title: string;
  description: string;
  proposed_payload_json: unknown;
  reason: string;
  risk_level: string;
  status: string;
  review_note: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminAssistantNote = {
  id: string;
  note_type: string;
  title: string;
  body: string;
  status: string;
  candidate_id: string | null;
  created_at: string;
  updated_at: string;
};
