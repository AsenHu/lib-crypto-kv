use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Locked {
    id: Box<str>,
    time: DateTime<Utc>,
}

pub struct Lock {
    lock: Mutex<HashMap<Box<str>, Locked>>,
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("key not found")]
    NotFound,
    #[error("key already locked")]
    Conflict,
}

impl Lock {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(HashMap::new()),
        }
    }

    pub async fn claim(
        &self,
        key: impl Into<Box<str>>,
        id: impl Into<Box<str>>,
    ) -> Result<(), LockError> {
        let id = id.into();
        let key = key.into();
        let locked = Locked {
            id: id.clone(),
            time: Utc::now(),
        };
        let mut lock = self.lock.lock().await;
        match lock.get(&key) {
            Some(l) => {
                if l.id == id {
                    lock.insert(key, locked);
                    return Ok(());
                }
                Err(LockError::Conflict)
            }
            None => {
                lock.insert(key, locked);
                Ok(())
            }
        }
    }

    pub async fn release(
        &self,
        key: impl Into<Box<str>>,
        id: impl Into<Box<str>>,
    ) -> Result<(), LockError> {
        let key = key.into();
        let lock = self.lock.lock().await;
        match lock.get(&key).cloned() {
            Some(l) => {
                drop(lock);
                if l.id == id.into() {
                    match self.lock.lock().await.remove(&key) {
                        Some(_) => Ok(()),
                        None => Err(LockError::NotFound),
                    }
                } else {
                    Err(LockError::Conflict)
                }
            }
            None => Err(LockError::NotFound),
        }
    }

    pub async fn release_outdated(&self, lock_ttl: i64) -> HashMap<Box<str>, Box<str>> {
        let mut lock = self.lock.lock().await;
        let mut keys = HashMap::new();
        lock.retain(|k, l| {
            if Utc::now().signed_duration_since(l.time).num_minutes() < lock_ttl {
                true
            } else {
                keys.insert(k.clone(), l.id.clone());
                false
            }
        });
        keys
    }
}
