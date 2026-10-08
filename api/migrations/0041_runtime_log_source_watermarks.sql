-- 来源去重水位不随 JSONL 轮转回收；每个 Agent 最多四个组件。
CREATE TABLE runtime_log_source_watermarks (
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    component TEXT NOT NULL CHECK (component IN ('agent','runner','executor','updater')),
    epoch TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    source_digest TEXT NOT NULL,
    central_sequence INTEGER NOT NULL CHECK (central_sequence >= 0),
    PRIMARY KEY(agent_id, component)
);
