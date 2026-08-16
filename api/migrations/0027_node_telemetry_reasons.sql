ALTER TABLE node_telemetry_current
ADD COLUMN gpu_reason TEXT
CHECK (gpu_reason IS NULL OR gpu_reason IN (
    'hardware_not_present', 'unsupported_platform', 'backend_unavailable',
    'permission_denied', 'timeout', 'parse_error', 'source_unavailable'
));
