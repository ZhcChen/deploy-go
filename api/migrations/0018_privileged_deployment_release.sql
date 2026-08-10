ALTER TABLE deployment_targets
ADD COLUMN privileged_release INTEGER NOT NULL DEFAULT 0
CHECK (privileged_release IN (0, 1));
