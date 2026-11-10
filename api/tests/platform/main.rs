// 按领域合并的集成测试二进制：每个子文件是原独立测试文件，作为模块编译进同一个二进制，
// 以减少重复链接。新增测试文件请放入对应分组并在此登记。
#[path = "../common/mod.rs"]
mod common;

mod audit_api;
mod auth_api;
mod authorization_api;
mod health;
mod openapi_contract;
mod session_lifetime;
mod settings_api;
mod terminal_api;
mod terminal_authorization;
mod terminal_store;
mod terminal_websocket;
mod users_api;
