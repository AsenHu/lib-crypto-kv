use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use log::error;
use serde::Deserialize;

use crate::AppState;

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct Q {
    pub lockID: Box<str>,
}

pub async fn get_lock(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Query(q): Query<Q>,
) -> impl IntoResponse {
    let db = &state.db;
    let lock = &state.lock;

    let lock_id = q.lockID;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                let status = StatusCode::FORBIDDEN;
                return (status).into_response();
            }
            if lock.claim(key.as_ref(), lock_id).await {
                let status = StatusCode::OK;
                (status).into_response()
            } else {
                let status = StatusCode::CONFLICT;
                (status).into_response()
            }
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status).into_response()
        }
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            error!("{}", e);
            (status).into_response()
        }
    }
}

pub async fn release_lock(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Query(q): Query<Q>,
) -> impl IntoResponse {
    let db = &state.db;
    let lock = &state.lock;

    let lock_id = q.lockID;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                let status = StatusCode::FORBIDDEN;
                return (status).into_response();
            }
            if lock.release(key.as_ref(), lock_id).await {
                let status = StatusCode::OK;
                (status).into_response()
            } else {
                let status = StatusCode::NOT_FOUND;
                (status).into_response()
            }
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status).into_response()
        }
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            error!("{}", e);
            (status).into_response()
        }
    }
}
