CREATE TABLE git_credentials (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    algorithm TEXT NOT NULL CHECK (algorithm = 'ed25519'),
    public_key TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    encrypted_private_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX git_credentials_status ON git_credentials (status);

CREATE TABLE application_sources (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL UNIQUE REFERENCES applications(id) ON DELETE RESTRICT,
    repository_url TEXT NOT NULL,
    git_credential_id TEXT REFERENCES git_credentials(id) ON DELETE RESTRICT,
    build_agent_id TEXT REFERENCES agents(id) ON DELETE RESTRICT,
    source_policy TEXT NOT NULL DEFAULT 'branch' CHECK (source_policy = 'branch'),
    deployment_branch TEXT,
    source_version INTEGER NOT NULL DEFAULT 1 CHECK (source_version > 0),
    branch_verified_at TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'verified', 'archived')),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX application_sources_status ON application_sources (status);

ALTER TABLE deployment_targets
ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'script'
CHECK (execution_mode IN ('script', 'two_stage'));

CREATE TABLE agent_tasks_new (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    deployment_id TEXT REFERENCES deployments(id) ON DELETE RESTRICT,
    stage TEXT CHECK (stage IS NULL OR stage IN ('prepare', 'release')),
    node_check_id TEXT REFERENCES node_checks(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('system_inspect', 'deployment_execute', 'git_refs_query', 'deployment_prepare', 'deployment_release', 'health_diagnose')),
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
        (kind IN ('deployment_prepare', 'deployment_release') AND stage IS NOT NULL)
        OR (kind NOT IN ('deployment_prepare', 'deployment_release') AND stage IS NULL)
    )
);

INSERT INTO agent_tasks_new (
    id, agent_id, deployment_id, stage, node_check_id, kind, idempotency_key,
    payload_digest, payload_json, status, deadline_at, lease_expires_at,
    delivered_at, acknowledged_at, started_at, finished_at, last_sequence,
    result_json, created_at, updated_at
)
SELECT
    id, agent_id, deployment_id, NULL, node_check_id, kind, idempotency_key,
    payload_digest, payload_json, status, deadline_at, lease_expires_at,
    delivered_at, acknowledged_at, started_at, finished_at, last_sequence,
    result_json, created_at, updated_at
FROM agent_tasks;

DROP TABLE agent_tasks;
ALTER TABLE agent_tasks_new RENAME TO agent_tasks;

CREATE UNIQUE INDEX agent_tasks_one_stage_per_deployment
ON agent_tasks (deployment_id, stage)
WHERE deployment_id IS NOT NULL AND stage IS NOT NULL;

CREATE INDEX agent_tasks_dispatch ON agent_tasks (agent_id, status, created_at, id);

CREATE UNIQUE INDEX agent_tasks_node_check_id
ON agent_tasks (node_check_id)
WHERE node_check_id IS NOT NULL;

CREATE TABLE git_ref_discoveries (
    id TEXT PRIMARY KEY,
    application_source_id TEXT NOT NULL REFERENCES application_sources(id) ON DELETE RESTRICT,
    source_version INTEGER NOT NULL CHECK (source_version > 0),
    task_id TEXT NOT NULL UNIQUE REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'expired')),
    refs_json TEXT NOT NULL DEFAULT '[]',
    error_code TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    finished_at TEXT
);

CREATE INDEX git_ref_discoveries_source
ON git_ref_discoveries (application_source_id, source_version, created_at DESC);

CREATE TABLE migration_0008_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0008_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0008_foreign_key_guard;
