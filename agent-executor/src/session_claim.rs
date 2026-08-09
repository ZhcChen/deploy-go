use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct SessionRegistry {
    state: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    active: Option<String>,
    cleanup_failed: bool,
}

impl SessionRegistry {
    pub fn claim(self: &Arc<Self>, session_id: &str) -> Option<SessionClaim> {
        let mut state = self.state.lock().ok()?;
        if state.cleanup_failed || state.active.is_some() {
            return None;
        }
        state.active = Some(session_id.to_owned());
        Some(SessionClaim {
            registry: Arc::clone(self),
            session_id: session_id.to_owned(),
        })
    }

    pub fn active(&self) -> Option<String> {
        self.state.lock().ok()?.active.clone()
    }

    pub fn block_after_cleanup_failure(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.cleanup_failed = true;
        }
    }

    pub fn cleanup_failed(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.cleanup_failed)
            .unwrap_or(true)
    }
}

pub struct SessionClaim {
    registry: Arc<SessionRegistry>,
    session_id: String,
}

impl Drop for SessionClaim {
    fn drop(&mut self) {
        if let Ok(mut state) = self.registry.state.lock()
            && state.active.as_deref() == Some(&self.session_id)
        {
            state.active = None;
        }
    }
}
