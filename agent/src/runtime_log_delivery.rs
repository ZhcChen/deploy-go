//! 独立 HTTP 诊断通道：失败只退避，绝不参与部署任务结果或可靠输出 ACK。
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use deploy_go_runtime_log::{
    COMPONENTS, LogRecord, MAX_BATCH_BYTES, MAX_BATCH_RECORDS, NODE_LOG_ROOT, read_page,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::token_refresh::AccessProvider;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    epoch: String,
    sequence: u64,
}

#[derive(Serialize)]
struct Batch<'a> {
    component: &'a str,
    epoch: &'a str,
    after: u64,
    minimum: u64,
    evicted_bytes: u64,
    entries: &'a [LogRecord],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    epoch: String,
    acknowledged_sequence: u64,
}

fn open_cursor(path: &Path, write: bool) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(write)
        .create(write)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 4096
    {
        return Err(io::Error::other("runtime_log_cursor_invalid"));
    }
    Ok(file)
}

fn load_cursors(path: &Path) -> io::Result<BTreeMap<String, Cursor>> {
    let file = match open_cursor(path, false) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error),
    };
    let cursors: BTreeMap<String, Cursor> =
        serde_json::from_reader(file).map_err(io::Error::other)?;
    if cursors.len() > 4
        || cursors.iter().any(|(component, cursor)| {
            !COMPONENTS.contains(&component.as_str()) || cursor.epoch.parse::<ulid::Ulid>().is_err()
        })
    {
        return Err(io::Error::other("runtime_log_cursor_invalid"));
    }
    Ok(cursors)
}

fn save_cursors(path: &Path, cursors: &BTreeMap<String, Cursor>) -> io::Result<()> {
    let temporary = path.with_extension("next");
    let mut file = open_cursor(&temporary, true)?;
    file.set_len(0)?;
    file.write_all(&serde_json::to_vec(cursors).map_err(io::Error::other)?)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    fs::File::open(
        path.parent()
            .ok_or_else(|| io::Error::other("runtime_log_cursor_invalid"))?,
    )?
    .sync_all()
}

fn apply_receipt(
    cursor: &Cursor,
    receipt: Receipt,
    epoch: &str,
    last: u64,
    maximum: u64,
) -> io::Result<Cursor> {
    if receipt.epoch != epoch
        || receipt.acknowledged_sequence < last
        || receipt.acknowledged_sequence > maximum
        || receipt.acknowledged_sequence < cursor.sequence
    {
        return Err(io::Error::other("runtime_log_receipt_invalid"));
    }
    Ok(Cursor {
        epoch: receipt.epoch,
        sequence: receipt.acknowledged_sequence,
    })
}

