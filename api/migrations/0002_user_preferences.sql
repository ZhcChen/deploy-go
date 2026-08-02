ALTER TABLE users ADD COLUMN display_name TEXT;
ALTER TABLE users ADD COLUMN email TEXT COLLATE NOCASE;

CREATE UNIQUE INDEX users_email_unique
ON users (email)
WHERE email IS NOT NULL;

CREATE TABLE session_csrf_tokens (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (session_id, token_hash)
);

CREATE INDEX session_csrf_tokens_session_created
ON session_csrf_tokens (session_id, created_at, id);

CREATE TABLE user_preferences (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    notify_deployment_failed INTEGER NOT NULL DEFAULT 1 CHECK (notify_deployment_failed IN (0, 1)),
    notify_deployment_completed INTEGER NOT NULL DEFAULT 1 CHECK (notify_deployment_completed IN (0, 1)),
    notify_node_unhealthy INTEGER NOT NULL DEFAULT 1 CHECK (notify_node_unhealthy IN (0, 1)),
    time_format TEXT NOT NULL DEFAULT '24h' CHECK (time_format IN ('12h', '24h')),
    follow_logs INTEGER NOT NULL DEFAULT 1 CHECK (follow_logs IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);
