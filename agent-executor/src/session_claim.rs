use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct SessionRegistry {
    active: Mutex<Option<String>>,
}

impl SessionRegistry {
    pub fn claim(self: &Arc<Self>, session_id: &str) -> Option<SessionClaim> {
        let mut active = self.active.lock().ok()?;
        if active.is_some() {
            return None;
        }
        *active = Some(session_id.to_owned());
        Some(SessionClaim {
            registry: Arc::clone(self),
            session_id: session_id.to_owned(),
        })
    }

    pub fn active(&self) -> Option<String> {
        self.active.lock().ok()?.clone()
    }
}

pub struct SessionClaim {
    registry: Arc<SessionRegistry>,
    session_id: String,
}

impl Drop for SessionClaim {
    fn drop(&mut self) {
        if let Ok(mut active) = self.registry.active.lock()
            && active.as_deref() == Some(&self.session_id)
        {
            *active = None;
        }
    }
}
