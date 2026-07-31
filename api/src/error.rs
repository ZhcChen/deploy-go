use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub request_id: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &str, message: &str, request_id: &str) -> Self {
        Self {
            status,
            body: ErrorResponse {
                code: code.to_owned(),
                message: message.to_owned(),
                request_id: request_id.to_owned(),
            },
        }
    }

    pub fn unauthorized(request_id: &str) -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "登录状态无效或已过期",
            request_id,
        )
    }

    pub fn forbidden(request_id: &str) -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "没有执行该操作的权限",
            request_id,
        )
    }

    pub fn validation(message: &str, request_id: &str) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            message,
            request_id,
        )
    }

    pub fn conflict(code: &str, message: &str, request_id: &str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message, request_id)
    }

    pub fn not_found(request_id: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "资源不存在", request_id)
    }

    pub fn internal(request_id: &str) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "服务处理请求时发生错误",
            request_id,
        )
    }

    pub fn service_not_ready(request_id: &str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: ErrorResponse {
                code: "service_not_ready".to_owned(),
                message: "服务尚未就绪".to_owned(),
                request_id: request_id.to_owned(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.body)).into_response()
    }
}
