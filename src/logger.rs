use std::{io::Write, net::SocketAddr};

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use chrono::Local;
use log::{Level, LevelFilter, info, warn};

pub fn init(level: LevelFilter) {
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
        .filter_level(level)
        .init();
}

pub async fn log_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let response = next.run(req).await;
    let status = response.status();
    info!("{} {}", addr, uri);
    if status.is_server_error() {
        warn!("{} {} {}", method, uri, response.status());
    } else {
        info!("{} {} {}", method, uri, response.status());
    }
    response
}
