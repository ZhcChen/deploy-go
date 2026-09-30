CREATE TABLE agent_task_event_receipts (
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    source_digest TEXT NOT NULL,
    committed INTEGER NOT NULL DEFAULT 0 CHECK (committed IN (0,1)),
    PRIMARY KEY (task_id, sequence)
);
