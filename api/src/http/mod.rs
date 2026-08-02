use std::future::Future;

use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;

use crate::{RequestId, error::ApiError};

pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .map(RequestId::as_str)
            .unwrap_or("req_unknown")
            .to_owned();
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_: JsonRejection| {
                ApiError::validation("请求 JSON 字段或类型不正确", &request_id)
            })
    }
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("安装 Ctrl+C 信号处理器失败");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("安装 SIGTERM 信号处理器失败")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    wait_for_shutdown(ctrl_c, terminate).await;
}

async fn wait_for_shutdown(first: impl Future<Output = ()>, second: impl Future<Output = ()>) {
    tokio::select! {
        () = first => {},
        () = second => {},
    }
}

#[cfg(test)]
mod tests {
    use super::wait_for_shutdown;

    #[tokio::test]
    async fn shutdown_completes_when_either_signal_arrives() {
        wait_for_shutdown(std::future::ready(()), std::future::pending()).await;
    }
}
