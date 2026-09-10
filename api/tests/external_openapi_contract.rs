use std::collections::BTreeSet;

use deploy_go_api::{external::external_openapi_document, openapi_document};

const EXPECTED_PATHS: &[&str] = &[
    "/external/v1/applications",
    "/external/v1/applications/{id}",
    "/external/v1/applications/{id}/deployments",
    "/external/v1/applications/{id}/env-files",
    "/external/v1/applications/{id}/env-files/{env_file_id}",
    "/external/v1/applications/{id}/targets",
    "/external/v1/applications/{id}/workspace-source",
    "/external/v1/deployment-targets/{target_id}",
    "/external/v1/deployment-targets/{target_id}/status",
    "/external/v1/deployments/{id}",
    "/external/v1/deployments/{id}/cancel",
];

#[test]
fn external_openapi_only_exposes_the_external_surface() {
    let document = external_openapi_document();
    let paths = document["paths"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = EXPECTED_PATHS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(paths, expected);

    let serialized = serde_json::to_string(&document).unwrap().to_lowercase();
    for forbidden in [
        "/api/v1",
        "credential",
        "terminal",
        "audit",
        "application-env",
        "env-gate",
        "requested_by",
        "external_api_key_id",
        "ciphertext",
        "nonce",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "对外 OpenAPI 不应包含 {forbidden}"
        );
    }
}

#[test]
fn external_application_update_schema_is_explicit() {
    let document = external_openapi_document();
    let update = &document["paths"]["/external/v1/applications/{id}"]["patch"];
    assert_eq!(
        update["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        serde_json::json!("#/components/schemas/ExternalApplicationUpdateRequest")
    );

    let schema = &document["components"]["schemas"]["ExternalApplicationUpdateRequest"];
    assert_eq!(schema["required"], serde_json::json!(["version"]));
    let properties = schema["properties"].as_object().unwrap();
    for allowed in [
        "version",
        "name",
        "slug",
        "description",
        "environment",
        "app_type",
        "type_version",
        "tags",
        "parameter_schema",
        "verification_config",
    ] {
        assert!(properties.contains_key(allowed), "缺少字段 {allowed}");
    }
    for forbidden in ["status", "template_id", "script_path", "requested_by"] {
        assert!(
            !properties.contains_key(forbidden),
            "{forbidden} 不应允许外部编辑"
        );
    }

    let detail = &document["components"]["schemas"]["ExternalApplicationDetail"];
    let detail_properties = detail["properties"].as_object().unwrap();
    for allowed in [
        "app_type",
        "type_version",
        "tags",
        "parameter_schema",
        "verification_config",
        "version",
    ] {
        assert!(
            detail_properties.contains_key(allowed),
            "应用详情缺少字段 {allowed}"
        );
    }
}

#[test]
fn external_application_create_schema_is_explicit() {
    let document = external_openapi_document();
    let create = &document["paths"]["/external/v1/applications"]["post"];
    assert_eq!(
        create["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        serde_json::json!("#/components/schemas/ExternalApplicationCreateRequest")
    );

    let schema = &document["components"]["schemas"]["ExternalApplicationCreateRequest"];
    assert_eq!(
        schema["required"],
        serde_json::json!(["name", "slug", "environment"])
    );
    let properties = schema["properties"].as_object().unwrap();
    for allowed in [
        "name",
        "slug",
        "description",
        "environment",
        "app_type",
        "type_version",
        "tags",
        "parameter_schema",
        "verification_config",
    ] {
        assert!(properties.contains_key(allowed), "缺少字段 {allowed}");
    }
    for forbidden in ["id", "status", "version", "template_id", "requested_by"] {
        assert!(
            !properties.contains_key(forbidden),
            "{forbidden} 不应允许外部创建时指定"
        );
    }
}

#[test]
fn external_openapi_uses_bearer_api_key_security() {
    let document = external_openapi_document();
    assert_eq!(
        document["components"]["securitySchemes"]["externalApiKey"],
        serde_json::json!({
            "type": "http",
            "scheme": "bearer",
            "description": "管理端创建的外部部署 API Key，格式为 dgx_..."
        })
    );
    for (path, path_item) in document["paths"].as_object().unwrap() {
        for (method, operation) in path_item.as_object().unwrap() {
            assert_eq!(
                operation["security"],
                serde_json::json!([{ "externalApiKey": [] }]),
                "{method} {path} 缺少外部 API Key security"
            );
        }
    }
    let create = &document["paths"]["/external/v1/applications/{id}/deployments"]["post"];
    let parameter_names = create["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|parameter| parameter["name"].as_str())
        .collect::<Vec<_>>();
    assert!(parameter_names.contains(&"Idempotency-Key"));
}

#[test]
fn internal_openapi_does_not_contain_external_paths() {
    let document = openapi_document();
    let serialized = serde_json::to_string(&document).unwrap();
    assert!(!serialized.contains("/external/v1"));
}

#[test]
fn external_deployment_contract_schema_covers_configurable_fields() {
    let document = external_openapi_document();
    let request = &document["components"]["schemas"]["SaveTargetRequest"];
    let properties = request["properties"].as_object().unwrap();
    for allowed in [
        "node_id",
        "target_code",
        "script_path",
        "timeout_seconds",
        "execution_mode",
        "secret_file_references",
        "image_spec",
        "version",
    ] {
        assert!(
            properties.contains_key(allowed),
            "部署目标契约缺少字段 {allowed}"
        );
    }
    for forbidden in [
        "environment",
        "parameter_schema",
        "verification_config",
        "snapshot_hash",
        "privileged_release",
    ] {
        assert!(
            !properties.contains_key(forbidden),
            "{forbidden} 不应允许外部直接写入"
        );
    }

    let env_schema = &document["components"]["schemas"]["ExternalEnvFile"];
    let env_properties = env_schema["properties"].as_object().unwrap();
    for allowed in [
        "file_name",
        "module",
        "format",
        "current_version",
        "current_digest",
        "pending_count",
        "failed_count",
    ] {
        assert!(
            env_properties.contains_key(allowed),
            "Env 元数据缺少字段 {allowed}"
        );
    }
    for forbidden in ["content", "ciphertext", "nonce", "key_version"] {
        assert!(
            !env_properties.contains_key(forbidden),
            "{forbidden} 不应出现在对外 Env 响应中"
        );
    }
}

#[test]
fn external_deployment_schema_keeps_internal_fields_out() {
    let document = external_openapi_document();
    let schema = &document["components"]["schemas"]["ExternalDeployment"];
    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("target_runs"));
    for forbidden in ["requested_by", "external_api_key_id", "snapshot_json"] {
        assert!(
            !properties.contains_key(forbidden),
            "{forbidden} 不应对外暴露"
        );
    }
    let request_schema = &document["components"]["schemas"]["ExternalDeploymentRequest"];
    let properties = request_schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("parameters"));
    assert!(properties.contains_key("target_id"));
}
