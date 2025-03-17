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

impl Lock {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(HashMap::new()),
        }
    }

    pub async fn claim(&self, key: impl Into<Box<str>>, id: impl Into<Box<str>>) -> bool {
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
                    return true;
                }
                false
            }
            None => {
                lock.insert(key, locked);
                true
            }
        }
    }

    pub async fn release(&self, key: impl Into<Box<str>>, id: impl Into<Box<str>>) -> bool {
        let key = key.into();
        let lock = self.lock.lock().await;
        match lock.get(&key).cloned() {
            Some(l) => {
                drop(lock);
                if l.id == id.into() {
                    match self.lock.lock().await.remove(&key) {
                        Some(_) => true,
                        None => false,
                    }
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub async fn release_outdated(&self) -> HashMap<Box<str>, Box<str>> {
        let mut lock = self.lock.lock().await;
        let mut keys = HashMap::new();
        lock.retain(|k, l| {
            if Utc::now().signed_duration_since(l.time).num_minutes() < 5 {
                true
            } else {
                keys.insert(k.clone(), l.id.clone());
                false
            }
        });
        keys
    }
}
