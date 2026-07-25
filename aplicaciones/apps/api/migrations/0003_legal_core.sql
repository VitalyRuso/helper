CREATE TABLE IF NOT EXISTS legal_sources (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_key text NOT NULL UNIQUE,
  title text NOT NULL,
  authority text NOT NULL,
  jurisdiction text NOT NULL DEFAULT 'ES',
  source_type text NOT NULL,
  base_url text,
  acquisition_method text NOT NULL DEFAULT 'manual',
  trust_level text NOT NULL DEFAULT 'official',
  language text NOT NULL DEFAULT 'es',
  enabled boolean NOT NULL DEFAULT true,
  terms_or_reuse_notes text NOT NULL DEFAULT '',
  parser_version text NOT NULL DEFAULT 'manual-v1',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT legal_sources_trust_level_chk
    CHECK (trust_level IN ('official', 'official_secondary', 'court', 'manual_import', 'unknown')),

  CONSTRAINT legal_sources_source_type_chk
    CHECK (source_type IN ('boe', 'migraciones', 'eurlex', 'bocm', 'cendoj', 'manual'))
);

CREATE TABLE IF NOT EXISTS legal_documents (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id uuid NOT NULL REFERENCES legal_sources(id) ON DELETE RESTRICT,
  official_id text,
  eli_id text,
  title text NOT NULL,
  document_type text NOT NULL,
  legal_area text NOT NULL DEFAULT 'immigration',
  procedure_key text,
  source_url text,
  status text NOT NULL DEFAULT 'active',
  first_seen_at timestamptz NOT NULL DEFAULT now(),
  last_checked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT legal_documents_status_chk
    CHECK (status IN ('draft', 'active', 'amended', 'repealed', 'superseded', 'archived')),

  CONSTRAINT legal_documents_document_type_chk
    CHECK (document_type IN ('law', 'royal_decree', 'instruction', 'info_sheet', 'form', 'court_decision', 'guide', 'manual'))
);

CREATE INDEX IF NOT EXISTS legal_documents_source_id_idx ON legal_documents(source_id);
CREATE INDEX IF NOT EXISTS legal_documents_official_id_idx ON legal_documents(official_id);
CREATE INDEX IF NOT EXISTS legal_documents_procedure_key_idx ON legal_documents(procedure_key);

CREATE TABLE IF NOT EXISTS document_versions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id uuid NOT NULL REFERENCES legal_documents(id) ON DELETE CASCADE,
  version_label text NOT NULL,
  publication_date date,
  effective_date date,
  version_date date,
  retrieved_at timestamptz NOT NULL DEFAULT now(),
  source_url text,
  raw_content_path text,
  normalized_text text NOT NULL,
  content_hash text NOT NULL,
  parser_version text NOT NULL DEFAULT 'manual-v1',
  legal_status text NOT NULL DEFAULT 'effective',
  is_current boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT document_versions_legal_status_chk
    CHECK (legal_status IN ('published', 'not_yet_effective', 'effective', 'partially_effective', 'amended', 'repealed', 'superseded', 'pending_consolidation')),

  CONSTRAINT document_versions_unique_hash UNIQUE (document_id, content_hash)
);

CREATE INDEX IF NOT EXISTS document_versions_document_id_idx ON document_versions(document_id);
CREATE INDEX IF NOT EXISTS document_versions_current_idx ON document_versions(document_id, is_current);

CREATE TABLE IF NOT EXISTS document_sections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  version_id uuid NOT NULL REFERENCES document_versions(id) ON DELETE CASCADE,
  stable_section_key text NOT NULL,
  section_type text NOT NULL,
  section_number text,
  title text NOT NULL DEFAULT '',
  text_content text NOT NULL,
  text_hash text NOT NULL,
  order_index integer NOT NULL DEFAULT 0,
  parent_section_id uuid REFERENCES document_sections(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT document_sections_section_type_chk
    CHECK (section_type IN ('title', 'chapter', 'article', 'section', 'paragraph', 'additional_provision', 'transitional_provision', 'repealing_provision', 'final_provision', 'annex', 'other')),

  CONSTRAINT document_sections_unique_key UNIQUE (version_id, stable_section_key)
);

CREATE INDEX IF NOT EXISTS document_sections_version_id_idx ON document_sections(version_id);
CREATE INDEX IF NOT EXISTS document_sections_text_search_idx
  ON document_sections USING gin(to_tsvector('simple', title || ' ' || text_content));

