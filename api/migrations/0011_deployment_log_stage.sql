-- 两阶段部署中 prepare/release 各自使用独立 task 序列，
-- 原 deployment_logs 以 (deployment_id, sequence) 为主键会把后续阶段日志丢弃。
-- 重建为全局 deployment 日志序号，并保留 task 序号用于按阶段分组展示。
CREATE TABLE deployment_logs_new (
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    task_id TEXT REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    task_sequence INTEGER NOT NULL CHECK (task_sequence > 0),
    stream TEXT NOT NULL CHECK (stream IN ('stdout', 'stderr', 'system')),
    content TEXT NOT NULL,
    truncated INTEGER NOT NULL DEFAULT 0 CHECK (truncated IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (deployment_id, sequence)
);

CREATE INDEX deployment_logs_task
ON deployment_logs_new (deployment_id, task_id, task_sequence);

INSERT INTO deployment_logs_new (
    deployment_id, task_id, sequence, task_sequence,
    stream, content, truncated, created_at
)
SELECT
    deployment_id, NULL, sequence, sequence,
    stream, content, truncated, created_at
FROM deployment_logs;

DROP TABLE deployment_logs;
ALTER TABLE deployment_logs_new RENAME TO deployment_logs;

CREATE TABLE migration_0011_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0011_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0011_foreign_key_guard;
