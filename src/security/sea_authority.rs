use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

const AUTHORITY_URL_ENV: &str = "SEA_AUTHORITY_URL";

pub fn env_override_reason(decision_var: &str, reason_var: &str) -> Option<String> {
    match std::env::var(decision_var) {
        Ok(value) if value.eq_ignore_ascii_case("deny") => {
            let reason = std::env::var(reason_var)
                .unwrap_or_else(|_| "denied by SEA Forge policy".to_string());
            Some(format!("Action blocked by SEA Forge: {reason}"))
        }
        _ => None,
    }
}

fn authority_url() -> Option<String> {
    std::env::var(AUTHORITY_URL_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn actor_id() -> String {
    std::env::var("SEA_ACTOR_ID")
        .ok()
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("LOGNAME").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "zeroclaw".to_string())
}

fn actor_type() -> String {
    std::env::var("SEA_ACTOR_TYPE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "agent".to_string())
}

fn identity_source() -> String {
    std::env::var("SEA_IDENTITY_SOURCE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "cli".to_string())
}

fn source_platform() -> String {
    std::env::var("SEA_SOURCE_PLATFORM")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "zeroclaw".to_string())
}

fn channel() -> String {
    std::env::var("SEA_CHANNEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "cli".to_string())
}

fn branch() -> String {
    std::env::var("SEA_GIT_BRANCH")
        .ok()
        .or_else(|| std::env::var("GIT_BRANCH").ok())
        .unwrap_or_default()
}

fn correlation_id() -> String {
    std::env::var("SEA_CORRELATION_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string())
}

fn workspace_identifier(workspace_dir: &Path) -> String {
    workspace_dir
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| workspace_dir.display().to_string())
}

async fn post_json(authority_url: &str, path: &str, payload: &Value) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .context("building SEA authority client")?;

    let response = client
        .post(format!("{authority_url}{path}"))
        .json(payload)
        .send()
        .await
        .with_context(|| format!("calling SEA authority endpoint {path}"))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("SEA authority endpoint {path} returned {status}: {body}");
    }

    serde_json::from_str(&body)
        .with_context(|| format!("decoding SEA authority response from {path}"))
}

pub async fn evaluate_action(
    workspace_dir: &Path,
    tool_name: &str,
    operation: &str,
    resource_type: &str,
    resource_id: &str,
    parameters: Value,
) -> Result<Option<String>> {
    let Some(base_url) = authority_url() else {
        return Ok(None);
    };

    let actor_id = actor_id();
    let actor_type = actor_type();
    let identity_source = identity_source();
    let source_platform = source_platform();
    let correlation_id = correlation_id();

    let onboard_request = json!({
        "actor_id": actor_id,
        "actor_type": actor_type,
        "source": identity_source,
        "workspace": workspace_identifier(workspace_dir),
        "platform": source_platform,
    });
    post_json(&base_url, "/policy/authority/onboard", &onboard_request).await?;

    let action_request = json!({
        "schema_version": "cam.v1",
        "action_id": Uuid::new_v4().to_string(),
        "correlation_id": correlation_id,
        "timestamp_utc": Utc::now().to_rfc3339(),
        "actor": {
            "actor_id": actor_id,
            "actor_type": actor_type,
        },
        "action": {
            "tool_name": tool_name,
            "operation": operation,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "parameters": parameters,
        },
        "context": {
            "repo": std::env::var("SEA_REPO").unwrap_or_default(),
            "branch": branch(),
            "environment": std::env::var("SEA_ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
            "workspace_root": workspace_dir.display().to_string(),
            "source_platform": source_platform,
            "channel": channel(),
            "identity_source": identity_source,
        },
        "evidence": {
            "identity_binding_source": "config/governance/identity_map.yaml",
            "tool_trace_ref": std::env::var("SEA_TOOL_TRACE_REF").unwrap_or_default(),
            "payload_hash": "",
        },
    });

    let response = post_json(&base_url, "/policy/authority/evaluate", &action_request).await?;
    let outcome = response
        .get("outcome")
        .and_then(Value::as_str)
        .unwrap_or("escalate");
    if outcome == "allow" {
        return Ok(None);
    }

    let reason_codes = response
        .get("reason_codes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "sea_authority_blocked".to_string());

    Ok(Some(format!(
        "Action blocked by SEA Forge: {outcome}: {reason_codes}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn evaluate_action_allows_when_authority_allows() {
        let server = MockServer::start().await;
        std::env::set_var(AUTHORITY_URL_ENV, server.uri());
        std::env::set_var("SEA_ACTOR_ID", "jdoe");
        std::env::set_var("SEA_ACTOR_TYPE", "human");
        std::env::set_var("SEA_IDENTITY_SOURCE", "github");

        Mock::given(method("POST"))
            .and(path("/policy/authority/onboard"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "principal": "sea:actors/jane-doe",
                "binding_resolution": "alias",
                "identity_binding_hash": "abc",
                "policy_bundle_ref": "bundle",
                "policy_bundle_hash": "def",
                "policy_bundle_version": "sha256:def"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/policy/authority/evaluate"))
            .and(body_partial_json(json!({
                "action": {
                    "tool_name": "file_write",
                    "operation": "write",
                    "resource_type": "file",
                    "resource_id": "docs/specs/test.md"
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "decision_id": Uuid::new_v4().to_string(),
                "action_id": Uuid::new_v4().to_string(),
                "correlation_id": "abc123",
                "outcome": "allow",
                "reason_codes": ["file_policy_allow"],
                "policy_refs": ["file-access-policy:allow"],
                "required_next_steps": [],
                "evaluated_at_utc": Utc::now().to_rfc3339(),
                "determinism": {
                    "policy_bundle_hash": "bundle",
                    "action_request_hash": "request",
                    "identity_binding_hash": "identity"
                }
            })))
            .mount(&server)
            .await;

        let workspace = tempdir().unwrap();
        let result = evaluate_action(
            workspace.path(),
            "file_write",
            "write",
            "file",
            "docs/specs/test.md",
            json!({"path": "docs/specs/test.md"}),
        )
        .await
        .unwrap();

        assert!(result.is_none());

        std::env::remove_var(AUTHORITY_URL_ENV);
        std::env::remove_var("SEA_ACTOR_ID");
        std::env::remove_var("SEA_ACTOR_TYPE");
        std::env::remove_var("SEA_IDENTITY_SOURCE");
    }

    #[tokio::test]
    async fn evaluate_action_blocks_when_authority_denies() {
        let server = MockServer::start().await;
        std::env::set_var(AUTHORITY_URL_ENV, server.uri());

        Mock::given(method("POST"))
            .and(path("/policy/authority/onboard"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "principal": "sea:actors/jane-doe",
                "binding_resolution": "alias",
                "identity_binding_hash": "abc",
                "policy_bundle_ref": "bundle",
                "policy_bundle_hash": "def",
                "policy_bundle_version": "sha256:def"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/policy/authority/evaluate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "decision_id": Uuid::new_v4().to_string(),
                "action_id": Uuid::new_v4().to_string(),
                "correlation_id": "abc123",
                "outcome": "deny",
                "reason_codes": ["file_policy_deny"],
                "policy_refs": ["file-access-policy:deny"],
                "required_next_steps": [],
                "evaluated_at_utc": Utc::now().to_rfc3339(),
                "determinism": {
                    "policy_bundle_hash": "bundle",
                    "action_request_hash": "request",
                    "identity_binding_hash": "identity"
                }
            })))
            .mount(&server)
            .await;

        let workspace = tempdir().unwrap();
        let result = evaluate_action(
            workspace.path(),
            "file_write",
            "write",
            "file",
            "libs/core/src/gen/output.py",
            json!({"path": "libs/core/src/gen/output.py"}),
        )
        .await
        .unwrap();

        assert_eq!(
            result.as_deref(),
            Some("Action blocked by SEA Forge: deny: file_policy_deny")
        );

        std::env::remove_var(AUTHORITY_URL_ENV);
    }
}
