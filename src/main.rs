mod api;
mod cli;
mod database;
mod lock;
mod logger;
mod metadata;
mod state;

use std::net::SocketAddr;

use axum::{
    Router, middleware,
    routing::{delete, get, head, post, put},
};
use clap::Parser;
use lock::Lock;
use log::{error, info};
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
    logger::init();
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
    let axum_service = axum::serve(listener, router);
    tokio::select! {
        _ = signal_handler => {}
        _ = axum_service => {}
    }
    Ok(())
}
