use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use deploy_go_agent::{
    credential_store::{AgentCredentials, CredentialStore, PendingRotation},
    token_refresh::{
        AccessProvider, CredentialAccessProvider, TokenPair, TokenRefreshError, TokenRefresher,
    },
};

struct MockRefresher {
    calls: Mutex<Vec<(String, String)>>,
    next_refresh_token: String,
}

#[async_trait]
impl TokenRefresher for MockRefresher {
    async fn refresh(
        &self,
        refresh_token: &str,
        rotation_id: &str,
    ) -> Result<TokenPair, TokenRefreshError> {
        self.calls
            .lock()
            .unwrap()
            .push((refresh_token.to_owned(), rotation_id.to_owned()));
        Ok(TokenPair {
            agent_id: "agent_01".to_owned(),
            rotation_id: rotation_id.to_owned(),
            access_token: "access_012345678901234567890123456789".to_owned(),
            access_expires_at: (Utc::now() + Duration::minutes(30)).to_rfc3339(),
            refresh_token: self.next_refresh_token.clone(),
            refresh_expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        })
    }
}

fn initial_credentials() -> AgentCredentials {
    AgentCredentials {
        agent_id: "agent_01".to_owned(),
        refresh_token: "refresh_old_012345678901234567890123456789".to_owned(),
        pending_rotation: None,
    }
}

#[tokio::test]
async fn pending_rotation_survives_replay_and_commits_only_after_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let store = CredentialStore::new(directory.path().join("data/credentials.json"));
    store.store(&initial_credentials()).unwrap();
    let refresher = Arc::new(MockRefresher {
        calls: Mutex::new(Vec::new()),
        next_refresh_token: "refresh_new_012345678901234567890123456789".to_owned(),
    });
    let provider = CredentialAccessProvider::new(store.clone(), refresher.clone());

    let first = provider.prepare().await.unwrap();
    let persisted = store.load().unwrap();
    assert_eq!(persisted.refresh_token, initial_credentials().refresh_token);
    assert_eq!(
        persisted
            .pending_rotation
            .as_ref()
            .unwrap()
            .next_refresh_token
            .as_deref(),
        Some("refresh_new_012345678901234567890123456789")
    );

    let restarted = CredentialAccessProvider::new(store.clone(), refresher.clone());
    let replay = restarted.prepare().await.unwrap();
    assert_eq!(first.rotation_id, replay.rotation_id);
    assert_eq!(first.access_token, replay.access_token);
    {
        let calls = refresher.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "pending rotation 未确认前不得重复调用 refresh"
        );
    }

    restarted
        .commit(first.rotation_id.as_deref().unwrap())
        .await
        .unwrap();
    let committed = store.load().unwrap();
    assert_eq!(
        committed.refresh_token,
        "refresh_new_012345678901234567890123456789"
    );
    assert!(committed.pending_rotation.is_none());
}

#[tokio::test]
async fn legacy_pending_without_cached_access_refreshes_once_and_caches() {
    let directory = tempfile::tempdir().unwrap();
    let store = CredentialStore::new(directory.path().join("data/credentials.json"));
    let mut credentials = initial_credentials();
    credentials.pending_rotation = Some(PendingRotation {
        rotation_id: "rotation_00000001".to_owned(),
        next_refresh_token: Some("refresh_new_012345678901234567890123456789".to_owned()),
        access_token: None,
        access_expires_at: None,
    });
    store.store(&credentials).unwrap();
    let refresher = Arc::new(MockRefresher {
        calls: Mutex::new(Vec::new()),
        next_refresh_token: "refresh_new_012345678901234567890123456789".to_owned(),
    });
    let provider = CredentialAccessProvider::new(store.clone(), refresher.clone());

    let prepared = provider.prepare().await.unwrap();
    assert_eq!(prepared.rotation_id.as_deref(), Some("rotation_00000001"));
    let persisted = store.load().unwrap();
    assert!(persisted
        .pending_rotation
        .as_ref()
        .unwrap()
        .access_token
        .is_some());
    let calls = refresher.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn mismatched_replay_result_keeps_the_pending_state() {
    let directory = tempfile::tempdir().unwrap();
    let store = CredentialStore::new(directory.path().join("data/credentials.json"));
    let mut credentials = initial_credentials();
    credentials.pending_rotation = Some(PendingRotation {
        rotation_id: "rotation_00000001".to_owned(),
        next_refresh_token: Some("refresh_new_012345678901234567890123456789".to_owned()),
        access_token: None,
        access_expires_at: None,
    });
    store.store(&credentials).unwrap();
    let conflicting = CredentialAccessProvider::new(
        store.clone(),
        Arc::new(MockRefresher {
            calls: Mutex::new(Vec::new()),
            next_refresh_token: "refresh_other_0123456789012345678901234567".to_owned(),
        }),
    );
    assert!(matches!(
        conflicting.prepare().await,
        Err(TokenRefreshError::StateConflict)
    ));
    assert_eq!(
        store.load().unwrap().refresh_token,
        initial_credentials().refresh_token
    );
}
