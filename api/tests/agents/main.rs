// 按领域合并的集成测试二进制：每个子文件是原独立测试文件，作为模块编译进同一个二进制，
// 以减少重复链接。新增测试文件请放入对应分组并在此登记。
#[path = "../common/mod.rs"]
mod common;

mod agent_dispatcher;
mod agent_download;
mod agent_end_to_end;
mod agent_enrollment;
mod agent_releases;
mod agent_upgrades;
mod agent_websocket;
mod agents_store;
mod env_sync_dispatcher;
mod git_refs_dispatcher;
mod node_telemetry_api;
mod node_telemetry_load;
mod nodes_api;
mod runtime_log_ingest;
mod runtime_probe_api;
