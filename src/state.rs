use std::sync::Arc;

use log::{debug, error};

use crate::{database, lock::Lock};
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<database::Db>,
    pub lock: Arc<Lock>,
}

impl AppState {
    pub fn new(db: database::Db, lock: Lock) -> Self {
        Self {
            db: Arc::new(db),
            lock: Arc::new(lock),
        }
    }

    pub async fn reclaim_worker(self) -> ! {
        loop {
            self.lock
                .release_outdated()
                .await
                .iter()
                .for_each(|(key, id)| {
                    debug!("released lock: {} for {}", id, key);
                });
            match self.db.reclaim_outdated() {
                Ok(outdated) => {
                    outdated.iter().for_each(|key| {
                        debug!("reclaimed key: {}", String::from_utf8_lossy(key));
                    });
                }
                Err(e) => {
                    error!("reclaim_outdated error: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
