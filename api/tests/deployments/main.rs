// 按领域合并的集成测试二进制：每个子文件是原独立测试文件，作为模块编译进同一个二进制，
// 以减少重复链接。新增测试文件请放入对应分组并在此登记。
#[path = "../common/mod.rs"]
mod common;

mod artifacts_api;
mod deploy_event_protocol;
mod deployer_release;
mod deployment_manifests;
mod deployment_recovery;
mod deployment_runtime;
mod deployment_targets_api;
mod deployments_api;
mod execution_spec;
mod two_stage_deployment;
