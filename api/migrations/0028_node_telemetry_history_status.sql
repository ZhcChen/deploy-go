ALTER TABLE node_telemetry_history
ADD COLUMN cpu_status TEXT NOT NULL DEFAULT 'warming_up'
CHECK (cpu_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN memory_status TEXT NOT NULL DEFAULT 'warming_up'
CHECK (memory_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN work_root_status TEXT NOT NULL DEFAULT 'warming_up'
CHECK (work_root_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN disk_io_status TEXT NOT NULL DEFAULT 'warming_up'
CHECK (disk_io_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN network_status TEXT NOT NULL DEFAULT 'warming_up'
CHECK (network_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN gpu_status TEXT NOT NULL DEFAULT 'unsupported'
CHECK (gpu_status IN ('available', 'warming_up', 'unsupported', 'collection_error'));

ALTER TABLE node_telemetry_history
ADD COLUMN gpu_reason TEXT
CHECK (gpu_reason IS NULL OR gpu_reason IN (
    'hardware_not_present', 'unsupported_platform', 'backend_unavailable',
    'permission_denied', 'timeout', 'parse_error', 'source_unavailable'
));

UPDATE node_telemetry_history
SET cpu_status = 'available'
WHERE cpu_usage_percent IS NOT NULL;

UPDATE node_telemetry_history
SET memory_status = 'available'
WHERE memory_total_bytes IS NOT NULL AND memory_used_bytes IS NOT NULL;

UPDATE node_telemetry_history
SET work_root_status = 'available'
WHERE work_root_total_bytes IS NOT NULL AND work_root_used_bytes IS NOT NULL;

UPDATE node_telemetry_history
SET disk_io_status = 'available'
WHERE disk_read_bytes_per_second IS NOT NULL
  AND disk_write_bytes_per_second IS NOT NULL
  AND disk_busy_percent IS NOT NULL;

UPDATE node_telemetry_history
SET network_status = 'available'
WHERE network_receive_bytes_per_second IS NOT NULL
  AND network_transmit_bytes_per_second IS NOT NULL;

UPDATE node_telemetry_history
SET gpu_status = 'available'
WHERE json_valid(gpus_json) AND json_array_length(gpus_json) > 0;
