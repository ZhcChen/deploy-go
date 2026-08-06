ALTER TABLE deployment_artifacts
ADD COLUMN upload_size INTEGER CHECK (upload_size IS NULL OR upload_size > 0);

ALTER TABLE deployment_artifacts
ADD COLUMN archive_digest TEXT CHECK (
    archive_digest IS NULL
    OR (length(archive_digest) = 64 AND archive_digest NOT GLOB '*[^0-9a-f]*')
);

CREATE TRIGGER deployment_artifacts_verified_upload_facts
BEFORE UPDATE OF status, upload_size, archive_digest, upload_offset, storage_key ON deployment_artifacts
WHEN NEW.status = 'verified'
 AND (
    NEW.upload_size IS NULL
    OR NEW.archive_digest IS NULL
    OR NEW.upload_offset <> NEW.upload_size
    OR NEW.storage_key <> NEW.archive_digest
 )
BEGIN
    SELECT RAISE(ABORT, 'verified artifact requires complete upload facts');
END;

CREATE TRIGGER deployment_artifacts_verified_insert_facts
BEFORE INSERT ON deployment_artifacts
WHEN NEW.status = 'verified'
 AND (
    NEW.upload_size IS NULL
    OR NEW.archive_digest IS NULL
    OR NEW.upload_offset <> NEW.upload_size
    OR NEW.storage_key <> NEW.archive_digest
 )
BEGIN
    SELECT RAISE(ABORT, 'verified artifact requires complete upload facts');
END;

CREATE TRIGGER deployment_artifacts_verified_facts_immutable
BEFORE UPDATE OF manifest_json, manifest_digest, total_size, file_count, storage_key, upload_offset, upload_size, archive_digest
ON deployment_artifacts
WHEN OLD.status = 'verified'
 AND (
    NEW.manifest_json <> OLD.manifest_json
    OR NEW.manifest_digest <> OLD.manifest_digest
    OR NEW.total_size <> OLD.total_size
    OR NEW.file_count <> OLD.file_count
    OR NOT (NEW.storage_key IS OLD.storage_key)
    OR NEW.upload_offset <> OLD.upload_offset
    OR NOT (NEW.upload_size IS OLD.upload_size)
    OR NOT (NEW.archive_digest IS OLD.archive_digest)
 )
BEGIN
    SELECT RAISE(ABORT, 'verified artifact facts are immutable');
END;
