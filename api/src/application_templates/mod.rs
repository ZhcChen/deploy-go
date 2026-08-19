use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use deploy_go_container_template::{
    TemplateDescriptor, TemplateFileDescriptor, all_template_descriptors, template_descriptor,
    template_from_id,
};

use crate::{
    AppState, RequestId,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ApplicationTemplateFileResponse {
    path: String,
    deploy_path: Option<String>,
    label: String,
    format: String,
    language: String,
    role: String,
    delivery: String,
    editable: bool,
    sensitive: bool,
    description: String,
    recommended_changes: String,
    digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ApplicationTemplateResponse {
    id: String,
    version: String,
    name: String,
    summary: String,
    deployment_mechanism: String,
    default_image: String,
    default_port: u16,
    digest: String,
    files: Vec<ApplicationTemplateFileResponse>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ApplicationTemplateListResponse {
    items: Vec<ApplicationTemplateResponse>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTemplateFileQuery {
    path: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/application-templates", get(list))
        .route("/application-templates/{id}", get(show))
        .route("/application-templates/{id}/file", get(file))
}

#[utoipa::path(
    operation_id = "application_templates_list",
    get,
    path = "/api/v1/application-templates",
    responses(
        (status = 200, body = ApplicationTemplateListResponse),
        (status = 401, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn list(
    State(_state): State<AppState>,
    Extension(_request_id): Extension<RequestId>,
    _actor: AuthUser,
) -> ApiResult<Json<ApplicationTemplateListResponse>> {
    Ok(Json(ApplicationTemplateListResponse {
        items: all_template_descriptors()
            .into_iter()
            .map(|template| response(template, false))
            .collect(),
    }))
}

#[utoipa::path(
    operation_id = "application_templates_show",
    get,
    path = "/api/v1/application-templates/{id}",
    params(("id" = String, Path)),
    responses(
        (status = 200, body = ApplicationTemplateResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn show(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    _actor: AuthUser,
) -> ApiResult<Json<ApplicationTemplateResponse>> {
    let template = find(&id, request_id.as_str())?;
    Ok(Json(response(template, true)))
}

#[utoipa::path(
    operation_id = "application_templates_file",
    get,
    path = "/api/v1/application-templates/{id}/file",
    params(
        ("id" = String, Path),
        ("path" = String, Query)
    ),
    responses(
        (status = 200, body = ApplicationTemplateFileResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn file(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ApplicationTemplateFileQuery>,
    Extension(request_id): Extension<RequestId>,
    _actor: AuthUser,
) -> ApiResult<Json<ApplicationTemplateFileResponse>> {
    let template = find(&id, request_id.as_str())?;
    let file = template
        .files
        .into_iter()
        .find(|file| file.path == query.path)
        .ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::NOT_FOUND,
                "application_template_file_not_found",
                "模板文件不存在",
                request_id.as_str(),
            )
        })?;
    Ok(Json(file_response(file, true)))
}

fn find(id: &str, request_id: &str) -> ApiResult<TemplateDescriptor> {
    let Some(template) = template_from_id(id).map(template_descriptor) else {
        return Err(ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "application_template_not_found",
            "应用模板不存在",
            request_id,
        ));
    };
    Ok(template)
}

fn response(template: TemplateDescriptor, include_content: bool) -> ApplicationTemplateResponse {
    ApplicationTemplateResponse {
        id: template.id,
        version: template.version,
        name: template.name,
        summary: template.summary,
        deployment_mechanism: serde_json::to_value(template.deployment_mechanism)
            .expect("模板部署机制可序列化")
            .as_str()
            .expect("模板部署机制为字符串")
            .to_owned(),
        default_image: template.default_image,
        default_port: template.default_port,
        digest: template.digest,
        files: template
            .files
            .into_iter()
            .map(|file| file_response(file, include_content))
            .collect(),
    }
}

fn file_response(
    file: TemplateFileDescriptor,
    include_content: bool,
) -> ApplicationTemplateFileResponse {
    ApplicationTemplateFileResponse {
        path: file.path,
        deploy_path: file.deploy_path,
        label: file.label,
        format: serde_json::to_value(file.format)
            .expect("模板格式可序列化")
            .as_str()
            .expect("模板格式为字符串")
            .to_owned(),
        language: file.language,
        role: serde_json::to_value(file.role)
            .expect("模板文件角色可序列化")
            .as_str()
            .expect("模板文件角色为字符串")
            .to_owned(),
        delivery: serde_json::to_value(file.delivery)
            .expect("模板文件交付方式可序列化")
            .as_str()
            .expect("模板文件交付方式为字符串")
            .to_owned(),
        editable: file.editable,
        sensitive: file.sensitive,
        description: file.description,
        recommended_changes: file.recommended_changes,
        digest: file.digest,
        content: include_content.then_some(file.content),
    }
}
