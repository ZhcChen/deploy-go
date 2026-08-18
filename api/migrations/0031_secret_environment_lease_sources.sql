-- Secret environment lease 的来源引用和 descriptor 摘要只保存元数据，不保存明文。
ALTER TABLE secret_environment_leases
ADD COLUMN credential_id TEXT REFERENCES configuration_center_credentials(id) ON DELETE RESTRICT;

ALTER TABLE secret_environment_leases
ADD COLUMN descriptor_digest TEXT;

ALTER TABLE secret_environment_leases
ADD COLUMN public_values_json TEXT NOT NULL DEFAULT '{}'
CHECK (json_valid(public_values_json) AND json_type(public_values_json) = 'object');

ALTER TABLE secret_environment_leases
ADD COLUMN credential_variable_name TEXT NOT NULL DEFAULT '';

ALTER TABLE secret_environment_leases
ADD COLUMN value_digest TEXT;

CREATE INDEX secret_environment_leases_credential
ON secret_environment_leases (credential_id, status, expires_at);
