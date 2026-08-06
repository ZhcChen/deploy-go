ALTER TABLE application_env_files ADD COLUMN last_declared_deployment_id TEXT REFERENCES deployments(id) ON DELETE RESTRICT;
ALTER TABLE application_env_files ADD COLUMN last_declared_commit_sha TEXT;
ALTER TABLE application_env_files ADD COLUMN last_manifest_digest TEXT;

CREATE TABLE application_env_registration_leases (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    purpose TEXT NOT NULL DEFAULT 'env_registration' CHECK (purpose = 'env_registration'),
    commit_sha TEXT NOT NULL,
    manifest_digest TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'revoked', 'failed')),
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (deployment_id)
);

CREATE INDEX application_env_registration_leases_lookup
ON application_env_registration_leases (id, agent_id, status, expires_at);

CREATE TABLE application_env_reveal_grants (
    id TEXT PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    action_scope TEXT NOT NULL CHECK (action_scope IN ('read_write', 'delete')),
    user_version INTEGER NOT NULL CHECK (user_version > 0),
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX application_env_reveal_grants_lookup
ON application_env_reveal_grants (token_hash, user_id, session_id, application_id, action_scope, expires_at);

CREATE TABLE application_env_reauth_attempts (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    failed_count INTEGER NOT NULL DEFAULT 0 CHECK (failed_count >= 0),
    window_started_at TEXT NOT NULL,
    blocked_until TEXT
);
