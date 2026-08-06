use jsonschema::Validator;
use serde_json::{Value, json};

fn validator(schema: &str) -> Validator {
    let schema: Value = serde_json::from_str(schema).unwrap();
    jsonschema::validator_for(&schema).unwrap()
}

fn artifact(module: &str, size: u64) -> Value {
    json!({
        "module": module,
        "path": format!("{module}/release.tar.gz"),
        "sha256": "0".repeat(64),
        "size": size
    })
}

#[test]
fn artifact_manifest_enforces_file_limits_and_strict_fields() {
    let validator = validator(include_str!(
        "../../docs/standards/deploy-artifact-manifest.schema.json"
    ));
    let manifest = |artifacts: Vec<Value>| {
        json!({
            "schema_version": 1,
            "release_version": "202608070001",
            "commit_sha": "0".repeat(40),
            "artifacts": artifacts
        })
    };

    let maximum = (0..256)
        .map(|index| artifact(&format!("module-{index}"), 536_870_912))
        .collect();
    assert!(validator.is_valid(&manifest(maximum)));

    let too_many = (0..257)
        .map(|index| artifact(&format!("module-{index}"), 1))
        .collect();
    assert!(!validator.is_valid(&manifest(too_many)));
    assert!(!validator.is_valid(&manifest(vec![artifact("api", 536_870_913)])));

    let mut unknown = manifest(vec![artifact("api", 1)]);
    unknown["artifacts"][0]["source_url"] = json!("https://example.test/file");
    assert!(!validator.is_valid(&unknown));
}

#[test]
fn env_manifest_accepts_only_bounded_dotenv_declarations() {
    let validator = validator(include_str!(
        "../../docs/standards/deploy-env-manifest.schema.json"
    ));
    let manifest = |file_name: &str, size: u64, format: &str| {
        json!({
            "schema_version": 1,
            "commit_sha": "0".repeat(40),
            "files": [{
                "file_name": file_name,
                "module": "api",
                "sha256": "0".repeat(64),
                "size": size,
                "format": format
            }]
        })
    };

    assert!(validator.is_valid(&manifest("api.env", 1_048_576, "dotenv-v1")));
    assert!(!validator.is_valid(&manifest("../api.env", 1, "dotenv-v1")));
    assert!(!validator.is_valid(&manifest("api.env", 1_048_577, "dotenv-v1")));
    assert!(!validator.is_valid(&manifest("api.env", 1, "dotenv-v2")));
}
