//! 对外 API：Git 来源配置、分支发现与目录（构建 Agent、Git 凭证）。
//!
//! 写入统一经过：Key 绑定 → 非生产守卫 → 节点白名单 → 与管理面共用的核心函数。
//! 目录接口只返回标识与状态，不返回节点连接信息、公钥或私钥。

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use serde::Serialize;
use utoipa::ToSchema;

use super::{
    ExternalApiKey, require_external_configurable_application, require_key_application_access,
};
use crate::{
    AppState, RequestId,
    application_sources::{
        self, ApplicationSourceResponse, GitRefDiscoveryResponse, SaveSourceRequest,
        SetBranchRequest,
    },
    auth::service_actor,
    error::{ApiError, ApiResult},
    external_keys,
};

/// 对外构建 Agent 目录项。Key 配置了可用节点时只含这些节点上的 Agent，未配置则含全部；不含主机、端口、用户名与目录。
#[derive(Serialize, ToSchema)]
pub struct ExternalBuildAgent {
    agent_id: String,
    node_id: String,
    node_name: String,
    node_status: String,
    /// 是否满足构建要求（节点在线、协议与能力满足）；与保存 Git 来源时的校验一致。
    ready: bool,
    /// `ready=false` 时的稳定原因码，例如 `agent_offline`。
    unavailable_code: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalBuildAgentListResponse {
    items: Vec<ExternalBuildAgent>,
}

/// 对外 Git 凭证目录项，只含元数据，外部只能引用，不能创建、轮换或停用。
#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ExternalGitCredential {
    id: String,
    name: String,
    fingerprint: String,
    status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalGitCredentialListResponse {
    items: Vec<ExternalGitCredential>,
}

#[derive(sqlx::FromRow)]
struct BuildAgentRow {
    agent_id: String,
    node_id: String,
    node_name: String,
    node_status: String,
    protocol_version: Option<i64>,
    capabilities_json: Option<String>,
}

const MAX_DIRECTORY_ITEMS: i64 = 500;

/// 对外保存 Git 来源时的仓库地址策略，比管理面共用的校验更严格，且只作用于这次保存：
/// 已保存的来源、分支发现与部署读取的是库里的地址，不会重新校验，管理面保存也不受影响。
///
/// - 只允许 `ssh`（含 `user@host:path` 写法）与 `https`；
/// - `https` 不允许用户信息（令牌应通过 `git_credential_id` 引用）；
/// - 主机是 `localhost`、回环、私网、CGNAT、链路本地（含云元数据地址）、未指定或组播的 IP 字面量时拒绝，
///   非规范写法的数字主机（十进制整数、八进制、十六进制）一并拒绝，因为 Git/SSH 会把它们解析成 IPv4。
///
/// 只做字符串级检查，不做 DNS 解析：域名形式的内网 Git 照常可用，也无法防止域名解析到内网地址。
pub(crate) fn validate_external_repository_url(url: &str, request_id: &str) -> ApiResult<()> {
    // 提示文案使用固定字符串，不回显用户提交的地址（可能含令牌）。
    let reject = |message: &'static str| Err(ApiError::validation(message, request_id));
    let url = url.trim();
    let (scheme, authority) = match url.split_once("://") {
        Some((scheme, rest)) => (
            scheme.to_ascii_lowercase(),
            rest.split(['/', '?', '#']).next().unwrap_or_default(),
        ),
        // scp 写法 `user@host:path`：第一个 ':' 之前是 `[user@]host`。
        // `helper::arg`（如 `ext::`、`fd::`）是 Git 的传输助手语法，含 '/' 的前缀则是本地路径，都不是主机，一并拒绝。
        None => {
            let Some((head, rest)) = url.split_once(':') else {
                return reject("对外 API 不接受该仓库地址：仅支持 ssh 或 https");
            };
            if rest.starts_with(':') || head.contains('/') {
                return reject("对外 API 不接受该仓库地址：仅支持 ssh 或 https");
            }
            ("ssh".to_owned(), head)
        }
    };
    if !matches!(scheme.as_str(), "ssh" | "https") {
        return reject("对外 API 不接受该仓库地址：仅支持 ssh 或 https");
    }
    let (userinfo, host_port) = match authority.rsplit_once('@') {
        Some((userinfo, host_port)) => (Some(userinfo), host_port),
        None => (None, authority),
    };
    if scheme == "https" && userinfo.is_some() {
        return reject(
            "对外 API 不接受该仓库地址：https 地址不能包含用户信息，请使用 Git 凭证引用",
        );
    }
    let host = match host_port.strip_prefix('[') {
        Some(rest) => rest.split_once(']').map_or("", |(host, _)| host),
        None if url.contains("://") => host_port.rsplit_once(':').map_or(host_port, |(h, _)| h),
        None => host_port,
    };
    if is_forbidden_host(host) {
        return reject("对外 API 不接受该仓库地址：不允许使用本机、内网或链路本地地址");
    }
    Ok(())
}

fn is_forbidden_host(host: &str) -> bool {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    fn forbidden_v4(ip: Ipv4Addr) -> bool {
        let [a, b, ..] = ip.octets();
        ip.is_loopback()
            || ip.is_private()
            || ip.is_link_local()
            || ip.is_unspecified()
            || ip.is_broadcast()
            || ip.is_multicast()
            || (a == 100 && (64..=127).contains(&b))
    }
    fn forbidden_v6(ip: Ipv6Addr) -> bool {
        if let Some(v4) = ip.to_ipv4_mapped() {
            return forbidden_v4(v4);
        }
        let first = ip.segments()[0];
        ip.is_loopback()
            || ip.is_unspecified()
            || ip.is_multicast()
            || (first & 0xfe00) == 0xfc00
            || (first & 0xffc0) == 0xfe80
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(ip) => forbidden_v4(ip),
            IpAddr::V6(ip) => forbidden_v6(ip),
        };
    }
    // 由数字、点和十六进制前缀构成、但不是规范 IPv4 的主机（如 `2130706433`、`0x7f.1`、`017700000001`）会被
    // Git/SSH 的 inet_aton 解析成 IPv4，无法逐一判断，统一拒绝。
    host.chars().all(|c| c.is_ascii_digit() || c == '.')
        || host.starts_with("0x")
        || host.contains(".0x")
}

