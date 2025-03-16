use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use log::info;
use tokio::sync::Mutex;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Locked {
    id: Box<str>,
    time: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Lock {
    lock: Arc<Mutex<HashMap<Box<str>, Locked>>>,
}

impl Lock {
    pub fn new() -> Self {
        let lock: Arc<Mutex<HashMap<Box<str>, Locked>>> = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn({
            let lock = lock.clone();
            async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    let mut lock = lock.lock().await;
                    lock.retain(|_, l| {
                        if Utc::now().signed_duration_since(l.time).num_minutes() < 5 {
                            true
                        } else {
                            info!("releasing lock {:?}", l.id);
                            false
                        }
                    });
                }
            }
        });
        Self { lock }
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

    async fn release_inner(&self, key: Box<str>) -> bool {
        match self.lock.lock().await.remove(&key) {
            Some(_) => true,
            None => false,
        }
    }

    pub async fn release(&self, key: impl Into<Box<str>>, id: impl Into<Box<str>>) -> bool {
        let key = key.into();
        let lock = self.lock.lock().await;
        match lock.get(&key).cloned() {
            Some(l) => {
                drop(lock); // prevent deadlock
                if l.id == id.into() {
                    self.release_inner(key).await
                } else {
                    false
                }
            }
            None => false,
        }
    }
}
