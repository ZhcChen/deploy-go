use std::future::Future;

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
