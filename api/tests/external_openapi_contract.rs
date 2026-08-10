use std::collections::BTreeSet;

use deploy_go_api::{external::external_openapi_document, openapi_document};

const EXPECTED_PATHS: &[&str] = &[
    "/external/v1/applications",
    "/external/v1/applications/{id}",
    "/external/v1/applications/{id}/deployments",
    "/external/v1/deployments/{id}",
    "/external/v1/deployments/{id}/cancel",
];

#[test]
fn external_openapi_only_exposes_the_deployment_surface() {
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
        "script_path",
        "parameter_schema",
        "requested_by",
        "external_api_key_id",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "对外 OpenAPI 不应包含 {forbidden}"
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
fn external_deployment_schema_keeps_internal_fields_out() {
    let document = external_openapi_document();
    let schema = &document["components"]["schemas"]["ExternalDeployment"];
    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("target_runs"));
    for forbidden in ["requested_by", "external_api_key_id", "snapshot_json"] {
        assert!(!properties.contains_key(forbidden), "{forbidden} 不应对外暴露");
    }
    let request_schema = &document["components"]["schemas"]["ExternalDeploymentRequest"];
    let properties = request_schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("parameters"));
    assert!(properties.contains_key("target_id"));
}
