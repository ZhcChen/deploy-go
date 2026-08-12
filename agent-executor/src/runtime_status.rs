use crate::release::FIXED_PATH;
use serde_json::{Map, Value};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;

const MAX_PAYLOAD_BYTES: usize = 32 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum RuntimeStatusError {
    #[error("runtime status request 无效")]
    InvalidRequest,
    #[error("运行时状态读取失败: {0}")]
    Collect(String),
    #[error("运行时状态输出无效")]
    InvalidOutput,
}

pub fn project_name(target_code: &str) -> Result<String, RuntimeStatusError> {
    if target_code.is_empty()
        || target_code.len() > 128
        || target_code.starts_with('-')
        || !target_code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RuntimeStatusError::InvalidRequest);
    }
    Ok(format!(
        "deploy-go-{}",
        target_code
            .to_ascii_lowercase()
            .chars()
            .map(
                |ch| if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '_'
                }
            )
            .collect::<String>()
    ))
}

pub fn validate_request_id(request_id: &str) -> bool {
    (1..=128).contains(&request_id.len())
        && request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub async fn collect(target_code: &str) -> Result<String, RuntimeStatusError> {
    let project = project_name(target_code)?;
    let compose = run_compose_ps(&project).await?;
    if !compose.is_empty() {
        return normalize_output(compose);
    }
    let ps = run_docker_ps(&project).await?;
    if !ps.is_empty() {
        return normalize_output(ps);
    }
    Err(RuntimeStatusError::InvalidOutput)
}

async fn run_compose_ps(project: &str) -> Result<String, RuntimeStatusError> {
    let mut command = tokio::process::Command::new("docker");
    command
        .args([
            "compose",
            "--project-name",
            project,
            "ps",
            "--format",
            "json",
        ])
        .current_dir("/")
        .env_clear()
        .env("PATH", FIXED_PATH)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| RuntimeStatusError::Collect("compose ps 超时".into()))?
        .map_err(|error| RuntimeStatusError::Collect(format!("compose ps 失败: {error}")))?;
    if !output.status.success() {
        return Err(RuntimeStatusError::Collect(utf8_trim(&output.stderr)));
    }
    Ok(utf8_trim(&output.stdout))
}

async fn run_docker_ps(project: &str) -> Result<String, RuntimeStatusError> {
    let mut command = tokio::process::Command::new("docker");
    command
        .args([
            "ps",
            "--filter",
            &format!("label=com.docker.compose.project={project}"),
            "--format",
            "json",
        ])
        .current_dir("/")
        .env_clear()
        .env("PATH", FIXED_PATH)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| RuntimeStatusError::Collect("docker ps 超时".into()))?
        .map_err(|error| RuntimeStatusError::Collect(format!("docker ps 失败: {error}")))?;
    if !output.status.success() {
        return Err(RuntimeStatusError::Collect(utf8_trim(&output.stderr)));
    }
    Ok(utf8_trim(&output.stdout))
}

fn utf8_trim(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_owned()
}

fn normalize_output(mut value: String) -> Result<String, RuntimeStatusError> {
    if value.is_empty() {
        return Err(RuntimeStatusError::InvalidOutput);
    }
    value.truncate(MAX_PAYLOAD_BYTES);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RuntimeStatusError::InvalidOutput);
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        let items = match parsed {
            Value::Array(items) => items,
            Value::Object(_) => vec![parsed],
            _ => return Err(RuntimeStatusError::InvalidOutput),
        };
        return sanitize_items(items);
    }
    let mut items = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value @ Value::Object(_)) => items.push(value),
            _ => return Err(RuntimeStatusError::InvalidOutput),
        }
    }
    if items.is_empty() {
        return Err(RuntimeStatusError::InvalidOutput);
    }
    sanitize_items(items)
}

fn sanitize_items(items: Vec<Value>) -> Result<String, RuntimeStatusError> {
    let sanitized = items
        .into_iter()
        .map(sanitize_item)
        .collect::<Option<Vec<_>>>()
        .ok_or(RuntimeStatusError::InvalidOutput)?;
    serde_json::to_string(&sanitized).map_err(|_| RuntimeStatusError::InvalidOutput)
}

fn sanitize_item(value: Value) -> Option<Value> {
    let object = value.as_object()?;
    let mut sanitized = Map::new();
    for (target, sources) in [
        ("id", &["ID"][..]),
        ("project", &["Project"][..]),
        ("name", &["Name", "Names"][..]),
        ("service", &["Service"][..]),
        ("state", &["State"][..]),
        ("health", &["Health"][..]),
        ("exit_code", &["ExitCode"][..]),
        ("publishers", &["Publishers"][..]),
    ] {
        if let Some(source) = sources.iter().find_map(|key| object.get(*key)) {
            sanitized.insert(target.to_owned(), source.clone());
        }
    }
    Some(Value::Object(sanitized))
}

#[cfg(test)]
mod tests {
    use super::{RuntimeStatusError, normalize_output, project_name};

    #[test]
    fn project_name_is_safe_and_stable() {
        assert_eq!(
            project_name("shared-prod-redis").unwrap(),
            "deploy-go-shared-prod-redis"
        );
        assert_eq!(project_name("PROD").unwrap(), "deploy-go-prod");
        assert!(matches!(
            project_name("-bad"),
            Err(RuntimeStatusError::InvalidRequest)
        ));
        assert!(matches!(
            project_name("bad;id"),
            Err(RuntimeStatusError::InvalidRequest)
        ));
    }

    #[test]
    fn normalize_output_accepts_array_object_and_json_lines() {
        let array = r#"[{"Service":"redis","State":"running","Command":"redis-server --requirepass secret"}]"#;
        assert_eq!(
            normalize_output(array.into()).unwrap(),
            r#"[{"service":"redis","state":"running"}]"#
        );
        let object = r#"{"Service":"redis","State":"running","Command":"redis-server --requirepass secret"}"#;
        assert_eq!(
            normalize_output(object.into()).unwrap(),
            r#"[{"service":"redis","state":"running"}]"#
        );
        let lines = "{\"Service\":\"redis\",\"State\":\"running\",\"Command\":\"redis-server\"}\n{\"Service\":\"redis-2\",\"State\":\"exited\"}\n";
        assert_eq!(
            normalize_output(lines.into()).unwrap(),
            r#"[{"service":"redis","state":"running"},{"service":"redis-2","state":"exited"}]"#
        );
    }

    #[test]
    fn normalize_output_rejects_empty_or_invalid_json() {
        assert!(matches!(
            normalize_output(String::new()),
            Err(RuntimeStatusError::InvalidOutput)
        ));
        assert!(matches!(
            normalize_output("not-json".into()),
            Err(RuntimeStatusError::InvalidOutput)
        ));
        assert!(matches!(
            normalize_output("\"string\"".into()),
            Err(RuntimeStatusError::InvalidOutput)
        ));
    }
}
