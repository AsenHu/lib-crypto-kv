pub mod kvs;
pub mod spin;

use std::sync::Arc;

use log::{debug, error};

use crate::db::Db;

fn lazy_reclaim(db: Arc<Db>) {
    debug!("lazy_reclaim: start");
    match db.reclaim_outdated() {
        Ok(outdated) => {
            outdated.iter().for_each(|key| {
                debug!("lazy_reclaim: reclaimed key: {}", key);
            });
        }
        Err(e) => {
            error!("lazy_reclaim: reclaim_outdated error: {}", e);
        }
    }
    debug!("lazy_reclaim: done");
}
