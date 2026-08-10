ALTER TABLE users
ADD COLUMN system_account INTEGER NOT NULL DEFAULT 0
CHECK (system_account IN (0, 1));

-- 对外部署 API 使用共享系统账号记录 requested_by；该账号不能登录且不展示在用户列表。
INSERT INTO users (
    id,
    username,
    password_hash,
    identity,
    status,
    display_name,
    system_account
)
VALUES (
    'usr_external_api_service',
    '__deploy_go_external_api__',
    '!system-account-login-disabled!',
    'user',
    'active',
    'Deploy Go External API',
    1
)
ON CONFLICT(username) DO NOTHING;

CREATE TABLE external_api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    token_hash BLOB NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    expires_at TEXT,
    last_used_at TEXT,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX external_api_keys_status
ON external_api_keys (status);

CREATE TABLE external_api_key_applications (
    api_key_id TEXT NOT NULL REFERENCES external_api_keys(id) ON DELETE CASCADE,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    granted_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    granted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (api_key_id, application_id)
);

CREATE INDEX external_api_key_applications_application
ON external_api_key_applications (application_id);

ALTER TABLE deployments
ADD COLUMN external_api_key_id TEXT
REFERENCES external_api_keys(id) ON DELETE SET NULL;

CREATE INDEX deployments_external_api_key
ON deployments (external_api_key_id);
