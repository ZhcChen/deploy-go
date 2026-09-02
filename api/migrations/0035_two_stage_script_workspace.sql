-- 新增 two_stage_script 工作区脚本变体与独立的工作区来源。
-- 既有 Git two_stage、image、script 模式保持不变。
-- 迁移门禁禁止重建表或删除列，因此不修改 execution_mode CHECK：
-- two_stage_script 在存储层使用 execution_mode='two_stage' + workspace_script=1，
-- API 响应与部署快照继续对外暴露 two_stage_script 语义。
ALTER TABLE deployment_targets
ADD COLUMN workspace_script INTEGER NOT NULL DEFAULT 0
CHECK (workspace_script IN (0, 1));

CREATE TABLE application_workspace_sources (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL UNIQUE REFERENCES applications(id) ON DELETE RESTRICT,
    build_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    workspace_path TEXT NOT NULL,
    workspace_version INTEGER NOT NULL DEFAULT 1 CHECK (workspace_version > 0),
    status TEXT NOT NULL DEFAULT 'verified'
        CHECK (status IN ('draft', 'verified', 'archived')),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX application_workspace_sources_status
ON application_workspace_sources (status);
