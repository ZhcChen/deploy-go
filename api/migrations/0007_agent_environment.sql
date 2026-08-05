ALTER TABLE agents
ADD COLUMN environment TEXT NOT NULL DEFAULT 'dev'
CHECK (environment IN ('dev', 'test', 'staging', 'prod'));
