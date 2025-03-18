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

use crate::{
    AppState,
    database::Db,
    lock::{Lock, LockError},
};

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct Q {
    pub lockID: Box<str>,
}

async fn lock_handler(
    db: &Db,
    lock: &Lock,
    key: &str,
    token: &str,
    lock_id: &str,
    release: bool,
) -> impl IntoResponse + use<> {
    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            match if release {
                lock.release(key, lock_id).await
            } else {
                lock.claim(key, lock_id).await
            } {
                Ok(_) => StatusCode::NO_CONTENT,
                Err(e) => match e {
                    LockError::NotFound => StatusCode::NOT_FOUND,
                    LockError::Conflict => StatusCode::CONFLICT,
                },
            }
        }
        Ok(None) => StatusCode::NOT_FOUND,
        Err(e) => {
            error!("{}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
    .into_response()
}

pub async fn get_lock(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
    Query(q): Query<Q>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };
    lock_handler(
        &state.db,
        &state.lock,
        &key,
        token.token(),
        &q.lockID,
        false,
    )
    .await
    .into_response()
}

pub async fn release_lock(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
    Query(q): Query<Q>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };
    lock_handler(&state.db, &state.lock, &key, token.token(), &q.lockID, true)
        .await
        .into_response()
}
