use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use log::error;
use rand::Rng;

use crate::{AppState, metadata::Metadata};

pub async fn new_key(
    State(state): State<AppState>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut header = header::HeaderMap::new();
    let db = &state.db;

    let mut key = String::new();
    let mut rng = rand::rng();
    let mut length = 1u8;
    for _ in 0..length {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let index = rng.random_range(0..CHARSET.len());
        key.push(CHARSET[index] as char);
        match db.get(key.as_bytes()) {
            Ok(None) => {}
            Ok(Some(_)) => {
                length += 1;
            }
            Err(e) => {
                error!("{}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
            }
        }
    }

    let value = &rng.random::<u16>().to_be_bytes();

    let metadata = Metadata::new(key.as_str(), value, &token.token());

    header.insert(
        header::LAST_MODIFIED,
        metadata.last_modified().to_rfc2822().parse().unwrap(),
    );
    header.insert(header::ETAG, hex::encode(metadata.dgst()).parse().unwrap());

    match db.insert(key.as_bytes(), value, metadata) {
        Ok(_) => {
            let status = StatusCode::CREATED;
            (status, header, key).into_response()
        }
        Err(e) => {
            error!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}

pub async fn modify_value(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    value: Bytes,
) -> impl IntoResponse {
    let mut header = header::HeaderMap::new();

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            let mut metadata = v.metadata;
            header.insert(
                header::LAST_MODIFIED,
                metadata.last_modified().to_rfc2822().parse().unwrap(),
            );
            header.insert(header::ETAG, hex::encode(metadata.dgst()).parse().unwrap());
            metadata.modifiy(&value);
            match db.insert(key.as_bytes(), &value, metadata) {
                Ok(_) => {
                    let status = StatusCode::NO_CONTENT;
                    (status, header).into_response()
                }
                Err(e) => {
                    error!("{}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            }
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status, header).into_response()
        }
        Err(e) => {
            error!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}

pub async fn remove_key(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut header = header::HeaderMap::new();

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            header.insert(
                header::LAST_MODIFIED,
                v.metadata.last_modified().to_rfc2822().parse().unwrap(),
            );
            header.insert(
                header::ETAG,
                hex::encode(v.metadata.dgst()).parse().unwrap(),
            );
            match db.remove(key.as_bytes()) {
                Ok(_) => {
                    let status = StatusCode::NO_CONTENT;
                    (status, header).into_response()
                }
                Err(e) => {
                    error!("{}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            }
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status, header).into_response()
        }
        Err(e) => {
            error!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}

pub async fn get_value(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut header = header::HeaderMap::new();

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            header.insert(
                header::CONTENT_TYPE,
                "application/octet-stream".parse().unwrap(),
            );
            header.insert(
                header::LAST_MODIFIED,
                v.metadata.last_modified().to_rfc2822().parse().unwrap(),
            );
            header.insert(
                header::ETAG,
                hex::encode(v.metadata.dgst()).parse().unwrap(),
            );
            let status = StatusCode::OK;
            (status, header, v.data.to_vec()).into_response()
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status, header).into_response()
        }
        Err(e) => {
            error!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}

pub async fn check_update(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut header = header::HeaderMap::new();

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            header.insert(
                header::LAST_MODIFIED,
                v.metadata.last_modified().to_rfc2822().parse().unwrap(),
            );
            header.insert(
                header::ETAG,
                hex::encode(v.metadata.dgst()).parse().unwrap(),
            );
            let status = StatusCode::NO_CONTENT;
            (status, header).into_response()
        }
        Ok(None) => {
            let status = StatusCode::NOT_FOUND;
            (status, header).into_response()
        }
        Err(e) => {
            error!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
