use std::sync::Arc;

use chrono::{Datelike, TimeZone as _, Utc};
use log::{debug, error};

use crate::{db::Db, lock::Lock};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub lock: Arc<Lock>,
    pub auto_del_max_days: u64,
    pub lazy: bool,
}

impl AppState {
    pub fn new(db: Db, lock: Lock, auto_del_max_days: u64, lazy: bool) -> Self {
        Self {
            db: Arc::new(db),
            lock: Arc::new(lock),
            auto_del_max_days,
            lazy,
        }
    }

    pub async fn reclaim_worker(self, lock_ttl: i64) -> ! {
        let lock = async {
            loop {
                self.lock
                    .release_outdated(lock_ttl)
                    .await
                    .iter()
                    .for_each(|(key, id)| {
                        debug!("released lock: {} for {}", id, key);
                    });
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        };
        let reclaim = async {
            if self.lazy {
                std::future::pending::<()>().await;
            }
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
                            debug!("reclaimed key: {}", key);
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
