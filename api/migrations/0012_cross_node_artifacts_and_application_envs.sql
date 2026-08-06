CREATE TABLE deployment_artifacts (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL UNIQUE REFERENCES deployments(id) ON DELETE RESTRICT,
    manifest_json TEXT NOT NULL DEFAULT '{}',
    manifest_digest TEXT NOT NULL,
    total_size INTEGER NOT NULL DEFAULT 0 CHECK (total_size BETWEEN 0 AND 2147483648),
    file_count INTEGER NOT NULL DEFAULT 0 CHECK (file_count BETWEEN 0 AND 256),
    storage_key TEXT,
    status TEXT NOT NULL CHECK (status IN ('uploading', 'verified', 'failed', 'expired', 'deleting')),
    upload_offset INTEGER NOT NULL DEFAULT 0 CHECK (upload_offset >= 0),
    expires_at TEXT NOT NULL,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK ((status = 'verified' AND storage_key IS NOT NULL AND verified_at IS NOT NULL) OR status <> 'verified')
);

CREATE INDEX deployment_artifacts_expiry
ON deployment_artifacts (status, expires_at);

CREATE TABLE deployment_target_runs (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    target_id TEXT NOT NULL REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT REFERENCES agents(id) ON DELETE RESTRICT,
    source_run_id TEXT REFERENCES deployment_target_runs(id) ON DELETE RESTRICT,
    artifact_id TEXT REFERENCES deployment_artifacts(id) ON DELETE RESTRICT,
    target_snapshot_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK (status IN ('pending', 'downloading', 'running', 'succeeded', 'failed', 'canceled', 'expired', 'reused')),
    phase TEXT NOT NULL DEFAULT 'pending',
    env_gate_status TEXT NOT NULL DEFAULT 'pending' CHECK (env_gate_status IN ('pending', 'ready', 'failed', 'not_required')),
    result_summary TEXT,
    error_code TEXT,
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (deployment_id, target_id)
);

CREATE INDEX deployment_target_runs_dispatch
ON deployment_target_runs (status, created_at, id);

CREATE INDEX deployment_target_runs_target
ON deployment_target_runs (target_id, created_at DESC);

INSERT INTO deployment_target_runs (
    id, deployment_id, target_id, node_id, agent_id, target_snapshot_json,
    status, phase, env_gate_status, result_summary, started_at, finished_at,
    created_at, updated_at
)
SELECT
    'legacy_run_' || d.id,
    d.id,
    d.target_id,
    t.node_id,
    a.id,
    json_object('legacy', 1, 'target_id', d.target_id, 'node_id', t.node_id),
    CASE d.status
        WHEN 'succeeded' THEN 'succeeded'
        WHEN 'failed' THEN 'failed'
        WHEN 'canceled' THEN 'canceled'
        WHEN 'interrupted' THEN 'failed'
        ELSE 'pending'
    END,
    d.phase,
    'not_required',
    d.result_summary,
    d.started_at,
    d.finished_at,
    d.created_at,
    d.updated_at
FROM deployments d
JOIN deployment_targets t ON t.id = d.target_id
LEFT JOIN agents a ON a.node_id = t.node_id;

CREATE TABLE artifact_leases (
    id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL REFERENCES deployment_artifacts(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    target_run_id TEXT REFERENCES deployment_target_runs(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('artifact_upload', 'artifact_download')),
    manifest_digest TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'revoked', 'expired', 'failed')),
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (
        (purpose = 'artifact_upload' AND target_run_id IS NULL)
        OR (purpose = 'artifact_download' AND target_run_id IS NOT NULL)
    )
);

CREATE INDEX artifact_leases_lookup
ON artifact_leases (id, agent_id, status, expires_at);

CREATE TABLE application_env_files (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    file_name TEXT NOT NULL COLLATE NOCASE,
    module TEXT NOT NULL,
    format TEXT NOT NULL CHECK (format = 'dotenv-v1'),
    current_version INTEGER NOT NULL DEFAULT 1 CHECK (current_version > 0),
    current_digest TEXT NOT NULL,
    declared_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    deleted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (application_id, file_name)
);

CREATE INDEX application_env_files_application
ON application_env_files (application_id, deleted_at, file_name);

CREATE TABLE application_env_versions (
    id TEXT PRIMARY KEY,
    env_file_id TEXT NOT NULL REFERENCES application_env_files(id) ON DELETE RESTRICT,
    env_version INTEGER NOT NULL CHECK (env_version > 0),
    algorithm TEXT NOT NULL CHECK (algorithm = 'chacha20poly1305-application-env-v1'),
    ciphertext BLOB NOT NULL,
    nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    digest TEXT NOT NULL,
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (env_file_id, env_version)
);

