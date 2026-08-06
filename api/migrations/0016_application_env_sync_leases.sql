ALTER TABLE application_env_syncs
ADD COLUMN action TEXT NOT NULL DEFAULT 'write'
CHECK (action IN ('write', 'delete'));

CREATE TABLE application_env_secret_leases (
    id TEXT PRIMARY KEY,
    env_sync_id TEXT NOT NULL REFERENCES application_env_syncs(id) ON DELETE RESTRICT,
    env_version_id TEXT NOT NULL REFERENCES application_env_versions(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    purpose TEXT NOT NULL CHECK (purpose = 'application_env'),
    status TEXT NOT NULL CHECK (status IN ('issued', 'consumed', 'expired', 'revoked', 'failed')),
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX application_env_secret_leases_lookup
ON application_env_secret_leases (id, agent_id, purpose, status, expires_at);

CREATE UNIQUE INDEX application_env_secret_leases_one_active
ON application_env_secret_leases (env_sync_id)
WHERE status = 'issued';

DROP INDEX agent_tasks_one_task_per_env_sync;

CREATE UNIQUE INDEX agent_tasks_one_active_task_per_env_sync
ON agent_tasks (env_sync_id)
WHERE kind = 'env_sync'
  AND status IN ('queued', 'delivered', 'accepted', 'running', 'canceling');
