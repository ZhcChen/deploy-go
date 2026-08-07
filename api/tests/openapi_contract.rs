use std::collections::HashSet;

use deploy_go_api::openapi_document;
use serde_json::Value;

const PUBLIC_ENDPOINTS: &[&str] = &[
    "/healthz",
    "/readyz",
    "/api/v1/setup",
    "/api/v1/auth/login",
    "/api/v1/agent/enroll",
    "/api/v1/agent/refresh",
];

const CSRF_EXEMPT_ENDPOINTS: &[&str] = &[
    "/api/v1/setup",
    "/api/v1/auth/login",
    "/api/v1/auth/csrf",
    "/api/v1/agent/enroll",
    "/api/v1/agent/refresh",
];

#[test]
fn operation_ids_are_present_and_unique() {
    let document = openapi_document();
    let mut seen = HashSet::new();
    for path_item in document["paths"].as_object().unwrap().values() {
        for operation in path_item.as_object().unwrap().values() {
            let operation_id = operation["operationId"]
                .as_str()
                .expect("OpenAPI operation 缺少 operationId");
            assert!(
                seen.insert(operation_id.to_owned()),
                "重复 operationId: {operation_id}"
            );
        }
    }
}

#[test]
fn every_client_list_has_a_typed_page_response() {
    let document = openapi_document();
    let expected = [
        ("/api/v1/users", "UserListResponse"),
        ("/api/v1/nodes", "NodeListResponse"),
        ("/api/v1/ssh-credentials", "SshCredentialListResponse"),
        ("/api/v1/applications", "ApplicationListResponse"),
        (
            "/api/v1/applications/{application_id}/targets",
            "DeploymentTargetListResponse",
        ),
        ("/api/v1/deployments", "DeploymentListResponse"),
        ("/api/v1/audit-logs", "AuditLogListResponse"),
        (
            "/api/v1/users/{user_id}/applications",
            "ApplicationGrantListResponse",
        ),
    ];
    for (path, schema) in expected {
        let reference = &document["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["$ref"];
        assert_eq!(
            reference,
            &Value::String(format!("#/components/schemas/{schema}")),
            "{path} 缺少列表响应 schema"
        );
    }
}

#[test]
fn resource_lists_expose_limit_and_after_cursor_parameters() {
    let document = deploy_go_api::openapi_document();
    let value = serde_json::to_value(document).unwrap();
    for path in [
        "/api/v1/applications",
        "/api/v1/nodes",
        "/api/v1/users",
        "/api/v1/applications/{application_id}/targets",
        "/api/v1/users/{user_id}/applications",
        "/api/v1/ssh-credentials",
    ] {
        let parameters = value["paths"][path]["get"]["parameters"]
            .as_array()
            .unwrap();
        let names = parameters
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"limit"), "{path} 缺少 limit");
        assert!(names.contains(&"after"), "{path} 缺少 after");
    }
}

#[test]
fn session_bootstrap_headers_are_part_of_the_contract() {
    let document = openapi_document();
    let cases = [
        ("/api/v1/setup", "post", vec!["Origin"]),
        ("/api/v1/auth/login", "post", vec!["Origin"]),
        (
            "/api/v1/auth/csrf",
            "post",
            vec!["Origin", "Sec-Fetch-Site", "Sec-Fetch-Mode"],
        ),
        ("/api/v1/auth/profile", "patch", vec!["X-CSRF-Token"]),
        ("/api/v1/auth/preferences", "put", vec!["X-CSRF-Token"]),
    ];
    for (path, method, expected) in cases {
        let parameters = document["paths"][path][method]["parameters"]
            .as_array()
            .unwrap();
        let names = parameters
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect::<HashSet<_>>();
        for name in expected {
            assert!(names.contains(name), "{method} {path} 缺少 {name} header");
        }
    }
}

#[test]
fn terminal_websocket_handshake_is_described_without_query_secret() {
    let document = openapi_document();
    let operation = &document["paths"]["/api/v1/terminal-sessions/{session_id}/stream"]["get"];
    let parameters = operation["parameters"].as_array().unwrap();
    let names = parameters
        .iter()
        .filter_map(|parameter| parameter["name"].as_str())
        .collect::<HashSet<_>>();
    assert!(names.contains("Origin"));
    assert!(names.contains("Sec-WebSocket-Protocol"));
    assert!(!parameters.iter().any(|parameter| {
        parameter["in"] == "query"
            && parameter["name"]
                .as_str()
                .is_some_and(|name| name.contains("token") || name.contains("csrf"))
    }));
    assert!(operation["responses"].get("101").is_some());
    assert_eq!(
        operation["security"],
        serde_json::json!([{ "cookieAuth": [] }])
    );
}

#[test]
fn protected_operations_describe_cookie_and_csrf_security() {
    let document = openapi_document();
    assert_eq!(
        document["components"]["securitySchemes"]["cookieAuth"],
        serde_json::json!({
            "type": "apiKey",
            "in": "cookie",
            "name": "deploy_go_session"
        })
    );
    assert_eq!(
        document["components"]["securitySchemes"]["agentBearerAuth"],
        serde_json::json!({"type": "http", "scheme": "bearer"})
    );
    for (path, path_item) in document["paths"].as_object().unwrap() {
        for (method, operation) in path_item.as_object().unwrap() {
            let public = PUBLIC_ENDPOINTS.contains(&path.as_str());
            let agent_bearer = path.starts_with("/api/v1/agent/artifact-leases/")
                || path.starts_with("/api/v1/agent/env-registration-leases/")
                || path.starts_with("/api/v1/agent/application-env-leases/");
            if agent_bearer {
                assert_eq!(
                    operation["security"],
                    serde_json::json!([{ "agentBearerAuth": [] }]),
                    "{method} {path} 缺少 Agent Bearer auth"
                );
            } else if !public {
                assert_eq!(
                    operation["security"],
                    serde_json::json!([{ "cookieAuth": [] }]),
                    "{method} {path} 缺少 Cookie auth"
                );
            }
            let needs_csrf = !matches!(method.as_str(), "get" | "head" | "options")
                && !agent_bearer
                && !CSRF_EXEMPT_ENDPOINTS.contains(&path.as_str());
            if needs_csrf {
                let has_header =
                    operation["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|parameter| {
                            parameter["name"] == "X-CSRF-Token" && parameter["required"] == true
                        });
                assert!(has_header, "{method} {path} 缺少必需 CSRF header");
            }
        }
    }
}

#[test]
fn json_request_bodies_describe_validation_errors() {
    let document = openapi_document();
    for (path, path_item) in document["paths"].as_object().unwrap() {
        for (method, operation) in path_item.as_object().unwrap() {
            if operation.get("requestBody").is_some() {
                assert_eq!(
                    operation["responses"]["422"]["content"]["application/json"]["schema"]["$ref"],
                    "#/components/schemas/ErrorResponse",
                    "{method} {path} 缺少 422 ErrorResponse"
                );
            }
        }
    }
    assert_eq!(
        document["paths"]["/api/v1/setup"]["post"]["responses"]["403"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/ErrorResponse"
    );
}
