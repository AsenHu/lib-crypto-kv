use std::sync::Arc;

use chrono::{Datelike, TimeZone as _, Utc};
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
        let lock = async {
            loop {
                self.lock
                    .release_outdated()
                    .await
                    .iter()
                    .for_each(|(key, id)| {
                        debug!("released lock: {} for {}", id, key);
                    });
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        };
        let reclaim = async {
            loop {
                let now = Utc::now();
                let next_midnight = Utc
                    .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 1)
                    .unwrap()
                    .checked_add_days(chrono::Days::new(1))
                    .unwrap();
                let sleep_duration = next_midnight.signed_duration_since(now);
                tokio::time::sleep(sleep_duration.to_std().unwrap()).await;
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
            }
        };
        tokio::join!(lock, reclaim);
        unreachable!()
    }
}