#[utoipa::path(operation_id = "external_build_agents_list", get, path = "/external/v1/build-agents", responses((status = 200, body = ExternalBuildAgentListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_build_agents(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalBuildAgentListResponse>> {
    let rows = sqlx::query_as::<_, BuildAgentRow>(
        "SELECT a.id AS agent_id,n.id AS node_id,n.name AS node_name,n.status AS node_status,a.protocol_version,a.capabilities_json FROM agents a JOIN nodes n ON n.id=a.node_id WHERE (NOT EXISTS (SELECT 1 FROM external_api_key_nodes WHERE api_key_id=?) OR EXISTS (SELECT 1 FROM external_api_key_nodes key_node WHERE key_node.api_key_id=? AND key_node.node_id=n.id)) AND a.revoked_at IS NULL AND a.archived_at IS NULL AND n.archived_at IS NULL ORDER BY n.name COLLATE NOCASE,a.id LIMIT ?",
    )
    .bind(&key.id)
    .bind(&key.id)
    .bind(MAX_DIRECTORY_ITEMS)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let items = rows
        .into_iter()
        .map(|row| {
            let unavailable = application_sources::build_agent_unavailable(
                &row.node_status,
                false,
                row.protocol_version,
                row.capabilities_json.as_deref(),
            );
            ExternalBuildAgent {
                agent_id: row.agent_id,
                node_id: row.node_id,
                node_name: row.node_name,
                node_status: row.node_status,
                ready: unavailable.is_none(),
                unavailable_code: unavailable.map(|(code, _)| code.to_owned()),
            }
        })
        .collect();
    Ok(Json(ExternalBuildAgentListResponse { items }))
}

#[utoipa::path(operation_id = "external_git_credentials_list", get, path = "/external/v1/git-credentials", responses((status = 200, body = ExternalGitCredentialListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_git_credentials(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    _key: ExternalApiKey,
) -> ApiResult<Json<ExternalGitCredentialListResponse>> {
    let items = sqlx::query_as::<_, ExternalGitCredential>(
        "SELECT id,name,fingerprint,status FROM git_credentials ORDER BY name COLLATE NOCASE,id LIMIT ?",
    )
    .bind(MAX_DIRECTORY_ITEMS)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalGitCredentialListResponse { items }))
}

#[utoipa::path(operation_id = "external_application_git_source_show", get, path = "/external/v1/applications/{id}/git-source", params(("id" = String, Path)), responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    // 来源详情含仓库地址与凭证名，既有 `sources.git` 摘要不含这些字段：新增读取不对 prod 开放。
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::find_source_view(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_application_git_source_save", put, path = "/external/v1/applications/{id}/git-source", params(("id" = String, Path)), request_body = SaveSourceRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 201, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn save_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveSourceRequest>,
) -> ApiResult<(StatusCode, Json<ApplicationSourceResponse>)> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    validate_external_repository_url(&payload.repository_url, request_id.as_str())?;
    external_keys::require_key_agent_node(
        state.pool(),
        &key.id,
        &payload.build_agent_id,
        request_id.as_str(),
    )
    .await?;
    let (created, view) = application_sources::save_source_core(
        &state,
        &id,
        &payload,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(view),
    ))
}

