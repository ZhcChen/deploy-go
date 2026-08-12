-- 服务端部署预览：确认时固化预览时解析到的 commit，
-- 分支在预览后移动不改变本次部署，重复确认同一快照不能创建多个部署。
CREATE TABLE deployment_previews (
    preview_id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    target_id TEXT REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    snapshot_hash TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    parameters_json TEXT NOT NULL,
    release_strategy TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'confirmed')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at TEXT NOT NULL,
    confirmed_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX deployment_previews_application_hash
ON deployment_previews (application_id, snapshot_hash);

CREATE INDEX deployment_previews_target_hash
ON deployment_previews (target_id, snapshot_hash)
WHERE target_id IS NOT NULL;

CREATE INDEX deployment_previews_expiry
ON deployment_previews (expires_at)
WHERE status = 'active';
