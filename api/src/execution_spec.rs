use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::error::{ApiError, ApiResult};

pub fn validate_script_path(root: &str, path: &str, request_id: &str) -> ApiResult<()> {
    if normalized_within(root, path) {
        Ok(())
    } else {
        Err(ApiError::validation(
            "脚本路径必须位于节点工作根目录",
            request_id,
        ))
    }
}

pub fn validate_secret_path(root: &str, path: &str, request_id: &str) -> ApiResult<()> {
    if normalized_within(root, path) {
        Ok(())
    } else {
        Err(ApiError::validation(
            "敏感文件路径必须位于节点 secrets root",
            request_id,
        ))
    }
}

pub fn validate_resolved_path(root: &Path, resolved: &Path, request_id: &str) -> ApiResult<()> {
    if root.is_absolute()
        && resolved.is_absolute()
        && resolved.starts_with(root)
        && resolved != root
    {
        Ok(())
    } else {
        Err(ApiError::validation(
            "解析后的路径逃逸允许根目录",
            request_id,
        ))
    }
}

pub fn validate_parameter_schema(schema: &Value, request_id: &str) -> ApiResult<()> {
    jsonschema::validator_for(schema)
        .map_err(|_| ApiError::validation("参数 schema 不是有效 JSON Schema", request_id))?;
    let object = schema
        .as_object()
        .ok_or_else(|| ApiError::validation("参数 schema 必须是对象", request_id))?;
    let allowed = ["type", "properties", "required", "additionalProperties"];
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || object.get("type").and_then(Value::as_str) != Some("object")
        || object.get("additionalProperties") != Some(&Value::Bool(false))
    {
        return Err(ApiError::validation(
            "参数 schema 超出首版允许范围",
            request_id,
        ));
    }
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| ApiError::validation("参数 schema 缺少 properties", request_id))?;
    if properties.len() > 50 {
        return Err(ApiError::validation("参数字段不能超过 50 个", request_id));
    }
    for (name, property) in properties {
        validate_parameter_name(name, request_id)?;
        let property = property
            .as_object()
            .ok_or_else(|| ApiError::validation("参数字段 schema 必须是对象", request_id))?;
        let allowed = [
            "type",
            "enum",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "x-options",
        ];
        if property.keys().any(|key| !allowed.contains(&key.as_str()))
            || !matches!(
                property.get("type").and_then(Value::as_str),
                Some("string" | "integer" | "number" | "boolean")
            )
        {
            return Err(ApiError::validation(
                "参数字段 schema 超出首版允许范围",
                request_id,
            ));
        }
        if let Some(options) = property.get("x-options") {
            let valid = options.as_array().is_some_and(|items| {
                let strings = items.iter().filter_map(Value::as_str).collect::<Vec<_>>();
                !items.is_empty()
                    && items.len() <= 32
                    && strings.len() == items.len()
                    && strings.iter().copied().collect::<HashSet<_>>().len() == items.len()
                    && items.iter().all(|item| {
                        item.as_str().is_some_and(|value| {
                            !value.is_empty()
                                && value.len() <= 64
                                && !value.chars().any(char::is_control)
                        })
                    })
            });
            if !valid {
                return Err(ApiError::validation(
                    "参数字段 x-options 必须包含 1 到 32 个有效字符串",
                    request_id,
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_parameter_values(
    schema: &Value,
    values: &Value,
    request_id: &str,
) -> ApiResult<()> {
    validate_parameter_schema(schema, request_id)?;
    let validator = jsonschema::validator_for(schema)
        .map_err(|_| ApiError::validation("参数 schema 无效", request_id))?;
    if !validator.is_valid(values) {
        return Err(ApiError::validation(
            "部署参数不符合目标 schema",
            request_id,
        ));
    }
    let properties = schema["properties"]
        .as_object()
        .ok_or_else(|| ApiError::validation("参数 schema 缺少 properties", request_id))?;
    for (name, property) in properties {
        let Some(options) = property.get("x-options").and_then(Value::as_array) else {
            continue;
        };
        let allowed = options
            .iter()
            .filter_map(Value::as_str)
            .collect::<HashSet<_>>();
        let selected = values
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if selected.is_empty() || selected.iter().any(|value| !allowed.contains(value)) {
            return Err(ApiError::validation("部署参数包含未允许的选项", request_id));
        }
    }
    Ok(())
}

pub fn parameter_tokens(values: &Value, request_id: &str) -> ApiResult<Vec<String>> {
    let values = values
        .as_object()
        .ok_or_else(|| ApiError::validation("部署参数必须是对象", request_id))?;
    let mut names = values.keys().collect::<Vec<_>>();
    names.sort();
    let mut tokens = Vec::new();
    for name in names {
        validate_parameter_name(name, request_id)?;
        match &values[name] {
            Value::Bool(true) => tokens.push(format!("--{name}")),
            Value::Bool(false) => {}
            Value::String(value) => {
                tokens.push(format!("--{name}"));
                tokens.push(value.clone());
            }
            Value::Number(value) => {
                tokens.push(format!("--{name}"));
                tokens.push(value.to_string());
            }
            _ => {
                return Err(ApiError::validation(
                    "部署参数只能是字符串、数字或布尔值",
                    request_id,
                ));
            }
        }
    }
    Ok(tokens)
}

pub fn validate_verification_config(
    config: &Value,
    work_root: &str,
    request_id: &str,
) -> ApiResult<()> {
    let object = config
        .as_object()
        .ok_or_else(|| ApiError::validation("验证配置必须是对象", request_id))?;
    match object.get("type").and_then(Value::as_str) {
        Some("http") => {
            exact_keys(
                object,
                &["type", "path", "expected_status", "timeout_ms"],
                request_id,
            )?;
            let path = object
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let status = object
                .get("expected_status")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let timeout = object
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            if !path.starts_with('/')
                || path.contains(char::is_control)
                || !(100..=599).contains(&status)
                || !(100..=60_000).contains(&timeout)
            {
                return Err(ApiError::validation("HTTP 验证配置无效", request_id));
            }
        }
        Some("tcp") => {
            exact_keys(object, &["type", "port", "timeout_ms"], request_id)?;
            let port = object
                .get("port")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let timeout = object
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            if !(1..=65_535).contains(&port) || !(100..=60_000).contains(&timeout) {
                return Err(ApiError::validation("TCP 验证配置无效", request_id));
            }
        }
        Some("command") => {
            exact_keys(object, &["type", "path", "args", "timeout_ms"], request_id)?;
            let path = object
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            validate_script_path(work_root, path, request_id)?;
            let args = object
                .get("args")
                .and_then(Value::as_array)
                .ok_or_else(|| ApiError::validation("command args 必须是数组", request_id))?;
            if args.len() > 32
                || args.iter().any(|value| {
                    value.as_str().is_none_or(|token| {
                        token.len() > 1024
                            || token.contains('\0')
                            || token.contains('\n')
                            || token.contains('\r')
                    })
                })
            {
                return Err(ApiError::validation(
                    "command args 超出允许范围",
                    request_id,
                ));
            }
            let timeout = object
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            if !(100..=60_000).contains(&timeout) {
                return Err(ApiError::validation("command timeout 无效", request_id));
            }
        }
        _ => return Err(ApiError::validation("验证配置类型不受支持", request_id)),
    }
    Ok(())
}

pub fn validate_environment_key(key: &str, request_id: &str) -> ApiResult<()> {
    if (1..=64).contains(&key.len())
        && key.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_uppercase() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
    {
        Ok(())
    } else {
        Err(ApiError::validation("环境变量键格式不正确", request_id))
    }
}

pub fn validate_parameter_name(name: &str, request_id: &str) -> ApiResult<()> {
    if (1..=64).contains(&name.len())
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(())
    } else {
        Err(ApiError::validation(
            "参数名必须是 kebab-case 长选项名",
            request_id,
        ))
    }
}

pub fn snapshot_hash(value: &Value) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.to_string().as_bytes()))
}

pub struct TargetSnapshotInput<'a> {
    pub application_id: &'a str,
    pub node_id: &'a str,
    pub environment: &'a str,
    pub script_path: &'a str,
    pub parameter_schema: &'a Value,
    pub timeout_seconds: i64,
    pub verification_config: &'a Value,
    pub secret_refs: &'a [(String, String)],
    pub privileged_release: bool,
    pub version: i64,
}

