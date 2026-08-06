ALTER TABLE deployments
ADD COLUMN application_id TEXT REFERENCES applications(id) ON DELETE RESTRICT;

UPDATE deployments
SET application_id = (
    SELECT target.application_id
    FROM deployment_targets target
    WHERE target.id = deployments.target_id
)
WHERE application_id IS NULL;

CREATE TABLE migration_0013_application_guard (
    invalid_count INTEGER NOT NULL CHECK (invalid_count = 0)
);

INSERT INTO migration_0013_application_guard (invalid_count)
SELECT COUNT(*) FROM deployments WHERE application_id IS NULL;

DROP TABLE migration_0013_application_guard;

CREATE INDEX deployments_application_created
ON deployments (application_id, created_at DESC);

CREATE TRIGGER deployments_application_matches_target_insert
BEFORE INSERT ON deployments
WHEN NEW.application_id IS NOT NULL
 AND NEW.application_id <> (SELECT application_id FROM deployment_targets WHERE id = NEW.target_id)
BEGIN
    SELECT RAISE(ABORT, 'deployments.application_id must match target');
END;

CREATE TRIGGER deployments_application_immutable_update
BEFORE UPDATE OF application_id, target_id ON deployments
WHEN NOT (NEW.application_id IS OLD.application_id) OR NEW.target_id <> OLD.target_id
BEGIN
    SELECT RAISE(ABORT, 'deployment ownership is immutable');
END;
