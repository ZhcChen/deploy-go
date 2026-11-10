// 按领域合并的集成测试二进制：每个子文件是原独立测试文件，作为模块编译进同一个二进制，
// 以减少重复链接。新增测试文件请放入对应分组并在此登记。
#[path = "../common/mod.rs"]
mod common;

mod external_api;
mod external_api_keys;
mod external_app_management;
mod external_git_source;
mod external_openapi_contract;
mod external_preview_probe;
mod external_production_guard;
