CREATE TABLE IF NOT EXISTS data_sources (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  title text NOT NULL,
  source_type text NOT NULL,
  original_path text UNIQUE,
  source_url text,
  raw_text text,
  trust_level text NOT NULL DEFAULT 'unknown',
  status text NOT NULL DEFAULT 'new',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS analysis_jobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id uuid NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'completed',
  output_json jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS extracted_facts (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id uuid NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
  fact_type text NOT NULL,
  title_ru text NOT NULL,
  text_ru text NOT NULL,
  original_text text NOT NULL,
  confidence real NOT NULL DEFAULT 0,
  source_location text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS content_candidates (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id uuid REFERENCES data_sources(id) ON DELETE SET NULL,
  candidate_type text NOT NULL,
  title_ru text NOT NULL,
  summary_ru text NOT NULL DEFAULT '',
  body_ru_markdown text NOT NULL DEFAULT '',
  category_slug text,
  risk_level text NOT NULL DEFAULT 'medium',
  status text NOT NULL DEFAULT 'draft',
  review_note text,
  article_id uuid REFERENCES articles(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS indexing_jobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id uuid NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'queued',
  message text NOT NULL DEFAULT '',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_profiles (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL,
  slug text NOT NULL UNIQUE,
  description text NOT NULL DEFAULT '',
  active_prompt_version_id uuid,
  active_policy_version_id uuid,
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_prompt_versions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  assistant_profile_id uuid NOT NULL REFERENCES assistant_profiles(id) ON DELETE CASCADE,
  title text NOT NULL,
  system_prompt text NOT NULL,
  answer_format text NOT NULL DEFAULT '',
  safety_rules text[] NOT NULL DEFAULT '{}',
  status text NOT NULL DEFAULT 'draft',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_policies (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  assistant_profile_id uuid NOT NULL REFERENCES assistant_profiles(id) ON DELETE CASCADE,
  title text NOT NULL,
  retrieval_top_k integer NOT NULL DEFAULT 5,
  require_sources boolean NOT NULL DEFAULT true,
  allow_llm_without_sources boolean NOT NULL DEFAULT false,
  allowed_collection text NOT NULL DEFAULT 'spain_helper_main',
  status text NOT NULL DEFAULT 'draft',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_change_candidates (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  assistant_profile_id uuid NOT NULL REFERENCES assistant_profiles(id) ON DELETE CASCADE,
  candidate_type text NOT NULL,
  title text NOT NULL,
  description text NOT NULL DEFAULT '',
  proposed_payload_json jsonb NOT NULL DEFAULT '{}',
  reason text NOT NULL DEFAULT '',
  risk_level text NOT NULL DEFAULT 'medium',
  status text NOT NULL DEFAULT 'draft',
  review_note text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_test_runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  assistant_profile_id uuid NOT NULL REFERENCES assistant_profiles(id) ON DELETE CASCADE,
  candidate_id uuid REFERENCES assistant_change_candidates(id) ON DELETE SET NULL,
  test_question text NOT NULL,
  response_preview text NOT NULL DEFAULT '',
  sources_json jsonb NOT NULL DEFAULT '[]',
  passed boolean NOT NULL DEFAULT false,
  notes text NOT NULL DEFAULT '',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS assistant_notes (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  note_type text NOT NULL,
  title text NOT NULL,
  body text NOT NULL DEFAULT '',
  status text NOT NULL DEFAULT 'open',
  candidate_id uuid REFERENCES assistant_change_candidates(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO assistant_profiles (name, slug, description, is_active)
VALUES ('Spain Immigration Helper', 'spain-immigration-helper', 'Default assistant profile for the current app.', true)
ON CONFLICT (slug) DO NOTHING;

CREATE INDEX IF NOT EXISTS data_sources_status_idx ON data_sources(status);
CREATE INDEX IF NOT EXISTS extracted_facts_source_id_idx ON extracted_facts(source_id);
CREATE INDEX IF NOT EXISTS content_candidates_status_idx ON content_candidates(status);
CREATE INDEX IF NOT EXISTS assistant_change_candidates_status_idx ON assistant_change_candidates(status);