pub fn target_snapshot(input: TargetSnapshotInput<'_>) -> Value {
    let refs: Vec<Value> = input
        .secret_refs
        .iter()
        .map(|(key, path)| json!({"environment_key":key,"file_path":path}))
        .collect();
    json!({"application_id":input.application_id,"node_id":input.node_id,"environment":input.environment,"script_path":input.script_path,"parameter_schema":input.parameter_schema,"timeout_seconds":input.timeout_seconds,"verification_config":input.verification_config,"secret_file_references":refs,"privileged_release":input.privileged_release,"version":input.version})
}

fn normalized_within(root: &str, candidate: &str) -> bool {
    let Some(root) = normalize_absolute(root) else {
        return false;
    };
    let Some(candidate) = normalize_absolute(candidate) else {
        return false;
    };
    candidate.starts_with(&root) && candidate != root
}

fn normalize_absolute(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if !path.is_absolute() || value.chars().any(char::is_control) {
        return None;
    }
    let mut output = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(value) => output.push(value),
            Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(output)
}

fn exact_keys(object: &Map<String, Value>, allowed: &[&str], request_id: &str) -> ApiResult<()> {
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || allowed.iter().any(|key| !object.contains_key(*key))
    {
        Err(ApiError::validation(
            "验证配置字段不完整或包含未知字段",
            request_id,
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalized_within, validate_parameter_schema, validate_parameter_values,
        validate_verification_config,
    };
    use serde_json::json;

    #[test]
    fn paths_reject_traversal_and_resolved_escape() {
        assert!(normalized_within("/srv/apps", "/srv/apps/api/deploy.sh"));
        assert!(!normalized_within("/srv/apps", "/srv/apps/../etc/passwd"));
        assert!(!normalized_within("/srv/apps", "/srv/apps-evil/deploy.sh"));
        assert!(!normalized_within("/srv/apps", "/etc/deploy.sh"));
        assert!(!normalized_within("/srv/apps", "/srv/apps"));
    }

    #[test]
    fn parameter_schema_rejects_unknown_fields_and_values() {
        let schema = json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false});
        assert!(validate_parameter_schema(&schema, "req_test").is_ok());
        assert!(
            validate_parameter_values(&schema, &json!({"release-version":"1.0"}), "req_test")
                .is_ok()
        );
        assert!(
            validate_parameter_values(
                &schema,
                &json!({"release-version":"1.0","shell":"bad"}),
                "req_test"
            )
            .is_err()
        );
        assert!(
            validate_parameter_schema(
                &json!({"type":"object","$ref":"bad","properties":{},"additionalProperties":false}),
                "req_test"
            )
            .is_err()
        );
    }

    #[test]
    fn parameter_schema_accepts_bounded_ui_options() {
        let schema = json!({
            "type":"object",
            "properties":{"modules":{"type":"string","maxLength":512,"x-options":["api","admin"]}},
            "required":["modules"],
            "additionalProperties":false
        });
        assert!(validate_parameter_schema(&schema, "req_test").is_ok());
        assert!(
            validate_parameter_values(&schema, &json!({"modules":"api,admin"}), "req_test").is_ok()
        );
        assert!(
            validate_parameter_values(&schema, &json!({"modules":"api,shell"}), "req_test")
                .is_err()
        );
    }

    #[test]
    fn verification_config_rejects_free_command_text() {
        assert!(validate_verification_config(&json!({"type":"command","path":"/srv/apps/check","args":["--ready"],"timeout_ms":1000}), "/srv/apps", "req_test").is_ok());
        assert!(
            validate_verification_config(
                &json!({"type":"command","command":"curl localhost"}),
                "/srv/apps",
                "req_test"
            )
            .is_err()
        );
    }
}
