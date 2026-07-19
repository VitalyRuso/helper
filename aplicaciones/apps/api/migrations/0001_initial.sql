CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS categories (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  title_ru text NOT NULL,
  slug text NOT NULL UNIQUE,
  description_ru text NOT NULL DEFAULT '',
  icon text NOT NULL DEFAULT 'file-text',
  sort_order integer NOT NULL DEFAULT 0,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS articles (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  category_id uuid REFERENCES categories(id) ON DELETE SET NULL,
  title_ru text NOT NULL,
  slug text NOT NULL UNIQUE,
  summary_ru text NOT NULL DEFAULT '',
  body_ru_markdown text NOT NULL DEFAULT '',
  tags text[] NOT NULL DEFAULT '{}',
  source_references jsonb NOT NULL DEFAULT '[]',
  legal_risk_level text NOT NULL DEFAULT 'medium',
  is_published boolean NOT NULL DEFAULT true,
  include_in_ai boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS guides (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  title_ru text NOT NULL,
  slug text NOT NULL UNIQUE,
  summary_ru text NOT NULL DEFAULT '',
  target_audience text NOT NULL DEFAULT '',
  required_documents jsonb NOT NULL DEFAULT '[]',
  steps jsonb NOT NULL DEFAULT '[]',
  deadlines jsonb NOT NULL DEFAULT '[]',
  fees jsonb NOT NULL DEFAULT '[]',
  where_to_submit text NOT NULL DEFAULT '',
  common_mistakes jsonb NOT NULL DEFAULT '[]',
  risks jsonb NOT NULL DEFAULT '[]',
  official_sources jsonb NOT NULL DEFAULT '[]',
  related_article_ids uuid[] NOT NULL DEFAULT '{}',
  is_published boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sessions (
  session_id text PRIMARY KEY,
  question_count integer NOT NULL DEFAULT 0,
  has_access boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS access_keys (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  key_hash text NOT NULL UNIQUE,
  label text NOT NULL DEFAULT 'env',
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS chat_messages (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id text NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
  role text NOT NULL,
  content text NOT NULL,
  sources jsonb NOT NULL DEFAULT '[]',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS source_documents (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_file text NOT NULL,
  file_name text NOT NULL,
  document_type text NOT NULL,
  page_number integer,
  chunk_count integer NOT NULL DEFAULT 0,
  language text NOT NULL DEFAULT 'ru',
  source_url text,
  imported_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS bot_profiles (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL,
  slug text NOT NULL UNIQUE,
  system_prompt text NOT NULL,
  is_default boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS articles_category_id_idx ON articles(category_id);
CREATE INDEX IF NOT EXISTS articles_search_idx ON articles USING gin(to_tsvector('simple', title_ru || ' ' || summary_ru || ' ' || body_ru_markdown));
CREATE INDEX IF NOT EXISTS guides_search_idx ON guides USING gin(to_tsvector('simple', title_ru || ' ' || summary_ru));
CREATE INDEX IF NOT EXISTS chat_messages_session_idx ON chat_messages(session_id);
