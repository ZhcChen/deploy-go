CREATE TABLE agent_upgrade_jobs (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    target_version TEXT NOT NULL CHECK (length(target_version) BETWEEN 1 AND 64),
    manifest_digest TEXT NOT NULL CHECK (manifest_digest GLOB 'sha256:*' AND length(manifest_digest) = 71),
    target_architecture TEXT NOT NULL CHECK (target_architecture IN ('x86_64')),
    status TEXT NOT NULL CHECK (status IN ('queued', 'blocked_bootstrap_required', 'waiting_for_online', 'waiting_for_idle', 'downloading', 'installing', 'reconnecting', 'succeeded', 'failed', 'blocked_unsupported_architecture')),
    phase TEXT CHECK (phase IS NULL OR phase IN ('validating', 'downloading', 'staged', 'installing', 'executor_restart', 'reconnecting')),
    current_version TEXT,
    connection_generation INTEGER CHECK (connection_generation IS NULL OR connection_generation > 0),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    lease_token TEXT,
    lease_expires_at TEXT,
    error_code TEXT,
    error_summary TEXT CHECK (error_summary IS NULL OR length(error_summary) <= 512),
    queued_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at TEXT,
    finished_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE UNIQUE INDEX agent_upgrade_jobs_active_target
ON agent_upgrade_jobs (agent_id, target_version)
WHERE status NOT IN ('succeeded', 'failed', 'blocked_unsupported_architecture');

CREATE INDEX agent_upgrade_jobs_dispatch
ON agent_upgrade_jobs (status, queued_at, id);

CREATE INDEX agent_upgrade_jobs_node
ON agent_upgrade_jobs (node_id, status, updated_at);

CREATE TABLE agent_upgrade_leases (
    lease_key TEXT PRIMARY KEY CHECK (lease_key = 'global'),
    job_id TEXT NOT NULL REFERENCES agent_upgrade_jobs(id) ON DELETE RESTRICT,
    lease_token TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE agent_maintenance_locks (
    node_id TEXT PRIMARY KEY REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    job_id TEXT NOT NULL REFERENCES agent_upgrade_jobs(id) ON DELETE RESTRICT,
    lease_token TEXT NOT NULL,
    lock_epoch INTEGER NOT NULL CHECK (lock_epoch > 0),
    reason TEXT NOT NULL CHECK (reason = 'agent_upgrade'),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX agent_maintenance_locks_job
ON agent_maintenance_locks (job_id);

CREATE TABLE agent_upgrade_events (
    job_id TEXT NOT NULL REFERENCES agent_upgrade_jobs(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    kind TEXT NOT NULL CHECK (kind IN ('state', 'progress', 'report', 'recovery')),
    phase TEXT,
    summary TEXT NOT NULL CHECK (length(summary) <= 512),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (job_id, sequence)
);

CREATE INDEX agent_upgrade_events_created_at
ON agent_upgrade_events (created_at);
