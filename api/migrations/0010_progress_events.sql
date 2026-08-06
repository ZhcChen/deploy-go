CREATE TABLE agent_task_events_new (
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    kind TEXT NOT NULL CHECK (kind IN ('output', 'state', 'result', 'diagnostic', 'progress')),
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (task_id, sequence)
);

INSERT INTO agent_task_events_new (
    task_id, sequence, kind, payload_json, created_at
)
SELECT
    task_id, sequence, kind, payload_json, created_at
FROM agent_task_events;

DROP TABLE agent_task_events;
ALTER TABLE agent_task_events_new RENAME TO agent_task_events;

CREATE TABLE migration_0010_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0010_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0010_foreign_key_guard;