/// 对已配置的来源做写前校验：应用必须非生产，且来源现用的构建 Agent 仍在 Key 的授权节点内
/// （发起发现和设置分支都会让该 Agent 参与，不能因为授权被收回后仍可驱动该节点）。
async fn require_configured_source_access(
    state: &AppState,
    key: &ExternalApiKey,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    require_key_application_access(state.pool(), key, application_id, request_id).await?;
    require_external_configurable_application(state.pool(), application_id, request_id).await?;
    let source =
        application_sources::find_source_view(state.pool(), application_id, request_id).await?;
    external_keys::require_key_agent_node(state.pool(), &key.id, &source.build_agent_id, request_id)
        .await
}

#[utoipa::path(operation_id = "external_application_git_source_refresh", post, path = "/external/v1/applications/{id}/git-source/refreshes", params(("id" = String, Path)), responses((status = 202, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn refresh_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<(StatusCode, Json<GitRefDiscoveryResponse>)> {
    require_configured_source_access(&state, &key, &id, request_id.as_str()).await?;
    let discovery = application_sources::refresh_core(
        &state,
        &service_actor().id,
        Some(&key.id),
        &id,
        request_id.as_str(),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(discovery)))
}

#[utoipa::path(operation_id = "external_application_git_source_refresh_show", get, path = "/external/v1/applications/{id}/git-source/refreshes/{refs_query_id}", params(("id" = String, Path), ("refs_query_id" = String, Path)), responses((status = 200, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_git_source_refresh(
    State(state): State<AppState>,
    Path((id, refs_query_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<GitRefDiscoveryResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::discovery_response(
            state.pool(),
            &id,
            &refs_query_id,
            request_id.as_str(),
        )
        .await?,
    ))
}

#[utoipa::path(operation_id = "external_application_git_source_branch_set", put, path = "/external/v1/applications/{id}/git-source/branch", params(("id" = String, Path)), request_body = SetBranchRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn set_git_branch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<SetBranchRequest>,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    require_configured_source_access(&state, &key, &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::set_branch_core(
            &state,
            &id,
            &payload,
            &service_actor().id,
            Some(&key.id),
            request_id.as_str(),
        )
        .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::validate_external_repository_url as check;

    fn ok(url: &str) {
        assert!(check(url, "req").is_ok(), "应允许 {url}");
    }
    fn denied(url: &str) {
        assert!(check(url, "req").is_err(), "应拒绝 {url}");
    }

    #[test]
    fn allows_public_ssh_https_and_hostname_based_internal_git() {
        ok("git@github.com:example/app.git");
        ok("ssh://git@github.com:22/example/app.git");
        ok("https://github.com/example/app.git");
        ok("https://git.example.com:8443/team/app.git");
        // 域名形式的内网 Git 不受影响（不做 DNS 解析）。
        ok("git@git.corp.local:team/app.git");
        ok("https://gitlab.internal.example.com/team/app.git");
        // 只有字母数字的普通主机名、含数字的域名不会被当成数字主机。
        ok("git@git1.example.com:team/app.git");
        ok("https://123.example.com/team/app.git");
        ok("https://[2606:4700::1111]/team/app.git");
    }

    #[test]
    fn rejects_other_schemes_and_https_userinfo() {
        for url in [
            "http://github.com/example/app.git",
            "git://github.com/example/app.git",
            "ftp://github.com/example/app.git",
            "ext::sh -c id",
            "ext::sh",
            "fd::3",
            "/srv/app:x",
            "./repo:x",
            "https://token@github.com/example/app.git",
            "https://token:secret@github.com/example/app.git",
        ] {
            denied(url);
        }
    }

    #[test]
    fn rejects_loopback_private_link_local_and_metadata_addresses() {
        for url in [
            "git@localhost:team/app.git",
            "git@LOCALHOST.:team/app.git",
            "https://app.localhost/team/app.git",
            "git@127.0.0.1:team/app.git",
            "ssh://git@127.0.0.1:2222/team/app.git",
            "git@10.0.0.5:team/app.git",
            "https://172.16.3.4/team/app.git",
            "https://192.168.1.10/team/app.git",
            "https://169.254.169.254/latest/meta-data",
            "https://100.64.0.1/team/app.git",
            "https://0.0.0.0/team/app.git",
            "https://[::1]/team/app.git",
            "https://[::ffff:10.0.0.1]/team/app.git",
            "https://[fe80::1]/team/app.git",
            "https://[fd00::1]/team/app.git",
        ] {
            denied(url);
        }
    }

    #[test]
    fn rejects_non_canonical_numeric_hosts_that_resolve_to_ipv4() {
        for url in [
            "https://2130706433/team/app.git",
            "git@2130706433:team/app.git",
            "https://0x7f.0.0.1/team/app.git",
            "https://0x7f000001/team/app.git",
            "https://017700000001/team/app.git",
            "https://127.1/team/app.git",
        ] {
            denied(url);
        }
    }

    #[test]
    fn rejects_empty_or_malformed_hosts() {
        for url in [
            "",
            "git@:team/app.git",
            "https:///team/app.git",
            "team/app.git",
        ] {
            denied(url);
        }
    }
}
