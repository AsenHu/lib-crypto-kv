mod api;
mod cli;
mod config;
mod database;
mod lock;
mod logger;
mod metadata;
mod state;

use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    Router, middleware,
    routing::{delete, get, head, post, put},
};
use clap::Parser;
use log::info;
use tokio::net::TcpListener;

use crate::cli::Cli;
use crate::config::Config;
use crate::database::Db;
use crate::lock::Lock;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_file(&Cli::parse().config)?;
    logger::init(config.log.level.into());
    let state = AppState::new(
        Db::open(
            config.database.path,
            config.database.mode.into(),
            config.database.compression_factor,
            config.database.cache_capacity,
        )?,
        Lock::new(),
        config.garbage_collection.auto_delete.max_days.into(),
        config.garbage_collection.auto_delete.lazy,
    );
    let reclaim = {
        let state = state.clone();
        state.reclaim_worker(config.garbage_collection.lock.expire_minute)
    };
    info!("listening on: {}", config.listen.addr);
    let listener = TcpListener::bind(config.listen.addr).await?;
    let router = Router::new()
        .route("/api/v1/kvs/{key}", get(api::v1::kvs::get_value))
        .route("/api/v1/newKey", post(api::v1::kvs::new_key))
        .route("/api/v1/kvs/{key}", put(api::v1::kvs::modify_value))
        .route("/api/v1/kvs/{key}", delete(api::v1::kvs::remove_key))
        .route("/api/v1/kvs/{key}", head(api::v1::kvs::check_update))
        .route("/api/v1/spin/{key}", get(api::v1::spin::get_lock))
        .route("/api/v1/spin/{key}", delete(api::v1::spin::release_lock))
        .route("/api/v1/kvs/{key}/autoDel", put(api::v1::kvs::set_auto_del))
        .with_state(state)
        .layer(middleware::from_fn(logger::log_middleware));
    info!("server started");
    let signal_handler = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).unwrap();
        let mut int = signal(SignalKind::interrupt()).unwrap();
        tokio::select!(
            _ = term.recv() => {},
            _ = int.recv() => {}
        );
        info!("shutting down");
    };
    let axum_service = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    );
    tokio::select! {
        _ = reclaim => {}
        _ = signal_handler => {}
        r = axum_service => { r?; }
    }
    Ok(())
}
