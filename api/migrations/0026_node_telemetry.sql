CREATE TABLE node_telemetry_current (
    node_id TEXT PRIMARY KEY REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    connection_generation INTEGER NOT NULL CHECK (connection_generation > 0),
    sample_sequence INTEGER NOT NULL CHECK (sample_sequence > 0),
    captured_at TEXT NOT NULL,
    received_at TEXT NOT NULL,
    cpu_status TEXT NOT NULL CHECK (cpu_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    cpu_usage_percent REAL,
    memory_status TEXT NOT NULL CHECK (memory_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    memory_total_bytes INTEGER,
    memory_used_bytes INTEGER,
    memory_usage_percent REAL,
    work_root_status TEXT NOT NULL CHECK (work_root_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    work_root_total_bytes INTEGER,
    work_root_used_bytes INTEGER,
    work_root_usage_percent REAL,
    disk_io_status TEXT NOT NULL CHECK (disk_io_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    disk_read_bytes_per_second REAL,
    disk_write_bytes_per_second REAL,
    disk_busy_percent REAL,
    network_status TEXT NOT NULL CHECK (network_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    network_receive_bytes_per_second REAL,
    network_transmit_bytes_per_second REAL,
    gpu_status TEXT NOT NULL CHECK (gpu_status IN ('available', 'warming_up', 'unsupported', 'collection_error')),
    gpus_json TEXT NOT NULL CHECK (length(gpus_json) <= 4096),
    CHECK ((cpu_status = 'available') = (cpu_usage_percent IS NOT NULL)),
    CHECK ((memory_status = 'available') = (memory_total_bytes IS NOT NULL AND memory_used_bytes IS NOT NULL AND memory_usage_percent IS NOT NULL)),
    CHECK ((work_root_status = 'available') = (work_root_total_bytes IS NOT NULL AND work_root_used_bytes IS NOT NULL AND work_root_usage_percent IS NOT NULL)),
    CHECK ((disk_io_status = 'available') = (disk_read_bytes_per_second IS NOT NULL AND disk_write_bytes_per_second IS NOT NULL AND disk_busy_percent IS NOT NULL)),
    CHECK ((network_status = 'available') = (network_receive_bytes_per_second IS NOT NULL AND network_transmit_bytes_per_second IS NOT NULL)),
    CHECK ((gpu_status = 'available') = (json_array_length(gpus_json) > 0))
);

CREATE TABLE node_telemetry_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    connection_generation INTEGER NOT NULL CHECK (connection_generation > 0),
    sample_sequence INTEGER NOT NULL CHECK (sample_sequence > 0),
    captured_at TEXT NOT NULL,
    received_at TEXT NOT NULL,
    cpu_usage_percent REAL,
    memory_total_bytes INTEGER,
    memory_used_bytes INTEGER,
    work_root_total_bytes INTEGER,
    work_root_used_bytes INTEGER,
    disk_read_bytes_per_second REAL,
    disk_write_bytes_per_second REAL,
    disk_busy_percent REAL,
    network_receive_bytes_per_second REAL,
    network_transmit_bytes_per_second REAL,
    gpus_json TEXT NOT NULL CHECK (length(gpus_json) <= 4096),
    UNIQUE (agent_id, connection_generation, sample_sequence)
);

CREATE INDEX node_telemetry_history_node_received
ON node_telemetry_history (node_id, received_at);

CREATE INDEX node_telemetry_history_received
ON node_telemetry_history (received_at);
