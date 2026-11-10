// 按领域合并的集成测试二进制：每个子文件是原独立测试文件，作为模块编译进同一个二进制，
// 以减少重复链接。新增测试文件请放入对应分组并在此登记。
#[path = "../common/mod.rs"]
mod common;

mod application_configs_api;
mod application_envs_api;
mod application_sources_api;
mod application_templates_api;
mod applications_api;
mod configuration_centers_api;
mod credential_encryption;
mod git_credentials_api;
mod ssh_credentials_api;
mod workspace_sources_api;
