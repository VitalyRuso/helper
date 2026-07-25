CREATE TABLE IF NOT EXISTS local_llm_workers (
  worker_id text PRIMARY KEY,
  reported_status text NOT NULL DEFAULT 'offline',
  model_id text,
  model_name text,
  reported_at timestamptz NOT NULL,
  heartbeat_at timestamptz NOT NULL DEFAULT now(),
  lm_studio_ready boolean NOT NULL DEFAULT false,
  busy boolean NOT NULL DEFAULT false,
  last_error text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT local_llm_workers_status_chk
    CHECK (reported_status IN ('online', 'offline'))
);

CREATE TABLE IF NOT EXISTS local_llm_jobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id text NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'queued',
  request_payload jsonb NOT NULL,
  response_payload jsonb,
  model_id text,
  lease_owner text,
  lease_token uuid,
  lease_expires_at timestamptz,
  expires_at timestamptz NOT NULL DEFAULT (now() + interval '10 minutes'),
  attempt_count integer NOT NULL DEFAULT 0,
  safe_error text,
  started_at timestamptz,
  completed_at timestamptz,
  worker_started_at timestamptz,
  worker_completed_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT local_llm_jobs_status_chk
    CHECK (status IN ('queued', 'running', 'completed', 'failed', 'expired')),
  CONSTRAINT local_llm_jobs_attempt_count_chk
    CHECK (attempt_count >= 0)
);

CREATE INDEX IF NOT EXISTS local_llm_workers_heartbeat_idx
  ON local_llm_workers(heartbeat_at DESC);
CREATE INDEX IF NOT EXISTS local_llm_jobs_session_idx
  ON local_llm_jobs(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS local_llm_jobs_status_idx
  ON local_llm_jobs(status, created_at);
CREATE INDEX IF NOT EXISTS local_llm_jobs_lease_idx
  ON local_llm_jobs(lease_expires_at)
  WHERE status = 'running';

-- One home worker is the MVP ceiling. Remove this index when a worker pool is introduced.
CREATE UNIQUE INDEX IF NOT EXISTS local_llm_jobs_single_active_idx
  ON local_llm_jobs ((true))
  WHERE status IN ('queued', 'running');
