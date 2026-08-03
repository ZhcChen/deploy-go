ALTER TABLE agents ADD COLUMN connection_generation INTEGER NOT NULL DEFAULT 0 CHECK (connection_generation >= 0);

ALTER TABLE agent_refresh_credentials ADD COLUMN token_key_version INTEGER NOT NULL DEFAULT 1 CHECK (token_key_version > 0);

ALTER TABLE agent_access_sessions ADD COLUMN refresh_credential_id TEXT REFERENCES agent_refresh_credentials(id) ON DELETE RESTRICT;
ALTER TABLE agent_access_sessions ADD COLUMN token_key_version INTEGER NOT NULL DEFAULT 1 CHECK (token_key_version > 0);

CREATE UNIQUE INDEX agent_access_refresh_credential
ON agent_access_sessions (refresh_credential_id)
WHERE refresh_credential_id IS NOT NULL;
