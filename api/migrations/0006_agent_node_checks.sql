ALTER TABLE agent_tasks
ADD COLUMN node_check_id TEXT REFERENCES node_checks(id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX agent_tasks_node_check_id
ON agent_tasks (node_check_id)
WHERE node_check_id IS NOT NULL;
