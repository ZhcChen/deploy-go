-- 统计受管输出正文的 UTF-8 字节，不包含 SQLite 页面和 WAL。
CREATE TABLE deployment_log_capacity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    total_bytes INTEGER NOT NULL CHECK (total_bytes >= 0)
);
INSERT INTO deployment_log_capacity(id, total_bytes)
SELECT 1,
    (SELECT COALESCE(SUM(length(CAST(content AS BLOB))), 0) FROM deployment_logs)
    + (SELECT COALESCE(SUM(length(CAST(payload_json AS BLOB))), 0) FROM agent_task_events WHERE kind = 'output');

CREATE TRIGGER deployment_log_capacity_insert AFTER INSERT ON deployment_logs BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes + length(CAST(NEW.content AS BLOB)) WHERE id = 1;
END;
CREATE TRIGGER deployment_log_capacity_update AFTER UPDATE OF content ON deployment_logs BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes + length(CAST(NEW.content AS BLOB)) - length(CAST(OLD.content AS BLOB)) WHERE id = 1;
END;
CREATE TRIGGER deployment_log_capacity_delete AFTER DELETE ON deployment_logs BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes - length(CAST(OLD.content AS BLOB)) WHERE id = 1;
END;

CREATE TRIGGER agent_output_capacity_insert AFTER INSERT ON agent_task_events WHEN NEW.kind = 'output' BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes + length(CAST(NEW.payload_json AS BLOB)) WHERE id = 1;
END;
CREATE TRIGGER agent_output_capacity_update AFTER UPDATE OF kind, payload_json ON agent_task_events BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes
        + CASE WHEN NEW.kind = 'output' THEN length(CAST(NEW.payload_json AS BLOB)) ELSE 0 END
        - CASE WHEN OLD.kind = 'output' THEN length(CAST(OLD.payload_json AS BLOB)) ELSE 0 END WHERE id = 1;
END;
CREATE TRIGGER agent_output_capacity_delete AFTER DELETE ON agent_task_events WHEN OLD.kind = 'output' BEGIN
    UPDATE deployment_log_capacity SET total_bytes = total_bytes - length(CAST(OLD.payload_json AS BLOB)) WHERE id = 1;
END;

CREATE INDEX deployment_log_capacity_terminal_order ON deployments(finished_at, id)
WHERE status IN ('succeeded', 'failed', 'canceled', 'interrupted');