CREATE TABLE application_env_syncs (
    id TEXT PRIMARY KEY,
    env_version_id TEXT NOT NULL REFERENCES application_env_versions(id) ON DELETE RESTRICT,
    target_id TEXT NOT NULL REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT REFERENCES agents(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'syncing', 'succeeded', 'failed')),
    actual_version INTEGER CHECK (actual_version IS NULL OR actual_version > 0),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    error_code TEXT,
    error_message TEXT,
    last_attempt_at TEXT,
    synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (env_version_id, target_id)
);

CREATE INDEX application_env_syncs_dispatch
ON application_env_syncs (agent_id, status, created_at, id);

CREATE TABLE agent_tasks_new (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    deployment_id TEXT REFERENCES deployments(id) ON DELETE RESTRICT,
    target_run_id TEXT REFERENCES deployment_target_runs(id) ON DELETE RESTRICT,
    env_sync_id TEXT REFERENCES application_env_syncs(id) ON DELETE RESTRICT,
    stage TEXT CHECK (stage IS NULL OR stage IN ('prepare', 'release')),
    node_check_id TEXT REFERENCES node_checks(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('system_inspect', 'deployment_execute', 'git_refs_query', 'deployment_prepare', 'deployment_release', 'env_sync', 'health_diagnose')),
    idempotency_key TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'delivered', 'accepted', 'running', 'canceling', 'succeeded', 'failed', 'canceled', 'interrupted')),
    deadline_at TEXT NOT NULL,
    lease_expires_at TEXT,
    delivered_at TEXT,
    acknowledged_at TEXT,
    started_at TEXT,
    finished_at TEXT,
    last_sequence INTEGER NOT NULL DEFAULT 0 CHECK (last_sequence >= 0),
    result_json TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (agent_id, idempotency_key),
    CHECK (
        (kind = 'deployment_prepare' AND deployment_id IS NOT NULL AND target_run_id IS NULL AND env_sync_id IS NULL AND stage = 'prepare')
        OR (kind = 'deployment_release' AND deployment_id IS NOT NULL AND target_run_id IS NOT NULL AND env_sync_id IS NULL AND stage = 'release')
        OR (kind = 'env_sync' AND deployment_id IS NULL AND target_run_id IS NULL AND env_sync_id IS NOT NULL AND stage IS NULL)
        OR (kind NOT IN ('deployment_prepare', 'deployment_release', 'env_sync') AND target_run_id IS NULL AND env_sync_id IS NULL AND stage IS NULL)
    )
);

INSERT INTO agent_tasks_new (
    id, agent_id, deployment_id, target_run_id, env_sync_id, stage,
    node_check_id, kind, idempotency_key, payload_digest, payload_json,
    status, deadline_at, lease_expires_at, delivered_at, acknowledged_at,
    started_at, finished_at, last_sequence, result_json, created_at, updated_at
)
SELECT
    id, agent_id, deployment_id,
    CASE WHEN kind = 'deployment_release' THEN 'legacy_run_' || deployment_id ELSE NULL END,
    NULL, stage, node_check_id, kind, idempotency_key, payload_digest,
    payload_json, status, deadline_at, lease_expires_at, delivered_at,
    acknowledged_at, started_at, finished_at, last_sequence, result_json,
    created_at, updated_at
FROM agent_tasks;

DROP TABLE agent_tasks;
ALTER TABLE agent_tasks_new RENAME TO agent_tasks;

CREATE UNIQUE INDEX agent_tasks_one_prepare_per_deployment
ON agent_tasks (deployment_id)
WHERE kind = 'deployment_prepare';

CREATE UNIQUE INDEX agent_tasks_one_release_per_target_run
ON agent_tasks (target_run_id)
WHERE kind = 'deployment_release';

CREATE UNIQUE INDEX agent_tasks_one_task_per_env_sync
ON agent_tasks (env_sync_id)
WHERE kind = 'env_sync';

CREATE INDEX agent_tasks_dispatch ON agent_tasks (agent_id, status, created_at, id);

CREATE UNIQUE INDEX agent_tasks_node_check_id
ON agent_tasks (node_check_id)
WHERE node_check_id IS NOT NULL;

CREATE TABLE migration_0012_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0012_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0012_foreign_key_guard;
