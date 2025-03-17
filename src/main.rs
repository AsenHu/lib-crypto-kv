mod api;
mod cli;
mod database;
mod lock;
mod metadata;
mod state;

use std::{io::Write, net::SocketAddr};

use axum::{
    Router,
    routing::{delete, get, head, post, put},
};
use chrono::Local;
use clap::Parser;
use lock::Lock;
use log::{Level, LevelFilter, error, info};
use state::AppState;
use tokio::net::TcpListener;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Sled(#[from] sled::Error),
    #[error(transparent)]
    AddrParse(#[from] std::net::AddrParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::new()
        .format(move |buf, record| {
            let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%SZ");
            let level_str = match record.level() {
                Level::Trace => "\x1B[1;35mTRACE\x1B[0m",
                Level::Debug => "\x1B[1;30mDEBUG\x1B[0m",
                Level::Info => "\x1B[1;36mINFO\x1B[0m",
                Level::Warn => "\x1B[1;93mWARN\x1B[0m",
                Level::Error => "\x1B[1;31mERROR\x1B[0m",
            };
            writeln!(buf, "[{} {}]: {}", timestamp, level_str, record.args())
        })
        .filter_level(match std::env::var("RUST_LOG") {
            Ok(val) => match val.as_str() {
                "trace" => LevelFilter::Trace,
                "debug" => LevelFilter::Debug,
                "info" => LevelFilter::Info,
                "warn" => LevelFilter::Warn,
                "error" => LevelFilter::Error,
                _ => LevelFilter::Info,
            },
            Err(_) => LevelFilter::Info,
        })
        .init();
    let args = cli::Cli::parse();
    let state = AppState::new(database::open(args.db.as_ref())?, Lock::new());
    tokio::spawn({
        let state = state.clone();
        state.reclaim_worker()
    });
    info!("listening on: {}", args.addr);
    let listener = TcpListener::bind(args.addr.parse::<SocketAddr>()?).await?;
    let router = Router::new()
        .route("/api/v1/kvs/{key}", get(api::v1::kvs::get_value))
        .route("/api/v1/newKey", post(api::v1::kvs::new_key))
        .route("/api/v1/kvs/{key}", put(api::v1::kvs::modify_value))
        .route("/api/v1/kvs/{key}", delete(api::v1::kvs::remove_key))
        .route("/api/v1/kvs/{key}", head(api::v1::kvs::check_update))
        .route("/api/v1/spin/{key}", get(api::v1::spin::get_lock))
        .route("/api/v1/spin/{key}", delete(api::v1::spin::release_lock))
        .with_state(state);
    info!("server started");
    axum::serve(listener, router).await?;
    Ok(())
}
