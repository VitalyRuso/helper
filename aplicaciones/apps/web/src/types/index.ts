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

export type AdminLegalReviewTask = {
  id: string;
  legal_change_id: string | null;
  document_id: string | null;
  title: string;
  status: string;
  priority: string;
  ai_summary: string;
};

export type AdminLegalReviewContext = {
  task: AdminLegalReviewTask | null;
  legal_change: {
    id: string;
    change_type: string;
    priority: string;
    affected_sections: unknown;
    detected_summary: string;
    status: string;
  };
  document: {
    id: string;
    official_id: string | null;
    eli_id: string | null;
    title: string;
    document_type: string;
    legal_area: string;
    procedure_key: string | null;
    source_url: string | null;
    status: string;
  };
  diff: {
    id: string;
    diff_type: string;
    summary: string;
    diff_json: unknown;
  };
  affected_sections: Array<{
    id: string;
    version_id: string;
    stable_section_key: string;
    section_number: string | null;
    title: string;
    text_content: string;
  }>;
  currentness: {
    reviewed_version_id: string;
    current_version_id: string | null;
    reviewed_version_is_current: boolean;
    is_stale: boolean;
  };
};

export type AdminLegalKnowledgeItem = {
  id: string;
  procedure_key: string;
  topic_key: string;
  title_es: string;
  canonical_answer_es: string;
  status: string;
  approved_by: string | null;
  approved_at: string | null;
  document_id: string | null;
  document_title: string | null;
  legal_status: string | null;
  reviewed_version_is_current: boolean;
  is_stale: boolean;
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