CREATE TABLE IF NOT EXISTS document_diffs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id uuid NOT NULL REFERENCES legal_documents(id) ON DELETE CASCADE,
  from_version_id uuid REFERENCES document_versions(id) ON DELETE SET NULL,
  to_version_id uuid NOT NULL REFERENCES document_versions(id) ON DELETE CASCADE,
  diff_type text NOT NULL DEFAULT 'version_diff',
  summary text NOT NULL DEFAULT '',
  diff_json jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT document_diffs_diff_type_chk
    CHECK (diff_type IN ('new_document', 'version_diff', 'metadata_only', 'parser_change'))
);

CREATE INDEX IF NOT EXISTS document_diffs_document_id_idx ON document_diffs(document_id);

CREATE TABLE IF NOT EXISTS legal_changes (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  document_diff_id uuid NOT NULL REFERENCES document_diffs(id) ON DELETE CASCADE,
  document_id uuid NOT NULL REFERENCES legal_documents(id) ON DELETE CASCADE,
  change_type text NOT NULL,
  priority text NOT NULL DEFAULT 'medium',
  affected_sections jsonb NOT NULL DEFAULT '[]',
  detected_summary text NOT NULL DEFAULT '',
  requires_analysis boolean NOT NULL DEFAULT true,
  status text NOT NULL DEFAULT 'pending_analysis',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT legal_changes_change_type_chk
    CHECK (change_type IN ('new_document', 'added_section', 'removed_section', 'modified_wording', 'renumbering', 'effective_date_change', 'repeal_notice', 'procedure_requirement_change', 'metadata_only')),

  CONSTRAINT legal_changes_priority_chk
    CHECK (priority IN ('low', 'medium', 'high', 'critical')),

  CONSTRAINT legal_changes_status_chk
    CHECK (status IN ('pending_analysis', 'pending_review', 'approved', 'rejected', 'superseded'))
);

CREATE INDEX IF NOT EXISTS legal_changes_status_idx ON legal_changes(status);
CREATE INDEX IF NOT EXISTS legal_changes_priority_idx ON legal_changes(priority);

CREATE TABLE IF NOT EXISTS review_tasks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  legal_change_id uuid REFERENCES legal_changes(id) ON DELETE SET NULL,
  document_id uuid REFERENCES legal_documents(id) ON DELETE SET NULL,
  task_type text NOT NULL DEFAULT 'legal_change_review',
  title text NOT NULL,
  status text NOT NULL DEFAULT 'pending',
  priority text NOT NULL DEFAULT 'medium',
  ai_summary text NOT NULL DEFAULT '',
  reviewer_note text NOT NULL DEFAULT '',
  reviewed_by text,
  reviewed_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT review_tasks_status_chk
    CHECK (status IN ('pending', 'needs_correction', 'approved', 'rejected', 'published', 'superseded')),

  CONSTRAINT review_tasks_priority_chk
    CHECK (priority IN ('low', 'medium', 'high', 'critical'))
);

CREATE INDEX IF NOT EXISTS review_tasks_status_idx ON review_tasks(status);

CREATE TABLE IF NOT EXISTS knowledge_items (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  procedure_key text NOT NULL,
  topic_key text NOT NULL,
  title_es text NOT NULL,
  canonical_answer_es text NOT NULL,
  summary_ru text NOT NULL DEFAULT '',
  summary_en text NOT NULL DEFAULT '',
  conditions_json jsonb NOT NULL DEFAULT '[]',
  required_evidence_json jsonb NOT NULL DEFAULT '[]',
  source_refs_json jsonb NOT NULL DEFAULT '[]',
  review_task_id uuid REFERENCES review_tasks(id) ON DELETE SET NULL,
  status text NOT NULL DEFAULT 'draft',
  effective_from date,
  effective_until date,
  approved_by text,
  approved_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT knowledge_items_status_chk
    CHECK (status IN ('draft', 'approved', 'published', 'superseded', 'archived'))
);

CREATE INDEX IF NOT EXISTS knowledge_items_procedure_idx ON knowledge_items(procedure_key);
CREATE INDEX IF NOT EXISTS knowledge_items_status_idx ON knowledge_items(status);

CREATE TABLE IF NOT EXISTS audit_events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type text NOT NULL,
  entity_type text NOT NULL,
  entity_id uuid,
  actor text NOT NULL DEFAULT 'system',
  details_json jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS audit_events_entity_idx ON audit_events(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS audit_events_type_idx ON audit_events(event_type);