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

fn metadata_header(header: &mut header::HeaderMap, metadata: &Metadata) {
    header.insert(
        header::LAST_MODIFIED,
        metadata.last_modified().to_rfc2822().parse().unwrap(),
    );
    header.insert(
        header::ETAG,
        String::from_utf8_lossy(&base91::slice_encode(metadata.dgst()))
            .parse()
            .unwrap(),
    );
}

pub async fn new_key(
    State(state): State<AppState>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };
    let db = &state.db;

    let mut key: Box<str>;
    let mut rng = rand::rng();
    let mut length = 1u8;
    loop {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        key = (0..length)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        match db.get(key.as_bytes()) {
            Ok(None) => break,
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

    let mut header = header::HeaderMap::new();

    let metadata = Metadata::new(&key, value, token.token());

    metadata_header(&mut header, &metadata);

    match db.insert(key.as_bytes(), value, metadata) {
        Ok(_) => (StatusCode::CREATED, header, key).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn modify_value(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
    value: Bytes,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            let mut header = header::HeaderMap::new();
            let mut metadata = v.metadata;
            metadata.modifiy(&value);
            metadata_header(&mut header, &metadata);
            match db.insert(key.as_bytes(), &value, metadata) {
                Ok(_) => (StatusCode::NO_CONTENT, header).into_response(),
                Err(e) => {
                    error!("{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn remove_key(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            match db.remove(key.as_bytes()) {
                Ok(_) => (StatusCode::NO_CONTENT).into_response(),
                Err(e) => {
                    error!("{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn get_value(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            let mut header = header::HeaderMap::new();
            header.insert(
                header::CONTENT_TYPE,
                "application/octet-stream".parse().unwrap(),
            );
            metadata_header(&mut header, &v.metadata);
            (StatusCode::OK, header, v.data.to_vec()).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn check_update(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let db = &state.db;

    let token = token.token();

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            let mut header = header::HeaderMap::new();
            metadata_header(&mut header, &v.metadata);
            let status = StatusCode::NO_CONTENT;
            (status, header).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn set_auto_del(
    State(state): State<AppState>,
    Path(key): Path<Box<str>>,
    token: Option<TypedHeader<Authorization<Bearer>>>,
    body: Bytes,
) -> impl IntoResponse {
    let token = match token {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let db = &state.db;

    let token = token.token();

    let days = match std::str::from_utf8(&body) {
        Ok(s) => match s.parse::<u16>() {
            Ok(n) => n,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid number".to_string()).into_response();
            }
        },
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid UTF-8".to_string()).into_response(),
    };

    match db.get(key.as_bytes()) {
        Ok(Some(v)) => {
            const MAX_DAYS: u16 = 365;
            if !v.metadata.is_allowed(&key, token) {
                return (StatusCode::FORBIDDEN).into_response();
            }
            if days > MAX_DAYS {
                return (StatusCode::BAD_REQUEST, MAX_DAYS.to_string()).into_response();
            }
            let mut metadata = v.metadata;
            metadata.set_auto_del(days);
            match db.insert(key.as_bytes(), &v.data, metadata) {
                Ok(_) => (StatusCode::NO_CONTENT).into_response(),
                Err(e) => {
                    error!("{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            error!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