pub fn start(mut endpoint: Url, client: reqwest::Client, access: Arc<dyn AccessProvider>) {
    endpoint.set_path("/api/v1/agent/runtime-logs");
    endpoint.set_query(None);
    endpoint.set_fragment(None);
    tokio::spawn(async move {
        let cursor_path = PathBuf::from("/var/lib/deploy-go-agent/runtime-log-cursors.json");
        let mut cursors = loop {
            match load_cursors(&cursor_path) {
                Ok(cursors) => break cursors,
                Err(_) => {
                    tracing::warn!(
                        error_code = "runtime_log_cursor_invalid",
                        "诊断游标不可用，退避后重读且保留本地日志"
                    );
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        };
        let mut delay = 10;
        loop {
            tokio::time::sleep(Duration::from_secs(delay)).await;
            delay = 10;
            for component in COMPONENTS {
                let root = PathBuf::from(NODE_LOG_ROOT).join(component);
                let cursor = cursors.get(component).cloned().unwrap_or_default();
                let after = cursor.sequence;
                let page = tokio::task::spawn_blocking(move || {
                    read_page(&root, after, MAX_BATCH_RECORDS, MAX_BATCH_BYTES - 2048)
                })
                .await;
                let mut page = match page {
                    Ok(Ok(page)) => page,
                    _ => continue,
                };
                let cursor = if cursor.epoch == page.epoch {
                    cursor
                } else {
                    let root = PathBuf::from(NODE_LOG_ROOT).join(component);
                    page = match tokio::task::spawn_blocking(move || {
                        read_page(&root, 0, MAX_BATCH_RECORDS, MAX_BATCH_BYTES - 2048)
                    })
                    .await
                    {
                        Ok(Ok(page)) => page,
                        _ => continue,
                    };
                    Cursor {
                        epoch: page.epoch.clone(),
                        sequence: 0,
                    }
                };
                let Some(last) = page.entries.last().map(|entry| entry.sequence) else {
                    continue;
                };
                let prepared =
                    match tokio::time::timeout(Duration::from_secs(5), access.prepare()).await {
                        Ok(Ok(prepared)) => prepared,
                        _ => {
                            delay = 60;
                            break;
                        }
                    };
                let batch = Batch {
                    component,
                    epoch: &page.epoch,
                    after: cursor.sequence,
                    minimum: page.minimum.unwrap_or(1),
                    evicted_bytes: page.evicted_bytes,
                    entries: &page.entries,
                };
                let receipt =
                    match send_batch(&client, &endpoint, &prepared.access_token, &batch).await {
                        Ok(receipt) => receipt,
                        Err(backoff) => {
                            delay = backoff;
                            break;
                        }
                    };
                let next = match apply_receipt(&cursor, receipt, &page.epoch, last, page.maximum) {
                    Ok(next) => next,
                    Err(_) => {
                        delay = 60;
                        break;
                    }
                };
                let mut pending = cursors.clone();
                pending.insert(component.into(), next);
                if save_cursors(&cursor_path, &pending).is_err() {
                    delay = 60;
                    break;
                }
                cursors = pending;
            }
        }
    });
}

async fn send_batch(
    client: &reqwest::Client,
    endpoint: &Url,
    token: &str,
    batch: &Batch<'_>,
) -> Result<Receipt, u64> {
    let response = client
        .post(endpoint.clone())
        .bearer_auth(token)
        .json(batch)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| 60_u64)?;
    if !response.status().is_success() {
        return Err(if response.status() == reqwest::StatusCode::NOT_FOUND {
            300
        } else {
            60
        });
    }
    // 不读取服务端错误正文；成功响应同样有界。
    bounded_receipt(response).await.map_err(|_| 60)
}

async fn bounded_receipt(mut response: reqwest::Response) -> io::Result<Receipt> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(io::Error::other)? {
        if bytes.len() + chunk.len() > 4096 {
            return Err(io::Error::other("runtime_log_receipt_invalid"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn http_collection_retries_old_api_and_rejects_unbounded_responses() {
        use axum::{Router, http::StatusCode, routing::post};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new()
            .route("/old", post(|| async { StatusCode::NOT_FOUND }))
            .route("/huge", post(|| async { "x".repeat(5000) }))
            .route(
                "/ok",
                post(|| async {
                    axum::Json(serde_json::json!({"epoch":"epoch","acknowledged_sequence":4}))
                }),
            );
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = crate::http_client::new_agent_client(Duration::from_secs(5));
        let batch = Batch {
            component: "agent",
            epoch: "epoch",
            after: 3,
            minimum: 1,
            evicted_bytes: 0,
            entries: &[],
        };
        for (path, backoff) in [("old", 300), ("huge", 60)] {
            assert_eq!(
                send_batch(
                    &client,
                    &format!("http://{address}/{path}").parse().unwrap(),
                    "fixture-token",
                    &batch
                )
                .await
                .err(),
                Some(backoff)
            );
        }
        let receipt = send_batch(
            &client,
            &format!("http://{address}/ok").parse().unwrap(),
            "fixture-token",
            &batch,
        )
        .await
        .unwrap();
        assert_eq!(receipt.acknowledged_sequence, 4);
        server.abort();
    }
    #[test]
    fn only_durable_matching_receipt_advances_cursor() {
        let cursor = Cursor {
            epoch: "epoch".into(),
            sequence: 3,
        };
        for (epoch, sequence) in [("other", 4), ("epoch", 2), ("epoch", 5)] {
            assert!(
                apply_receipt(
                    &cursor,
                    Receipt {
                        epoch: epoch.into(),
                        acknowledged_sequence: sequence
                    },
                    "epoch",
                    4,
                    4
                )
                .is_err()
            );
        }
        assert_eq!(
            apply_receipt(
                &cursor,
                Receipt {
                    epoch: "epoch".into(),
                    acknowledged_sequence: 4
                },
                "epoch",
                4,
                4
            )
            .unwrap()
            .sequence,
            4
        );
        let resumed = apply_receipt(
            &Cursor::default(),
            Receipt {
                epoch: "epoch".into(),
                acknowledged_sequence: 1000,
            },
            "epoch",
            64,
            1000,
        )
        .unwrap();
        assert_eq!(resumed.sequence, 1000);
    }
    #[test]
    fn cursor_survives_restart_and_rejects_links() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cursors.json");
        let cursors = BTreeMap::from([(
            "agent".into(),
            Cursor {
                epoch: ulid::Ulid::new().to_string(),
                sequence: 42,
            },
        )]);
        save_cursors(&path, &cursors).unwrap();
        assert_eq!(load_cursors(&path).unwrap()["agent"].sequence, 42);
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &path).unwrap();
        assert!(load_cursors(&path).is_err());
    }
}
